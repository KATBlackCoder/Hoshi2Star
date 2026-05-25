import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
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

  // Actions
  setProviderConfig: (cfg: Partial<ProviderConfig>) => void;
  startTranslation: (segmentIds: string[], fileId?: string) => Promise<void>;
  reset: () => void;
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

export const useLlmStore = create<LlmState>()((set, get) => ({
  isTranslating: false,
  translationProgress: -1,
  providerConfig: { ...DEFAULT_CONFIG },
  error: null,

  setProviderConfig: (cfg) =>
    set((s) => ({ providerConfig: { ...s.providerConfig, ...cfg } })),

  startTranslation: async (segmentIds, fileId) => {
    if (get().isTranslating) return;

    set({ isTranslating: true, translationProgress: 0, error: null });

    // Tear down any previous listeners
    progressUnlisten?.();
    completedUnlisten?.();
    errorUnlisten?.();

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
      set({ isTranslating: false, translationProgress: 100 });
      progressUnlisten?.();
      completedUnlisten?.();
      errorUnlisten?.();
    });

    errorUnlisten = await listen<{ message: string }>(
      "h2s://llm/error",
      (event) => {
        set({
          isTranslating: false,
          translationProgress: -1,
          error: event.payload.message,
        });
        progressUnlisten?.();
        completedUnlisten?.();
        errorUnlisten?.();
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
    }
  },

  reset: () =>
    set({ isTranslating: false, translationProgress: -1, error: null }),
}));

// Selectors
export const useIsTranslating = () => useLlmStore((s) => s.isTranslating);
export const useTranslationProgress = () =>
  useLlmStore((s) => s.translationProgress);
export const useProviderConfig = () => useLlmStore((s) => s.providerConfig);
