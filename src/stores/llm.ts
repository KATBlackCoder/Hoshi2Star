import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { t } from "i18next";
import type { ProviderConfig } from "@/lib/types";
import {
  DEFAULT_BATCH_SIZE,
  DEFAULT_OLLAMA_MODEL,
  DEFAULT_OLLAMA_URL,
} from "@/lib/constants";
import { errorMessage } from "@/lib/utils";
import { EVENTS } from "@/lib/events";

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
  /** Other rows of the same name (earlier fallbacks) given this run's translation. */
  repaired: number;
}

/**
 * Whether a model is configured; if not, says so in a `level` notice.
 * Every "translate" action checks it before starting a run.
 */
export function ensureModelConfigured(
  level: "error" | "warning" = "error",
): boolean {
  if (useLlmStore.getState().providerConfig.model.trim()) return true;
  toast[level](t("segmentGrid.noModelConfigured"));
  return false;
}

/**
 * Whether a run persisted anything: translated or needs-review rows, or
 * earlier fallbacks repaired. Failed segments are never written.
 */
export function runWroteSegments(summary: RunSummary): boolean {
  return summary.translated + summary.needsReview + summary.repaired > 0;
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
    await listen<ProgressPayload>(EVENTS.llmProgress, (event) => {
      opts.onProgress(event.payload.done, event.payload.total);
    }),
  );

  fns.push(
    await listen<RunSummary>(EVENTS.llmCompleted, (event) => {
      opts.onCompleted(event.payload);
      teardown();
    }),
  );

  fns.push(
    await listen<LlmErrorPayload>(EVENTS.llmError, (event) => {
      opts.onError(event.payload);
      teardown();
    }),
  );

  if (opts.onCooling) {
    const onCooling = opts.onCooling;
    fns.push(
      await listen<CoolingPayload>(EVENTS.llmCooling, (event) => {
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
  const lines = [
    types ? t("llmRun.reviewTypes", { types }) : "",
    summary.repaired > 0
      ? t("llmRun.repaired", { count: summary.repaired })
      : "",
  ].filter(Boolean);
  const description = lines.length > 0 ? lines.join("\n") : undefined;
  if (summary.cancelled) {
    toast.info(t("llmRun.cancelled", counts), {
      id: RUN_TOAST_ID,
      description,
      duration: 8000,
    });
  } else if (summary.needsReview === 0 && summary.failed === 0) {
    toast.success(t("llmRun.completedOk", counts), {
      id: RUN_TOAST_ID,
      description,
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

export const useLlmStore = create<LlmState>()((set, get) => {
  /** Run state once a run ends, whatever the way (event or failed launch). */
  const idle = {
    isTranslating: false,
    isCancelling: false,
    isCooling: false,
    cooldownRemaining: 0,
  };

  /**
   * Shared by both entry points: one run at a time, one set of listeners
   * (torn down on completed/error), one notice per run. `withCooling` listens
   * to the automatic cooldown of "Translate all".
   */
  async function runTranslation(
    command: string,
    args: Record<string, unknown>,
    withCooling: boolean,
  ): Promise<void> {
    if (get().isTranslating) return;

    set({
      ...idle,
      isTranslating: true,
      translationProgress: 0,
      error: null,
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
        set({ ...idle, translationProgress: 100 });
        notifyRunCompleted(summary);
        activeTeardown = null;
      },
      onError: (payload) => {
        get().stopTimer();
        set({ ...idle, translationProgress: -1, error: payload.message });
        notifyRunError(payload);
        activeTeardown = null;
      },
      onCooling: withCooling
        ? (remainingSecs) =>
            set({
              isCooling: remainingSecs > 0,
              cooldownRemaining: remainingSecs,
            })
        : undefined,
    });

    try {
      await invoke(command, { ...args, providerConfig: get().providerConfig });
    } catch (err) {
      set({ ...idle, translationProgress: -1, error: errorMessage(err) });
      activeTeardown?.();
      activeTeardown = null;
    }
  }

  return {
    isTranslating: false,
    translationProgress: -1,
    providerConfig: { ...DEFAULT_CONFIG },
    error: null,
    translationStartTime: null,
    isCooling: false,
    cooldownRemaining: 0,
    isCancelling: false,

    setProviderConfig: (cfg) =>
      set((s) => ({ providerConfig: { ...s.providerConfig, ...cfg } })),

    startTimer: () => set({ translationStartTime: Date.now() }),

    stopTimer: () => set({ translationStartTime: null }),

    startTranslation: (segmentIds, fileId) =>
      runTranslation(
        "translate_segments",
        { ids: segmentIds, fileId: fileId ?? null },
        false,
      ),

    startTranslateAll: (projectId, thresholdMins, cooldownMins) =>
      runTranslation(
        "translate_all_segments",
        {
          projectId,
          cooldownThresholdSecs: thresholdMins * 60,
          cooldownDurationSecs: cooldownMins * 60,
        },
        true,
      ),

    cancelTranslation: async () => {
      if (!get().isTranslating || get().isCancelling) return;
      set({ isCancelling: true });
      try {
        await invoke("cancel_translation");
      } catch (err) {
        set({ isCancelling: false });
        toast.error(errorMessage(err), { id: RUN_TOAST_ID });
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
      }),
  };
});

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
