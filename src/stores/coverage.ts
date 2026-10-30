import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  CategoryChange,
  CoverageStatus,
  CoverageView,
} from "@/lib/types";

// ---------------------------------------------------------------------------
// Optional text coverage (MV/MZ plugin & system UI text)
// ---------------------------------------------------------------------------

interface CoverageState {
  open: boolean;
  loading: boolean;
  view: CoverageView | null;
  error: string | null;
  setOpen: (open: boolean) => void;
  setView: (view: CoverageView | null) => void;
}

export const useCoverageStore = create<CoverageState>()((set) => ({
  open: false,
  loading: false,
  view: null,
  error: null,
  setOpen: (open) => set({ open }),
  setView: (view) => set({ view }),
}));

/** Scan the project and open the coverage dialog. */
export async function openCoverageDialog(projectId: string): Promise<void> {
  useCoverageStore.setState({ open: true, loading: true, error: null });
  try {
    const view = await invoke<CoverageView>("scan_text_coverage", {
      projectId,
    });
    useCoverageStore.setState({ view, loading: false });
  } catch (err) {
    useCoverageStore.setState({ loading: false, error: String(err) });
  }
}

/** Apply decisions; the backend re-scans and validates every change. */
export async function applyCoverageChanges(
  projectId: string,
  changes: CategoryChange[],
): Promise<CoverageView> {
  useCoverageStore.setState({ loading: true, error: null });
  try {
    const view = await invoke<CoverageView>("set_text_categories", {
      projectId,
      changes,
    });
    useCoverageStore.setState({ view, loading: false });
    return view;
  } catch (err) {
    useCoverageStore.setState({ loading: false, error: String(err) });
    throw err;
  }
}

/** Outcome of the post-open step, used for the user-facing notice. */
export type PostOpenCoverage =
  | { kind: "none" }
  /** Fresh extraction: this many categories can be enabled safely. */
  | { kind: "available"; categories: number }
  /** Restored project that never decided (not scanned). */
  | { kind: "undecided" };

/**
 * After opening an MV/MZ project, decide whether to offer the coverage
 * review. Nothing is ever enabled here: the user decides in the dialog.
 * - fresh extraction → scan (its snapshot was just taken) and offer the
 *   review only when some category can be enabled;
 * - restored project → no scan (it could snapshot the whole game on opening);
 *   offer the review while no decision was ever recorded.
 */
export async function applyPostOpenCoverage(
  projectId: string,
  wasRestored: boolean,
): Promise<PostOpenCoverage> {
  if (wasRestored) {
    const status = await invoke<CoverageStatus>("get_text_coverage_status", {
      projectId,
    });
    return status.supported && !status.hasDecisions
      ? { kind: "undecided" }
      : { kind: "none" };
  }
  const view = await invoke<CoverageView>("scan_text_coverage", { projectId });
  if (!view.supported || view.hasDecisions) return { kind: "none" };
  const offered = view.categories.filter(
    (category) => category.state !== "blocked",
  ).length;
  return offered > 0
    ? { kind: "available", categories: offered }
    : { kind: "none" };
}
