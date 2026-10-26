import { useTranslation } from "react-i18next";
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
import { parseSourceChange } from "@/lib/exportIntegrity";

// ---------------------------------------------------------------------------
// ExportIntegrityDialog — the export was refused by the integrity check
// ---------------------------------------------------------------------------

interface ExportIntegrityDialogProps {
  /** Problem lines reported by the check; `null` hides the dialog. */
  issues: string[] | null;
  onClose: () => void;
  /** Offered when the game changed since extraction (MV/MZ realignment). */
  onResync?: () => void;
}

export function ExportIntegrityDialog({
  issues,
  onClose,
  onResync,
}: ExportIntegrityDialogProps) {
  const { t } = useTranslation();
  const lines = (issues ?? []).map((issue) => ({
    issue,
    change: parseSourceChange(issue),
  }));
  const gameChanged = lines.some(({ change }) => change !== null);
  return (
    <AlertDialog open={issues !== null}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("exportIntegrity.title")}</AlertDialogTitle>
          <AlertDialogDescription>
            {t(
              gameChanged
                ? "exportIntegrity.gameChanged"
                : "exportIntegrity.description",
            )}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <ul className="max-h-64 space-y-1 overflow-y-auto rounded-md border bg-muted/40 p-2 font-mono text-xs break-all">
          {lines.map(({ issue, change }, index) =>
            change ? (
              <li key={index} className="space-y-0.5">
                <div>
                  {change.file} {change.pointer}
                </div>
                <div className="pl-2">
                  {t("exportIntegrity.expected", { text: change.expected })}
                </div>
                <div className="pl-2">
                  {change.found === null
                    ? t("exportIntegrity.foundNothing")
                    : t("exportIntegrity.found", { text: change.found })}
                </div>
              </li>
            ) : (
              <li key={index}>{issue}</li>
            ),
          )}
        </ul>
        <AlertDialogFooter>
          {gameChanged && onResync ? (
            <>
              <AlertDialogCancel onClick={onClose}>
                {t("toolbar.exportAllClose")}
              </AlertDialogCancel>
              <AlertDialogAction
                onClick={() => {
                  onClose();
                  onResync();
                }}
              >
                {t("exportIntegrity.resync")}
              </AlertDialogAction>
            </>
          ) : (
            <AlertDialogAction onClick={onClose}>
              {t("toolbar.exportAllClose")}
            </AlertDialogAction>
          )}
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
