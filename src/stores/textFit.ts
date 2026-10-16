import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { FitScan, TextFitSettings, TextFitView } from "@/lib/types";

// ---------------------------------------------------------------------------
// Text fitting (RPG Maker MZ): settings dialog state
// ---------------------------------------------------------------------------

interface TextFitState {
  open: boolean;
  loading: boolean;
  scanning: boolean;
  view: TextFitView | null;
  scan: FitScan | null;
  error: string | null;
  /** Bumped when settings or per-text choices change (previews refetch). */
  version: number;
  setOpen: (open: boolean) => void;
  bump: () => void;
}

export const useTextFitStore = create<TextFitState>()((set) => ({
  open: false,
  loading: false,
  scanning: false,
  view: null,
  scan: null,
  error: null,
  version: 0,
  setOpen: (open) => set({ open }),
  bump: () => set((state) => ({ version: state.version + 1 })),
}));

export const useTextFitVersion = () => useTextFitStore((s) => s.version);

/** Load the settings and detected layout, then scan with the saved settings. */
export async function openTextFitDialog(projectId: string): Promise<void> {
  useTextFitStore.setState({
    open: true,
    loading: true,
    error: null,
    scan: null,
  });
  try {
    const view = await invoke<TextFitView>("get_text_fit", { projectId });
    useTextFitStore.setState({ view, loading: false });
    if (view.supported) await rescanTextFit(projectId, null);
  } catch (err) {
    useTextFitStore.setState({ loading: false, error: String(err) });
  }
}

/** Counts for `settings` (unsaved draft) or the saved settings (`null`). */
export async function rescanTextFit(
  projectId: string,
  settings: TextFitSettings | null,
): Promise<void> {
  useTextFitStore.setState({ scanning: true });
  try {
    const scan = await invoke<FitScan>("scan_text_fit", {
      projectId,
      settings,
    });
    useTextFitStore.setState({ scan, scanning: false, error: null });
  } catch (err) {
    useTextFitStore.setState({ scanning: false, error: String(err) });
  }
}

export async function saveTextFit(
  projectId: string,
  settings: TextFitSettings,
): Promise<void> {
  const saved = await invoke<TextFitSettings>("set_text_fit", {
    projectId,
    settings,
  });
  useTextFitStore.setState((state) => ({
    view: state.view ? { ...state.view, settings: saved } : state.view,
    version: state.version + 1,
  }));
}
