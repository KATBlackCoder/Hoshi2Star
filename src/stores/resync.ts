import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { ResyncSummary } from "@/lib/types";
import { refreshProjectData } from "@/stores/project";

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

/** Open the dialog with a preview: nothing is written yet. */
export async function openResyncDialog(projectId: string): Promise<void> {
  useResyncStore.setState({
    open: true,
    loading: true,
    summary: null,
    error: null,
  });
  try {
    const summary = await invoke<ResyncSummary>("resync_project", {
      projectId,
      apply: false,
    });
    useResyncStore.setState({ summary, loading: false });
  } catch (err) {
    useResyncStore.setState({ loading: false, error: String(err) });
  }
}

/** Apply the realignment (recomputed by the backend, in one transaction). */
export async function applyResync(projectId: string): Promise<void> {
  useResyncStore.setState({ loading: true, error: null });
  try {
    const summary = await invoke<ResyncSummary>("resync_project", {
      projectId,
      apply: true,
    });
    useResyncStore.setState({ summary, loading: false });
    await refreshProjectData(projectId);
  } catch (err) {
    useResyncStore.setState({ loading: false, error: String(err) });
  }
}
