import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { save } from "@tauri-apps/plugin-dialog";
import { AlertTriangle, CheckCircle, FileDown } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { useActiveSegmentId } from "@/stores/editor";
import { useProjectStore } from "@/stores/project";
import type { QaErrorType, QaReport, QaResult } from "@/lib/types";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Error row
// ---------------------------------------------------------------------------

function errorIcon(type: QaErrorType["type"]) {
  switch (type) {
    case "missing_placeholder":
      return <AlertTriangle className="h-3 w-3 text-red-400 shrink-0" />;
    case "line_too_long":
      return <AlertTriangle className="h-3 w-3 text-yellow-400 shrink-0" />;
    case "bom_detected":
      return <AlertTriangle className="h-3 w-3 text-yellow-400 shrink-0" />;
  }
}

function errorLabel(
  error: QaErrorType,
  t: (key: string, opts?: Record<string, unknown>) => string,
): string {
  switch (error.type) {
    case "missing_placeholder":
      return t("qaPanel.errors.missing_placeholder", {
        name: error.placeholder,
      });
    case "line_too_long":
      return t("qaPanel.errors.line_too_long", {
        line: error.line,
        units: error.units.toFixed(1),
        maxUnits: error.max_units.toFixed(1),
        chars: error.char_count,
      });
    case "bom_detected":
      return t("qaPanel.errors.bom_detected");
  }
}

// ---------------------------------------------------------------------------
// Score ring
// ---------------------------------------------------------------------------

const QA_RING_RADIUS = 22;
const QA_RING_CIRCUMFERENCE = 2 * Math.PI * QA_RING_RADIUS;

function QAScoreRing({ score }: { score: number }) {
  const offset = QA_RING_CIRCUMFERENCE * (1 - score / 100);
  const colorClass =
    score === 100
      ? "text-star"
      : score >= 75
        ? "text-yellow-400"
        : "text-red-400";

  return (
    <div className="relative h-[52px] w-[52px] shrink-0">
      <svg width="52" height="52" className="-rotate-90">
        <circle
          cx="26"
          cy="26"
          r={QA_RING_RADIUS}
          fill="none"
          stroke="currentColor"
          strokeWidth="3"
          className="text-primary/15"
        />
        <circle
          cx="26"
          cy="26"
          r={QA_RING_RADIUS}
          fill="none"
          stroke="currentColor"
          strokeWidth="3"
          strokeLinecap="round"
          strokeDasharray={QA_RING_CIRCUMFERENCE}
          strokeDashoffset={offset}
          className={cn(
            "transition-[stroke-dashoffset] duration-300",
            colorClass,
          )}
        />
      </svg>
      <div
        className={cn(
          "absolute inset-0 flex items-center justify-center font-mono text-sm font-semibold tabular-nums",
          colorClass,
        )}
      >
        {score}
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// QAPanel
// ---------------------------------------------------------------------------

interface QAPanelProps {
  /** Source text to QA-check in real time (active segment). */
  sourceText: string | null;
  /** Current target text draft (may not be saved yet). */
  targetText: string | null;
}

export function QAPanel({ sourceText, targetText }: QAPanelProps) {
  const { t, i18n } = useTranslation();
  const activeSegmentId = useActiveSegmentId();
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const [isExporting, setIsExporting] = useState(false);

  const handleExport = async () => {
    if (!activeProjectId) return;
    const path = await save({
      filters: [{ name: "HTML", extensions: ["html"] }],
      defaultPath: "qa-report.html",
    });
    if (!path) return;
    setIsExporting(true);
    invoke("export_qa_report", {
      projectId: activeProjectId,
      outputPath: path,
      lang: i18n.language,
    })
      .then(() => toast.success(t("qaPanel.exportSuccess")))
      .catch((e) => toast.error(t("qaPanel.exportError", { error: String(e) })))
      .finally(() => setIsExporting(false));
  };

  // Real-time QA: invoked as a query keyed on source+target text
  // Uses a simple hash to avoid re-running on identical input
  const { data: qaResult } = useQuery<QaResult>({
    queryKey: ["qa-check", sourceText, targetText],
    queryFn: () =>
      invoke<QaResult>("qa_check_segment", {
        sourceText: sourceText!,
        targetText: targetText ?? "",
      }),
    enabled: !!sourceText,
    staleTime: 300,
  });

  // Project QA report badge
  const { data: qaReport } = useQuery<QaReport>({
    queryKey: ["qa-report", activeProjectId],
    queryFn: () =>
      invoke<QaReport>("get_qa_report", { projectId: activeProjectId! }),
    enabled: !!activeProjectId,
    staleTime: 1000 * 10, // refresh every 10 s
  });

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Header with project QA badge */}
      <div className="shrink-0 border-b px-3 py-2 flex items-center gap-2">
        <span className="text-[10px] font-semibold uppercase tracking-[0.12em] text-muted-foreground/80 select-none">
          {t("qaPanel.title")}
        </span>
        {qaReport && qaReport.totalSegments > 0 && (
          <span className="text-[10px] text-muted-foreground tabular-nums">
            {qaReport.okCount}/{qaReport.totalSegments} ok
          </span>
        )}
        {activeProjectId && (
          <Button
            variant="ghost"
            size="icon"
            className="h-5 w-5 ml-auto"
            onClick={handleExport}
            disabled={isExporting}
            title={t("qaPanel.export")}
          >
            <FileDown className="h-3 w-3" />
          </Button>
        )}
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        {!activeSegmentId && (
          <p className="py-4 text-center text-xs text-muted-foreground leading-relaxed">
            {t("qaPanel.empty")}
          </p>
        )}

        {activeSegmentId && qaResult && (
          <div className="flex items-center gap-3">
            <QAScoreRing score={qaResult.score} />

            {/* Error list */}
            {qaResult.errors.length === 0 ? (
              <div className="flex items-center gap-1.5 text-xs text-green-400">
                <CheckCircle className="h-3 w-3 shrink-0" />
                <span>{t("qaPanel.ok")}</span>
              </div>
            ) : (
              <ul className="flex-1 space-y-1">
                {qaResult.errors.map((err, i) => (
                  <li
                    key={i}
                    className="flex items-start gap-1.5 rounded bg-muted/30 px-2 py-1.5 text-xs"
                  >
                    {errorIcon(err.type)}
                    <span className="leading-snug">{errorLabel(err, t)}</span>
                  </li>
                ))}
              </ul>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
