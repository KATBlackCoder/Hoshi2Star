import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { applyResync, useResyncStore } from "@/stores/resync";
import { useIsTranslating } from "@/stores/llm";

// ---------------------------------------------------------------------------
// ResyncDialog — realign the project after a game update (MV/MZ)
// ---------------------------------------------------------------------------

interface ResyncDialogProps {
  projectId: string | null;
}

/**
 * Preview, then apply, the realignment computed by `resync_project`:
 * translations follow their lines to their new place, new lines arrive
 * untranslated, translations of lines that are gone are kept aside and in the
 * translation memory.
 */
export function ResyncDialog({ projectId }: ResyncDialogProps) {
  const { t } = useTranslation();
  const { open, loading, summary, error, setOpen } = useResyncStore();
  const isTranslating = useIsTranslating();
  const applied = summary?.applied ?? false;
  const nothingToDo = summary !== null && summary.files.length === 0;

  return (
    <Dialog open={open} onOpenChange={(next) => !loading && setOpen(next)}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{t("resync.title")}</DialogTitle>
          <DialogDescription>
            {t(applied ? "resync.appliedDescription" : "resync.description")}
          </DialogDescription>
        </DialogHeader>

        {loading && (
          <p className="flex items-center gap-2 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" aria-hidden />
            {t("resync.loading")}
          </p>
        )}
        {error && (
          <p className="rounded-md bg-destructive/10 p-2 text-sm text-destructive">
            {error}
          </p>
        )}

        {summary && !loading && nothingToDo && (
          <p className="text-sm text-muted-foreground">
            {t("resync.nothingToDo")}
          </p>
        )}
        {summary && !loading && !nothingToDo && (
          <div className="space-y-3 text-sm">
            <p>
              {t("resync.totals", {
                moved: summary.moved,
                added: summary.added,
                gone: summary.gone,
                orphaned: summary.orphaned,
              })}
            </p>
            {summary.changedSources > 0 && (
              <p className="text-muted-foreground">
                {t("resync.changedSources", { count: summary.changedSources })}
              </p>
            )}
            <table className="w-full text-xs">
              <thead className="text-left text-muted-foreground">
                <tr>
                  <th className="py-1 font-medium">{t("resync.file")}</th>
                  <th className="py-1 font-medium">{t("resync.moved")}</th>
                  <th className="py-1 font-medium">{t("resync.added")}</th>
                  <th className="py-1 font-medium">{t("resync.gone")}</th>
                </tr>
              </thead>
              <tbody>
                {summary.files.map((file) => (
                  <tr key={file.file} className="border-t">
                    <td className="py-1 font-mono break-all">
                      {file.file}
                      {file.newFile && ` (${t("resync.newFile")})`}
                    </td>
                    <td className="py-1">{file.moved}</td>
                    <td className="py-1">{file.added}</td>
                    <td className="py-1">{file.gone}</td>
                  </tr>
                ))}
              </tbody>
            </table>
            {summary.goneTranslations.length > 0 && (
              <section>
                <h3 className="mb-1 text-xs font-medium">
                  {t("resync.goneTranslations")}
                </h3>
                <ul className="max-h-40 space-y-1 overflow-y-auto rounded-md border bg-muted/40 p-2 text-xs break-all">
                  {summary.goneTranslations.map((gone) => (
                    <li key={`${gone.file} ${gone.key}`}>
                      <span className="font-mono text-muted-foreground">
                        {gone.file}
                      </span>{" "}
                      {gone.source} → {gone.target}
                    </li>
                  ))}
                </ul>
              </section>
            )}
          </div>
        )}

        <DialogFooter>
          <Button
            variant="outline"
            disabled={loading}
            onClick={() => setOpen(false)}
          >
            {t(applied ? "resync.close" : "resync.cancel")}
          </Button>
          {!applied && (
            <Button
              disabled={
                loading ||
                !summary ||
                nothingToDo ||
                isTranslating ||
                projectId === null
              }
              onClick={() => projectId && void applyResync(projectId)}
            >
              {t("resync.apply")}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
