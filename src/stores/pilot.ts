import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";
import type {
  PilotPhase,
  PilotProgressPayload,
  PilotRunReport,
  PilotSampleCategory,
  Project,
  ProviderConfig,
} from "@/lib/types";

export type PilotCategoryFilter = "all" | PilotSampleCategory;
export type PilotReviewDecision =
  | "baseline"
  | "contextual"
  | "equivalent"
  | "review";

interface PilotState {
  isOpen: boolean;
  isRunning: boolean;
  sampleSize: number;
  progress: PilotProgressPayload | null;
  report: PilotRunReport | null;
  error: string | null;
  categoryFilter: PilotCategoryFilter;
  decisions: Record<string, PilotReviewDecision>;
  openWorkspace: () => void;
  closeWorkspace: () => void;
  setSampleSize: (sampleSize: number) => void;
  setCategoryFilter: (category: PilotCategoryFilter) => void;
  setDecision: (stableKey: string, decision: PilotReviewDecision) => void;
  clearReport: () => void;
  runPilot: (project: Project, providerConfig: ProviderConfig) => Promise<void>;
}

let activeProgressUnlisten: UnlistenFn | null = null;

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

export const usePilotStore = create<PilotState>()((set, get) => ({
  isOpen: false,
  isRunning: false,
  sampleSize: 12,
  progress: null,
  report: null,
  error: null,
  categoryFilter: "all",
  decisions: {},

  openWorkspace: () => set({ isOpen: true, error: null }),
  closeWorkspace: () => {
    if (get().isRunning) return;
    set({ isOpen: false });
  },
  setSampleSize: (sampleSize) =>
    set({ sampleSize: Math.min(50, Math.max(4, Math.round(sampleSize))) }),
  setCategoryFilter: (categoryFilter) => set({ categoryFilter }),
  setDecision: (stableKey, decision) =>
    set((state) => ({
      decisions: { ...state.decisions, [stableKey]: decision },
    })),
  clearReport: () =>
    set({
      report: null,
      progress: null,
      error: null,
      categoryFilter: "all",
      decisions: {},
    }),

  runPilot: async (project, providerConfig) => {
    if (get().isRunning || project.engine !== "mv_mz") return;

    activeProgressUnlisten?.();
    activeProgressUnlisten = null;
    set({
      isOpen: true,
      isRunning: true,
      progress: { phase: "preparing", done: 0, total: 1 },
      report: null,
      error: null,
      categoryFilter: "all",
      decisions: {},
    });

    try {
      activeProgressUnlisten = await listen<PilotProgressPayload>(
        "h2s://pilot/progress",
        (event) => set({ progress: event.payload }),
      );
      const report = await invoke<PilotRunReport>("run_mv_mz_pilot", {
        gamePath: project.gamePath,
        sourceLang: project.sourceLang,
        targetLang: project.targetLang,
        sampleSize: get().sampleSize,
        providerConfig,
      });
      set({
        report,
        isRunning: false,
        progress: {
          phase: "quality_review",
          done: report.comparisons.length,
          total: report.comparisons.length,
        },
      });
    } catch (error) {
      set({ isRunning: false, error: errorMessage(error), progress: null });
    } finally {
      activeProgressUnlisten?.();
      activeProgressUnlisten = null;
    }
  },
}));

export const PILOT_PHASES: PilotPhase[] = [
  "preparing",
  "baseline",
  "contextual",
  "quality_review",
];
