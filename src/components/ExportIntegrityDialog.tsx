import { useTranslation } from "react-i18next";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";

// ---------------------------------------------------------------------------
// ExportIntegrityDialog — the export was refused by the integrity check
// ---------------------------------------------------------------------------

interface ExportIntegrityDialogProps {
  /** Problem lines reported by the check; `null` hides the dialog. */
  issues: string[] | null;
  onClose: () => void;
}

export function ExportIntegrityDialog({
  issues,
  onClose,
}: ExportIntegrityDialogProps) {
  const { t } = useTranslation();
  return (
    <AlertDialog open={issues !== null}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("exportIntegrity.title")}</AlertDialogTitle>
          <AlertDialogDescription>
            {t("exportIntegrity.description")}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <ul className="max-h-64 space-y-1 overflow-y-auto rounded-md border bg-muted/40 p-2 font-mono text-xs break-all">
          {(issues ?? []).map((issue, index) => (
            <li key={index}>{issue}</li>
          ))}
        </ul>
        <AlertDialogFooter>
          <AlertDialogAction onClick={onClose}>
            {t("toolbar.exportAllClose")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
