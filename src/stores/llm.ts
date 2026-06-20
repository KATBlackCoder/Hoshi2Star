import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { toast } from "sonner";
import type { ProviderConfig } from "@/lib/types";

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

  // Actions
  setProviderConfig: (cfg: Partial<ProviderConfig>) => void;
  startTranslation: (segmentIds: string[], fileId?: string) => Promise<void>;
  startTranslateAll: (
    projectId: string,
    thresholdMins: number,
    cooldownMins: number,
  ) => Promise<void>;
  reset: () => void;
  startTimer: () => void;
  stopTimer: () => void;
}

// ---------------------------------------------------------------------------
// Default config — matches OllamaProvider defaults in Rust
// ---------------------------------------------------------------------------

const DEFAULT_CONFIG: ProviderConfig = {
  url: "http://localhost:11434",
  model: "qwen3:4b",
  batchSize: 20,
};

// ---------------------------------------------------------------------------
// Listener helpers
// ---------------------------------------------------------------------------

// Single teardown for any active translation session.
let activeTeardown: (() => void) | null = null;

interface TranslationListenerOpts {
  onProgress: (done: number, total: number) => void;
  onCompleted: () => void;
  onError: (msg: string) => void;
  onWarning: (segmentId: string) => void;
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
    await listen<ProgressPayload>("h2s://llm/progress", (event) => {
      opts.onProgress(event.payload.done, event.payload.total);
    }),
  );

  fns.push(
    await listen("h2s://llm/completed", () => {
      opts.onCompleted();
      teardown();
    }),
  );

  fns.push(
    await listen<{ message: string }>("h2s://llm/error", (event) => {
      opts.onError(event.payload.message);
      teardown();
    }),
  );

  fns.push(
    await listen<{ segmentId: string }>(
      "h2s://llm/placeholder-warning",
      (event) => {
        opts.onWarning(event.payload.segmentId);
      },
    ),
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

  setProviderConfig: (cfg) =>
    set((s) => ({ providerConfig: { ...s.providerConfig, ...cfg } })),

  startTimer: () => set({ translationStartTime: Date.now() }),

  stopTimer: () => set({ translationStartTime: null }),

  startTranslation: async (segmentIds, fileId) => {
    if (get().isTranslating) return;

    set({ isTranslating: true, translationProgress: 0, error: null });
    get().startTimer();

    activeTeardown?.();

    activeTeardown = await setupTranslationListeners({
      onProgress: (done, total) => {
        const pct = total > 0 ? Math.round((done / total) * 100) : 0;
        set({ translationProgress: pct });
      },
      onCompleted: () => {
        get().stopTimer();
        set({
          isTranslating: false,
          translationProgress: 100,
          isCooling: false,
          cooldownRemaining: 0,
        });
        activeTeardown = null;
      },
      onError: (msg) => {
        set({
          isTranslating: false,
          translationProgress: -1,
          isCooling: false,
          cooldownRemaining: 0,
          error: msg,
        });
        toast.error(`Erreur de traduction : ${msg}`, { duration: 6000 });
        activeTeardown = null;
      },
      onWarning: (segmentId) => {
        const shortId = segmentId.slice(0, 8);
        toast.warning(
          `⚠️ Segment ${shortId}… : placeholder non préservé — marqué comme 'À réviser'`,
          { duration: 5000 },
        );
      },
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
      translationProgress: 0,
      error: null,
      isCooling: false,
      cooldownRemaining: 0,
    });
    get().startTimer();

    activeTeardown?.();

    activeTeardown = await setupTranslationListeners({
      onProgress: (done, total) => {
        const pct = total > 0 ? Math.round((done / total) * 100) : 0;
        set({ translationProgress: pct });
      },
      onCompleted: () => {
        get().stopTimer();
        set({
          isTranslating: false,
          translationProgress: 100,
          isCooling: false,
          cooldownRemaining: 0,
        });
        activeTeardown = null;
      },
      onError: (msg) => {
        set({
          isTranslating: false,
          translationProgress: -1,
          isCooling: false,
          cooldownRemaining: 0,
          error: msg,
        });
        toast.error(`Erreur de traduction : ${msg}`, { duration: 6000 });
        activeTeardown = null;
      },
      onWarning: (segmentId) => {
        const shortId = segmentId.slice(0, 8);
        toast.warning(
          `⚠️ Segment ${shortId}… : placeholder non préservé — marqué comme 'À réviser'`,
          { duration: 5000 },
        );
      },
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

  reset: () =>
    set({
      isTranslating: false,
      translationProgress: -1,
      error: null,
      translationStartTime: null,
      isCooling: false,
      cooldownRemaining: 0,
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
export const useCooldownRemaining = () =>
  useLlmStore((s) => s.cooldownRemaining);
