import { invoke } from "@tauri-apps/api/core";
import type { ImportPolicy, ImportPreview, ImportReport } from "@/lib/types";

/** `.h2s` exchange packs. */
export const packApi = {
  previewImport: (args: {
    projectId: string;
    packPath: string;
    langPair: string;
  }) => invoke<ImportPreview>("preview_h2s_import", args),
  applyImport: (args: {
    projectId: string;
    packPath: string;
    policy: ImportPolicy;
    applySourceChanged: boolean;
    importTerminology: boolean;
    importTm: boolean;
    langPair: string;
  }) => invoke<ImportReport>("apply_h2s_import", args),
};
