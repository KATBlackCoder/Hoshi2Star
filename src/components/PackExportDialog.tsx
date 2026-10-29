import { useState } from "react";
import { useTranslation } from "react-i18next";
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
import { H2S_FILTER } from "@/lib/constants";
import { useExportToFile } from "@/hooks/useExportToFile";

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
 * Hoshi2Star user can resume the translation on their own copy of the game.
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
  const { isExporting, exportToFile } = useExportToFile();

  async function handleExport() {
    const exported = await exportToFile<PackExportSummary>({
      dialog: {
        title: t("pack.exportTitle"),
        defaultPath: `${projectName}.h2s`,
        filters: [H2S_FILTER],
      },
      command: "export_h2s_pack",
      args: { projectId, includeTm, langPair },
      successKey: "pack.exportDone",
      successParams: (summary) => ({
        segments: summary.segmentCount,
        files: summary.fileCount,
      }),
      errorKey: "pack.exportError",
    });
    if (exported) onClose();
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
