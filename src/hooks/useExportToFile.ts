import { useState } from "react";
import { save, type SaveDialogOptions } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";

interface ExportToFileConfig<R> {
  /** Options passed to the native save dialog (filters, defaultPath, …). */
  dialog: SaveDialogOptions;
  /** Tauri command to invoke with the chosen path. */
  command: string;
  /** Extra command args; the chosen path is added as `outputPath`. */
  args?: Record<string, unknown>;
  /** i18n key for the success toast. */
  successKey: string;
  /** Params of the success toast, from the command's result. */
  successParams?: (result: R) => Record<string, unknown>;
  /** i18n key for the error toast (receives `{ error }`). */
  errorKey: string;
}

/**
 * Shared "export to a file" flow (QA report, TM, .h2s pack): open the save
 * dialog, invoke a backend command with the chosen path, and surface a
 * success/error toast — while tracking an `isExporting` flag for the button.
 * Resolves to whether the file was written.
 *
 * A cancelled dialog (`path === null`) is a silent no-op. A dialog error and a
 * command error both raise the same `errorKey` toast, matching the previous
 * per-panel implementations.
 */
export function useExportToFile() {
  const { t } = useTranslation();
  const [isExporting, setIsExporting] = useState(false);

  async function exportToFile<R = unknown>({
    dialog,
    command,
    args = {},
    successKey,
    successParams,
    errorKey,
  }: ExportToFileConfig<R>): Promise<boolean> {
    let path: string | null;
    try {
      path = await save(dialog);
    } catch (e) {
      toast.error(t(errorKey, { error: String(e) }));
      return false;
    }
    if (!path) return false;
    setIsExporting(true);
    try {
      const result = await invoke<R>(command, { ...args, outputPath: path });
      toast.success(t(successKey, successParams?.(result)));
      return true;
    } catch (e) {
      toast.error(t(errorKey, { error: String(e) }));
      return false;
    } finally {
      setIsExporting(false);
    }
  }

  return { isExporting, exportToFile };
}
