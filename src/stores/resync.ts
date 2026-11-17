import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { ResyncSummary } from "@/lib/types";
import { errorMessage } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Realignment after a game update (MV/MZ)
// ---------------------------------------------------------------------------

interface ResyncState {
  open: boolean;
  loading: boolean;
  /** Preview until applied (`summary.applied`). */
  summary: ResyncSummary | null;
  error: string | null;
  setOpen: (open: boolean) => void;
}

export const useResyncStore = create<ResyncState>()((set) => ({
  open: false,
  loading: false,
  summary: null,
  error: null,
  setOpen: (open) => set({ open }),
}));

/** Ask the backend for the realignment, applied or as a preview. */
async function runResync(projectId: string, apply: boolean): Promise<void> {
  try {
    const summary = await invoke<ResyncSummary>("resync_project", {
      projectId,
      apply,
    });
    useResyncStore.setState({ summary, loading: false });
  } catch (err) {
    useResyncStore.setState({ loading: false, error: errorMessage(err) });
  }
}

/** Open the dialog with a preview: nothing is written yet. */
export async function openResyncDialog(projectId: string): Promise<void> {
  useResyncStore.setState({
    open: true,
    loading: true,
    summary: null,
    error: null,
  });
  await runResync(projectId, false);
}

/** Apply the realignment (recomputed by the backend, in one transaction). */
export async function applyResync(projectId: string): Promise<void> {
  useResyncStore.setState({ loading: true, error: null });
  // The backend emits `h2s://project/resynced`: the views refresh from it.
  await runResync(projectId, true);
}
