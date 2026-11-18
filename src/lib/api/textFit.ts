import { invoke } from "@tauri-apps/api/core";
import type {
  FitOverride,
  FitScan,
  TextFitPreview,
  TextFitSettings,
  TextFitView,
} from "@/lib/types";

/** Text fitting: settings, scan, preview and per-text choices. */
export const textFitApi = {
  get: (projectId: string) =>
    invoke<TextFitView>("get_text_fit", { projectId }),
  scan: (args: { projectId: string; settings: TextFitSettings | null }) =>
    invoke<FitScan>("scan_text_fit", args),
  save: (args: { projectId: string; settings: TextFitSettings }) =>
    invoke<TextFitSettings>("set_text_fit", args),
  preview: (args: {
    projectId: string;
    segmentId: string;
    draftTarget: string | null;
  }) => invoke<TextFitPreview | null>("preview_text_fit", args),
  setOverride: (args: {
    projectId: string;
    segmentId: string;
    choice: FitOverride | null;
  }) => invoke("set_text_fit_override", args),
  gameFont: (projectId: string) =>
    invoke<ArrayBuffer>("get_game_font", { projectId }),
};
