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
import { useUpdaterStore, useUpdaterProgress } from "@/stores/updater";

// ---------------------------------------------------------------------------
// UpdateDialog — self-contained, driven by the updater store's state machine.
// Visible for `available` (yes/no), `downloading` (progress) and `ready`
// (restart). Hidden for idle / checking / dismissed / error.
// ---------------------------------------------------------------------------

export function UpdateDialog() {
  const { t } = useTranslation();
  const status = useUpdaterStore((s) => s.status);
  const version = useUpdaterStore((s) => s.version);
  const notes = useUpdaterStore((s) => s.notes);
  const progress = useUpdaterProgress();
  const startDownload = useUpdaterStore((s) => s.startDownload);
  const dismiss = useUpdaterStore((s) => s.dismiss);
  const postpone = useUpdaterStore((s) => s.postpone);
  const applyAndRestart = useUpdaterStore((s) => s.applyAndRestart);

  const open =
    status === "available" || status === "downloading" || status === "ready";

  return (
    <AlertDialog open={open}>
      <AlertDialogContent>
        {status === "available" && (
          <>
            <AlertDialogHeader>
              <AlertDialogTitle>{t("updater.title")}</AlertDialogTitle>
              <AlertDialogDescription>
                {t("updater.body", { version: version ?? "" })}
              </AlertDialogDescription>
            </AlertDialogHeader>
            {notes && (
              <div className="max-h-40 overflow-y-auto whitespace-pre-wrap rounded-md border bg-muted/30 p-3 text-xs text-muted-foreground">
                <p className="mb-1 font-medium text-foreground">
                  {t("updater.notesLabel")}
                </p>
                {notes}
              </div>
            )}
            <AlertDialogFooter>
              <AlertDialogCancel onClick={() => void dismiss()}>
                {t("updater.no")}
              </AlertDialogCancel>
              <AlertDialogAction onClick={() => void startDownload()}>
                {t("updater.yes")}
              </AlertDialogAction>
            </AlertDialogFooter>
          </>
        )}

        {status === "downloading" && (
          <>
            <AlertDialogHeader>
              <AlertDialogTitle>{t("updater.title")}</AlertDialogTitle>
              <AlertDialogDescription>
                {t("updater.downloading")}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <div className="flex flex-col gap-1.5 py-1">
              <div className="h-2 w-full overflow-hidden rounded-full bg-muted">
                <div
                  className="h-full bg-primary transition-all"
                  style={{ width: `${progress}%` }}
                />
              </div>
              <span className="text-right font-mono text-[11px] tabular-nums text-muted-foreground">
                {progress}%
              </span>
            </div>
          </>
        )}

        {status === "ready" && (
          <>
            <AlertDialogHeader>
              <AlertDialogTitle>{t("updater.readyTitle")}</AlertDialogTitle>
              <AlertDialogDescription>
                {t("updater.readyBody")}
              </AlertDialogDescription>
            </AlertDialogHeader>
            <AlertDialogFooter>
              <AlertDialogCancel onClick={postpone}>
                {t("updater.later")}
              </AlertDialogCancel>
              <AlertDialogAction onClick={() => void applyAndRestart()}>
                {t("updater.restart")}
              </AlertDialogAction>
            </AlertDialogFooter>
          </>
        )}
      </AlertDialogContent>
    </AlertDialog>
  );
}
