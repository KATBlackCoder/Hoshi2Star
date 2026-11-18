import { useTranslation } from "react-i18next";
import { Loader2 } from "lucide-react";
import { useShallow } from "zustand/react/shallow";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { EVENTS } from "@/lib/events";
import { wolfUnpackErrorDetail, wolfUnpackErrorKey } from "@/lib/projectError";
import type { UnpackProgress, UnpackReport } from "@/lib/types";
import {
  cancelWolfUnpack,
  closeWolfUnpackDialog,
  setWolfUnpackProgress,
  startWolfUnpack,
  useWolfUnpackStore,
} from "@/stores/wolfUnpack";

// ---------------------------------------------------------------------------
// WolfUnpackDialog — decrypt a packed Wolf RPG game before opening it (UWF)
// ---------------------------------------------------------------------------

/**
 * Offered when a Wolf RPG game cannot be opened yet: still packed
 * (`wolf_packed`) or keeping text archives (`wolf_archive_present`). Says what
 * changes in the game folder, runs `unpack_wolf_game` with its progress and a
 * stop button, then shows the report while the game opens again.
 */
export function WolfUnpackDialog() {
  const { t } = useTranslation();
  const {
    gamePath,
    reason,
    archives,
    phase,
    progress,
    report,
    error,
    cancelling,
  } = useWolfUnpackStore(
    useShallow((s) => ({
      gamePath: s.gamePath,
      reason: s.reason,
      archives: s.archives,
      phase: s.phase,
      progress: s.progress,
      report: s.report,
      error: s.error,
      cancelling: s.cancelling,
    })),
  );
  useTauriEvent<UnpackProgress>(
    EVENTS.wolfUnpackProgress,
    setWolfUnpackProgress,
  );

  const running = phase === "running";
  const title =
    phase === "done"
      ? t("wolfUnpack.doneTitle")
      : phase === "error"
        ? t("wolfUnpack.errorTitle")
        : t("wolfUnpack.title");

  return (
    <Dialog
      open={gamePath !== null}
      onOpenChange={(next) => !next && closeWolfUnpackDialog()}
    >
      <DialogContent
        className="max-w-xl"
        onEscapeKeyDown={(event) => running && event.preventDefault()}
        onPointerDownOutside={(event) => running && event.preventDefault()}
      >
        <DialogHeader>
          <DialogTitle>{title}</DialogTitle>
          <DialogDescription className="break-all">
            {gamePath}
          </DialogDescription>
        </DialogHeader>

        {phase === "confirm" && (
          <div className="space-y-3 text-sm">
            <p>
              {t(
                reason === "packed"
                  ? "wolfUnpack.packedDescription"
                  : "wolfUnpack.archivePresentDescription",
                { archives: archives ?? "Data.wolf" },
              )}
            </p>
            <div>
              <p className="font-medium">{t("wolfUnpack.changesTitle")}</p>
              <ul className="mt-1 list-disc space-y-1 pl-5 text-muted-foreground">
                <li>{t("wolfUnpack.changeExtract")}</li>
                <li>{t("wolfUnpack.changePro")}</li>
                <li>{t("wolfUnpack.changeMove")}</li>
              </ul>
            </div>
            <p className="rounded-md bg-amber-500/10 p-2 text-xs">
              {t("wolfUnpack.copyAdvice")}
            </p>
          </div>
        )}

        {running && <UnpackRunning progress={progress} />}

        {phase === "done" && report && <UnpackReportView report={report} />}

        {phase === "error" && error && (
          <div className="space-y-2 text-sm">
            <p>{t(wolfUnpackErrorKey(error))}</p>
            <p className="rounded-md bg-destructive/10 p-2 font-mono text-xs break-all text-destructive">
              {wolfUnpackErrorDetail(error)}
            </p>
          </div>
        )}

        <DialogFooter>
          {phase === "confirm" && (
            <>
              <Button variant="outline" onClick={closeWolfUnpackDialog}>
                {t("wolfUnpack.cancel")}
              </Button>
              <Button onClick={() => void startWolfUnpack()}>
                {t("wolfUnpack.start")}
              </Button>
            </>
          )}
          {running && (
            <Button
              variant="outline"
              disabled={cancelling}
              onClick={() => void cancelWolfUnpack()}
            >
              {cancelling ? t("wolfUnpack.stopping") : t("wolfUnpack.stop")}
            </Button>
          )}
          {(phase === "done" || phase === "error") && (
            <Button onClick={closeWolfUnpackDialog}>
              {t("wolfUnpack.close")}
            </Button>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}

function UnpackRunning({ progress }: { progress: UnpackProgress | null }) {
  const { t } = useTranslation();
  const percent =
    progress && progress.total > 0
      ? Math.round((progress.index / progress.total) * 100)
      : 0;
  return (
    <div className="space-y-2 text-sm">
      <p className="flex items-center gap-2">
        <Loader2 className="size-4 animate-spin text-primary" aria-hidden />
        {t("wolfUnpack.running")}
      </p>
      {progress && (
        <>
          <p className="text-xs text-muted-foreground tabular-nums">
            {t("wolfUnpack.progress", {
              archive: progress.archive,
              index: progress.index,
              total: progress.total,
            })}
          </p>
          <div
            className="h-1.5 overflow-hidden rounded-full bg-muted"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={progress.total}
            aria-valuenow={progress.index}
            aria-label={t("wolfUnpack.running")}
          >
            <div
              className="h-full rounded-full bg-primary"
              style={{ width: `${percent}%` }}
            />
          </div>
        </>
      )}
    </div>
  );
}

function UnpackReportView({ report }: { report: UnpackReport }) {
  const { t } = useTranslation();
  const nothingToDo =
    report.extracted.length === 0 &&
    report.alreadyUnpacked.length === 0 &&
    report.moved.length === 0;
  if (nothingToDo) {
    return (
      <p className="text-sm text-muted-foreground">
        {t("wolfUnpack.nothingToDo")}
      </p>
    );
  }
  const list = (items: string[]) => items.join(", ");
  return (
    <ul className="space-y-1 text-sm">
      {report.extracted.length > 0 && (
        <li>{t("wolfUnpack.extracted", { list: list(report.extracted) })}</li>
      )}
      {report.alreadyUnpacked.length > 0 && (
        <li>
          {t("wolfUnpack.alreadyUnpacked", {
            list: list(report.alreadyUnpacked),
          })}
        </li>
      )}
      {report.protection === "removedV35" && (
        <li>
          {t("wolfUnpack.proRemoved", {
            count: report.unprotectedFiles.length,
          })}
        </li>
      )}
      {report.protection === "alreadyUnprotected" && (
        <li>{t("wolfUnpack.proAlready")}</li>
      )}
      <li>{t("wolfUnpack.moved", { list: list(report.moved) })}</li>
      {report.failed.length > 0 && (
        <li className="text-amber-700 dark:text-amber-400">
          {t("wolfUnpack.failed", {
            list: list(report.failed.map((f) => `${f.archive} (${f.error})`)),
          })}
        </li>
      )}
      {report.warnings.length > 0 && (
        <li className="text-amber-700 dark:text-amber-400">
          {t("wolfUnpack.warnings", { list: list(report.warnings) })}
        </li>
      )}
      <li className="text-muted-foreground">{t("wolfUnpack.doneOpening")}</li>
    </ul>
  );
}
