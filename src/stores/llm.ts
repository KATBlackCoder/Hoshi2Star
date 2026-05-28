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

interface LlmState {
  isTranslating: boolean;
  /** 0–100, -1 when idle */
  translationProgress: number;
  providerConfig: ProviderConfig;
  error: string | null;
  /** timestamp Date.now() when translation started, null when idle */
  translationStartTime: number | null;
  /** fileId → elapsed seconds after completion */
  fileTranslationTimes: Record<string, number>;

  // Actions
  setProviderConfig: (cfg: Partial<ProviderConfig>) => void;
  startTranslation: (segmentIds: string[], fileId?: string) => Promise<void>;
  reset: () => void;
  startTimer: () => void;
  stopTimer: (fileId: string) => void;
  clearFileTime: (fileId: string) => void;
}

// ---------------------------------------------------------------------------
// Default config — matches OllamaProvider defaults in Rust
// ---------------------------------------------------------------------------

const DEFAULT_CONFIG: ProviderConfig = {
  url: "http://localhost:11434",
  model: "qwen3:4b",
};

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

let progressUnlisten: UnlistenFn | null = null;
let completedUnlisten: UnlistenFn | null = null;
let errorUnlisten: UnlistenFn | null = null;
let warningUnlisten: UnlistenFn | null = null;

export const useLlmStore = create<LlmState>()((set, get) => ({
  isTranslating: false,
  translationProgress: -1,
  providerConfig: { ...DEFAULT_CONFIG },
  error: null,
  translationStartTime: null,
  fileTranslationTimes: {},

  setProviderConfig: (cfg) =>
    set((s) => ({ providerConfig: { ...s.providerConfig, ...cfg } })),

  startTimer: () => set({ translationStartTime: Date.now() }),

  stopTimer: (fileId) => {
    const { translationStartTime, fileTranslationTimes } = get();
    if (translationStartTime === null) return;
    const elapsed = Math.floor((Date.now() - translationStartTime) / 1000);
    set({
      translationStartTime: null,
      fileTranslationTimes: { ...fileTranslationTimes, [fileId]: elapsed },
    });
  },

  clearFileTime: (fileId) =>
    set((s) => {
      const times = { ...s.fileTranslationTimes };
      delete times[fileId];
      return { fileTranslationTimes: times };
    }),

  startTranslation: async (segmentIds, fileId) => {
    if (get().isTranslating) return;

    set({ isTranslating: true, translationProgress: 0, error: null });
    get().startTimer();

    // Tear down any previous listeners
    progressUnlisten?.();
    completedUnlisten?.();
    errorUnlisten?.();
    warningUnlisten?.();

    // Listen to progress events from Rust pipeline
    progressUnlisten = await listen<ProgressPayload>(
      "h2s://llm/progress",
      (event) => {
        const { done, total } = event.payload;
        const pct = total > 0 ? Math.round((done / total) * 100) : 0;
        set({ translationProgress: pct });
      },
    );

    completedUnlisten = await listen("h2s://llm/completed", () => {
      if (fileId) get().stopTimer(fileId);
      set({ isTranslating: false, translationProgress: 100 });
      progressUnlisten?.();
      completedUnlisten?.();
      errorUnlisten?.();
      warningUnlisten?.();
    });

    errorUnlisten = await listen<{ message: string }>(
      "h2s://llm/error",
      (event) => {
        set({
          isTranslating: false,
          translationProgress: -1,
          error: event.payload.message,
        });
        toast.error(`Erreur de traduction : ${event.payload.message}`, {
          duration: 6000,
        });
        progressUnlisten?.();
        completedUnlisten?.();
        errorUnlisten?.();
        warningUnlisten?.();
      },
    );

    warningUnlisten = await listen<{ segmentId: string }>(
      "h2s://llm/placeholder-warning",
      (event) => {
        const shortId = event.payload.segmentId.slice(0, 8);
        toast.warning(
          `⚠️ Segment ${shortId}… : placeholder non préservé — marqué comme 'À réviser'`,
          { duration: 5000 },
        );
      },
    );

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
      progressUnlisten?.();
      completedUnlisten?.();
      errorUnlisten?.();
      warningUnlisten?.();
    }
  },

  reset: () =>
    set({
      isTranslating: false,
      translationProgress: -1,
      error: null,
      translationStartTime: null,
    }),
}));

// Selectors
export const useIsTranslating = () => useLlmStore((s) => s.isTranslating);
export const useTranslationProgress = () =>
  useLlmStore((s) => s.translationProgress);
export const useProviderConfig = () => useLlmStore((s) => s.providerConfig);
export const useTranslationStartTime = () =>
  useLlmStore((s) => s.translationStartTime);
export const useFileTranslationTimes = () =>
  useLlmStore((s) => s.fileTranslationTimes);
