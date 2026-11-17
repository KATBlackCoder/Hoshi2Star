import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { create } from "zustand";
import { errorMessage } from "@/lib/utils";
import { EVENTS } from "@/lib/events";
import type {
  PilotPhase,
  PilotProgressPayload,
  PilotRunReport,
  PilotSampleCategory,
  Project,
  ProviderConfig,
} from "@/lib/types";
import { llmApi } from "@/lib/api/llm";
import { hasPilot } from "@/lib/engine";

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
    if (get().isRunning || !hasPilot(project.engine)) return;

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
        EVENTS.pilotProgress,
        (event) => set({ progress: event.payload }),
      );
      const report = await llmApi.runPilot({
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
