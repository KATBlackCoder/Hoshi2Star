import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { t } from "i18next";
import type {
  PipelineBatchMetrics,
  ProviderCallMetrics,
  ProviderConfig,
} from "@/lib/types";
import {
  DEFAULT_BATCH_SIZE,
  DEFAULT_OLLAMA_MODEL,
  DEFAULT_OLLAMA_URL,
} from "@/lib/constants";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

interface ProgressPayload {
  done: number;
  total: number;
}

interface CoolingPayload {
  remainingSecs: number;
}

/** `h2s://llm/completed` — one summary per run (Rust `RunSummary`). */
export interface RunSummary {
  count: number;
  translated: number;
  needsReview: number;
  /** No usable output: the segment was left unchanged. */
  failed: number;
  qaErrorsByType: Record<string, number>;
  cancelled: boolean;
}

/** `h2s://llm/error` — one notice per run (Rust `LlmErrorPayload`). */
export interface LlmErrorPayload {
  message: string;
  providerUnavailable: boolean;
  processed: number;
  total: number;
}

interface LlmState {
  isTranslating: boolean;
  /** 0–100, -1 when idle */
  translationProgress: number;
  providerConfig: ProviderConfig;
  error: string | null;
  /** timestamp Date.now() when translation started, null when idle */
  translationStartTime: number | null;
  isCooling: boolean;
  cooldownRemaining: number;
  requestMetrics: ProviderCallMetrics[];
  pipelineMetrics: PipelineBatchMetrics[];
  /** "Stop" was pressed; the run ends after its current batch. */
  isCancelling: boolean;

  // Actions
  setProviderConfig: (cfg: Partial<ProviderConfig>) => void;
  startTranslation: (segmentIds: string[], fileId?: string) => Promise<void>;
  startTranslateAll: (
    projectId: string,
    thresholdMins: number,
    cooldownMins: number,
  ) => Promise<void>;
  cancelTranslation: () => Promise<void>;
  reset: () => void;
  startTimer: () => void;
  stopTimer: () => void;
  appendMetrics: (metrics: ProviderCallMetrics[]) => void;
  appendPipelineMetrics: (metrics: PipelineBatchMetrics) => void;
}

// ---------------------------------------------------------------------------
// Default config — Ollama through its OpenAI-compatible endpoint
// ---------------------------------------------------------------------------

const DEFAULT_CONFIG: ProviderConfig = {
  providerId: "ollama",
  url: DEFAULT_OLLAMA_URL,
  model: DEFAULT_OLLAMA_MODEL,
  batchSize: DEFAULT_BATCH_SIZE,
  resourceProfile: "balanced",
};

// ---------------------------------------------------------------------------
// Listener helpers
// ---------------------------------------------------------------------------

// Single teardown for any active translation session.
let activeTeardown: (() => void) | null = null;

interface TranslationListenerOpts {
  onProgress: (done: number, total: number) => void;
  onCompleted: (summary: RunSummary) => void;
  onError: (payload: LlmErrorPayload) => void;
  onCooling?: (remainingSecs: number) => void;
  onMetrics: (metrics: ProviderCallMetrics[]) => void;
  onPipelineMetrics: (metrics: PipelineBatchMetrics) => void;
}

/**
 * Register all translation event listeners and return a single teardown fn.
 * Teardown is called automatically on completed/error events, and also from
 * each startTranslation* call to clear any previous session.
 */
async function setupTranslationListeners(
  opts: TranslationListenerOpts,
): Promise<() => void> {
  const fns: UnlistenFn[] = [];

  function teardown() {
    fns.forEach((fn) => fn());
    fns.length = 0;
  }

  fns.push(
    await listen<ProgressPayload>("h2s://llm/progress", (event) => {
      opts.onProgress(event.payload.done, event.payload.total);
    }),
  );

  fns.push(
    await listen<PipelineBatchMetrics>(
      "h2s://llm/pipeline-metrics",
      (event) => opts.onPipelineMetrics(event.payload),
    ),
  );

  fns.push(
    await listen<RunSummary>("h2s://llm/completed", (event) => {
      opts.onCompleted(event.payload);
      teardown();
    }),
  );

  fns.push(
    await listen<LlmErrorPayload>("h2s://llm/error", (event) => {
      opts.onError(event.payload);
      teardown();
    }),
  );

  fns.push(
    await listen<ProviderCallMetrics[]>("h2s://llm/metrics", (event) => {
      opts.onMetrics(event.payload);
    }),
  );

  if (opts.onCooling) {
    const onCooling = opts.onCooling;
    fns.push(
      await listen<CoolingPayload>("h2s://llm/cooling", (event) => {
        onCooling(event.payload.remainingSecs);
      }),
    );
  }

  return teardown;
}

// ---------------------------------------------------------------------------
// Run notices — exactly one toast per run, never one per segment
// ---------------------------------------------------------------------------

/** Same id for every run notice: a new one replaces the previous one. */
export const RUN_TOAST_ID = "llm-run";

function qaTypesLine(byType: Record<string, number>): string {
  return Object.entries(byType)
    .map(
      ([type, count]) =>
        `${t(`llmRun.qaTypes.${type}`, { defaultValue: type })} ×${count}`,
    )
    .join(", ");
}

export function notifyRunCompleted(summary: RunSummary): void {
  const counts = {
    translated: summary.translated,
    needsReview: summary.needsReview,
    failed: summary.failed,
  };
  const types = qaTypesLine(summary.qaErrorsByType);
  const description = types
    ? t("llmRun.reviewTypes", { types })
    : undefined;
  if (summary.cancelled) {
    toast.info(t("llmRun.cancelled", counts), {
      id: RUN_TOAST_ID,
      description,
      duration: 8000,
    });
  } else if (summary.needsReview === 0 && summary.failed === 0) {
    toast.success(t("llmRun.completedOk", counts), {
      id: RUN_TOAST_ID,
      duration: 5000,
    });
  } else {
    toast.warning(t("llmRun.completedIssues", counts), {
      id: RUN_TOAST_ID,
      description,
      duration: 8000,
    });
  }
}

export function notifyRunError(payload: LlmErrorPayload): void {
  const title = payload.providerUnavailable
    ? t("llmRun.providerDown", {
        processed: payload.processed,
        total: payload.total,
      })
    : t("llmRun.error");
  toast.error(title, {
    id: RUN_TOAST_ID,
    description: payload.message,
    duration: 10000,
  });
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

export const useLlmStore = create<LlmState>()((set, get) => ({
  isTranslating: false,
  translationProgress: -1,
  providerConfig: { ...DEFAULT_CONFIG },
  error: null,
  translationStartTime: null,
  isCooling: false,
  cooldownRemaining: 0,
  requestMetrics: [],
  pipelineMetrics: [],
  isCancelling: false,

  setProviderConfig: (cfg) =>
    set((s) => ({ providerConfig: { ...s.providerConfig, ...cfg } })),

  startTimer: () => set({ translationStartTime: Date.now() }),

  stopTimer: () => set({ translationStartTime: null }),

  appendMetrics: (metrics) =>
    set((state) => ({ requestMetrics: [...state.requestMetrics, ...metrics] })),
  appendPipelineMetrics: (metrics) =>
    set((state) => ({
      pipelineMetrics: [...state.pipelineMetrics, metrics],
    })),

  startTranslation: async (segmentIds, fileId) => {
    if (get().isTranslating) return;

    set({
      isTranslating: true,
      isCancelling: false,
      translationProgress: 0,
      error: null,
      requestMetrics: [],
      pipelineMetrics: [],
    });
    get().startTimer();

    activeTeardown?.();

    activeTeardown = await setupTranslationListeners({
      onProgress: (done, total) => {
        const pct = total > 0 ? Math.round((done / total) * 100) : 0;
        set({ translationProgress: pct });
      },
      onCompleted: (summary) => {
        get().stopTimer();
        set({
          isTranslating: false,
          isCancelling: false,
          translationProgress: 100,
          isCooling: false,
          cooldownRemaining: 0,
        });
        notifyRunCompleted(summary);
        activeTeardown = null;
      },
      onError: (payload) => {
        get().stopTimer();
        set({
          isTranslating: false,
          isCancelling: false,
          translationProgress: -1,
          isCooling: false,
          cooldownRemaining: 0,
          error: payload.message,
        });
        notifyRunError(payload);
        activeTeardown = null;
      },
      onMetrics: (metrics) => get().appendMetrics(metrics),
      onPipelineMetrics: (metrics) => get().appendPipelineMetrics(metrics),
    });

    try {
      await invoke("translate_segments", {
        ids: segmentIds,
        fileId: fileId ?? null,
        providerConfig: get().providerConfig,
      });
    } catch (err) {
      set({
        isTranslating: false,
        translationProgress: -1,
        error: err instanceof Error ? err.message : String(err),
      });
      activeTeardown?.();
      activeTeardown = null;
    }
  },

  startTranslateAll: async (projectId, thresholdMins, cooldownMins) => {
    if (get().isTranslating) return;

    set({
      isTranslating: true,
      isCancelling: false,
      translationProgress: 0,
      error: null,
      isCooling: false,
      cooldownRemaining: 0,
      requestMetrics: [],
      pipelineMetrics: [],
    });
    get().startTimer();

    activeTeardown?.();

    activeTeardown = await setupTranslationListeners({
      onProgress: (done, total) => {
        const pct = total > 0 ? Math.round((done / total) * 100) : 0;
        set({ translationProgress: pct });
      },
      onCompleted: (summary) => {
        get().stopTimer();
        set({
          isTranslating: false,
          isCancelling: false,
          translationProgress: 100,
          isCooling: false,
          cooldownRemaining: 0,
        });
        notifyRunCompleted(summary);
        activeTeardown = null;
      },
      onError: (payload) => {
        get().stopTimer();
        set({
          isTranslating: false,
          isCancelling: false,
          translationProgress: -1,
          isCooling: false,
          cooldownRemaining: 0,
          error: payload.message,
        });
        notifyRunError(payload);
        activeTeardown = null;
      },
      onMetrics: (metrics) => get().appendMetrics(metrics),
      onPipelineMetrics: (metrics) => get().appendPipelineMetrics(metrics),
      onCooling: (remainingSecs) => {
        set({ isCooling: remainingSecs > 0, cooldownRemaining: remainingSecs });
      },
    });

    try {
      await invoke("translate_all_segments", {
        projectId,
        providerConfig: get().providerConfig,
        cooldownThresholdSecs: thresholdMins * 60,
        cooldownDurationSecs: cooldownMins * 60,
      });
    } catch (err) {
      set({
        isTranslating: false,
        translationProgress: -1,
        isCooling: false,
        cooldownRemaining: 0,
        error: err instanceof Error ? err.message : String(err),
      });
      activeTeardown?.();
      activeTeardown = null;
    }
  },

  cancelTranslation: async () => {
    if (!get().isTranslating || get().isCancelling) return;
    set({ isCancelling: true });
    try {
      await invoke("cancel_translation");
    } catch (err) {
      set({ isCancelling: false });
      toast.error(err instanceof Error ? err.message : String(err), {
        id: RUN_TOAST_ID,
      });
    }
  },

  reset: () =>
    set({
      isTranslating: false,
      isCancelling: false,
      translationProgress: -1,
      error: null,
      translationStartTime: null,
      isCooling: false,
      cooldownRemaining: 0,
      requestMetrics: [],
      pipelineMetrics: [],
    }),
}));

// Selectors
export const useIsTranslating = () => useLlmStore((s) => s.isTranslating);
export const useTranslationProgress = () =>
  useLlmStore((s) => s.translationProgress);
export const useProviderConfig = () => useLlmStore((s) => s.providerConfig);
export const useTranslationStartTime = () =>
  useLlmStore((s) => s.translationStartTime);
export const useIsCooling = () => useLlmStore((s) => s.isCooling);
export const useIsCancelling = () => useLlmStore((s) => s.isCancelling);
export const useCooldownRemaining = () =>
  useLlmStore((s) => s.cooldownRemaining);
export const useRequestMetrics = () => useLlmStore((s) => s.requestMetrics);
export const usePipelineMetrics = () => useLlmStore((s) => s.pipelineMetrics);
