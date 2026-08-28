import { useState } from "react";
import { useTranslation } from "react-i18next";
import { save } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { Loader2 } from "lucide-react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Checkbox } from "@/components/ui/checkbox";
import type { PackExportSummary } from "@/lib/types";

interface PackExportDialogProps {
  open: boolean;
  projectId: string;
  projectName: string;
  langPair: string;
  onClose: () => void;
}

/**
 * "Share project" dialog — exports the project's translation state as a
 * `.h2s` exchange pack (segments + terminology, TM opt-in) so another
 * Hoshiyomi user can resume the translation on their own copy of the game.
 * No game file ever enters the pack.
 */
export function PackExportDialog({
  open,
  projectId,
  projectName,
  langPair,
  onClose,
}: PackExportDialogProps) {
  const { t } = useTranslation();
  const [includeTm, setIncludeTm] = useState(false);
  const [isExporting, setIsExporting] = useState(false);

  async function handleExport() {
    let path: string | null;
    try {
      path = await save({
        title: t("pack.exportTitle"),
        defaultPath: `${projectName}.h2s`,
        filters: [{ name: "Hoshiyomi pack", extensions: ["h2s"] }],
      });
    } catch (e) {
      toast.error(t("pack.exportError", { error: String(e) }));
      return;
    }
    if (!path) return;
    setIsExporting(true);
    try {
      const summary = await invoke<PackExportSummary>("export_h2s_pack", {
        projectId,
        outputPath: path,
        includeTm,
        langPair,
      });
      toast.success(
        t("pack.exportDone", {
          segments: summary.segmentCount,
          files: summary.fileCount,
        }),
      );
      onClose();
    } catch (e) {
      toast.error(t("pack.exportError", { error: String(e) }));
    } finally {
      setIsExporting(false);
    }
  }

  return (
    <AlertDialog open={open}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("pack.exportTitle")}</AlertDialogTitle>
          <AlertDialogDescription>
            {t("pack.exportDesc")}
          </AlertDialogDescription>
        </AlertDialogHeader>

        <label className="flex items-center gap-2 py-1 text-xs">
          <Checkbox
            checked={includeTm}
            onCheckedChange={(v) => setIncludeTm(v === true)}
          />
          {t("pack.exportIncludeTm")}
        </label>

        <AlertDialogFooter>
          <AlertDialogCancel disabled={isExporting} onClick={onClose}>
            {t("pack.cancel")}
          </AlertDialogCancel>
          <AlertDialogAction
            disabled={isExporting}
            onClick={(e) => {
              // Keep the dialog open while the save dialog + command run.
              e.preventDefault();
              void handleExport();
            }}
          >
            {isExporting && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
            {t("pack.exportConfirm")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
