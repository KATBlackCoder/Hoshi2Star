import { Loader2, Square } from "lucide-react";
import { Button } from "@/components/ui/button";
import type { TerminologyScanProgress } from "@/lib/types";
import { useTranslation } from "react-i18next";

export function ScanProgress({
  progress,
  onCancel,
}: {
  progress: TerminologyScanProgress;
  onCancel: () => void;
}) {
  const { t } = useTranslation();
  const percent =
    progress.total === 0
      ? 0
      : Math.round((progress.processed / progress.total) * 100);
  return (
    <section
      className="rounded-xl bg-muted/65 p-3 shadow-[var(--shadow-surface)]"
      aria-live="polite"
      aria-busy="true"
    >
      <div className="flex items-center gap-3">
        <Loader2
          className="size-4 shrink-0 animate-spin text-primary"
          aria-hidden="true"
        />
        <div className="min-w-0 flex-1">
          <div className="flex justify-between gap-3 text-xs">
            <span>{t("terminology.scanLocal")}</span>
            <span className="tabular-nums">
              {progress.processed}/{progress.total} · {progress.discovered}{" "}
              termes
            </span>
          </div>
          <div
            className="mt-2 h-1.5 overflow-hidden rounded-full bg-background"
            role="progressbar"
            aria-valuemin={0}
            aria-valuemax={progress.total}
            aria-valuenow={progress.processed}
            aria-label="Progression de l’analyse"
          >
            <div
              className="h-full rounded-full bg-primary transition-[width] duration-[var(--duration-normal)]"
              style={{ width: `${percent}%` }}
            />
          </div>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={onCancel}
          aria-label={t("terminology.cancelScan")}
        >
          <Square className="size-3" /> {t("terminology.cancel")}
        </Button>
      </div>
    </section>
  );
}
