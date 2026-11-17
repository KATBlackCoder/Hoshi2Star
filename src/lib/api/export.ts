import { invoke } from "@tauri-apps/api/core";
import type {
  DebugExport,
  ExportPreflight,
  FontScanResult,
} from "@/lib/types";

/** Export of the game, its checks, and exports to a chosen file. */
export const exportApi = {
  preflight: (projectId: string) =>
    invoke<ExportPreflight>("preflight_export", { projectId }),
  scanFontStatus: (projectId: string) =>
    invoke<FontScanResult>("scan_font_status", { projectId }),
  stripFontPrefixes: (projectId: string) =>
    invoke("strip_font_prefixes", { projectId }),
  exportProject: (args: {
    projectId: string;
    fontSize: number | null;
    replaceExisting: boolean;
    setLocale: boolean | null;
  }) => invoke<string>("export_project", args),
  exportDebug: (projectId: string) =>
    invoke<DebugExport>("export_debug_project", { projectId }),
  /** A command that writes to `outputPath` (QA report, TM, `.h2s` pack). */
  toFile: <R>(command: string, args: Record<string, unknown>) =>
    invoke<R>(command, args),
};
