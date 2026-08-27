import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, CheckCircle, FileDown } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useExportToFile } from "@/hooks/useExportToFile";
import { useActiveSegmentId } from "@/stores/editor";
import { useProjectStore } from "@/stores/project";
import type { QaErrorType, QaReport, QaResult } from "@/lib/types";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Error row
// ---------------------------------------------------------------------------

function errorIcon(error: QaErrorType) {
  switch (error.type) {
    case "missing_placeholder":
      return <AlertTriangle className="h-3 w-3 text-red-400 shrink-0" />;
    case "empty_translation":
    case "unchanged_source":
    case "source_script_remaining":
    case "suspicious_expansion":
    case "context_leak":
    case "inconsistent_repeated_source":
      return <AlertTriangle className="h-3 w-3 text-red-400 shrink-0" />;
    case "line_too_long":
      return <AlertTriangle className="h-3 w-3 text-yellow-400 shrink-0" />;
    case "bom_detected":
      return <AlertTriangle className="h-3 w-3 text-yellow-400 shrink-0" />;
    case "terminology_mismatch":
      return (
        <AlertTriangle
          className={cn(
            "h-3 w-3 shrink-0",
            error.severity === "critical"
              ? "text-red-400"
              : error.severity === "warning"
                ? "text-yellow-400"
                : "text-blue-400",
          )}
        />
      );
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
    case "empty_translation":
      return t("qaPanel.errors.empty_translation");
    case "unchanged_source":
      return t("qaPanel.errors.unchanged_source");
    case "source_script_remaining":
      return t("qaPanel.errors.source_script_remaining", {
        language: error.source_language,
      });
    case "suspicious_expansion":
      return t("qaPanel.errors.suspicious_expansion", {
        sourceChars: error.source_chars,
        targetChars: error.target_chars,
      });
    case "context_leak":
      return t("qaPanel.errors.context_leak", {
        neighbor: error.neighbor_text,
      });
    case "inconsistent_repeated_source":
      return t("qaPanel.errors.inconsistent_repeated_source", {
        variants: error.variants,
      });
    case "terminology_mismatch":
      return t("qaPanel.errors.terminology_mismatch", {
        source: error.source_term,
        target: error.expected_targets.join(" / "),
        severity: t(`qaPanel.severity.${error.severity}`),
      });
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
  const { isExporting, exportToFile } = useExportToFile();

  const handleExport = () => {
    if (!activeProjectId) return;
    void exportToFile({
      dialog: {
        filters: [{ name: "HTML", extensions: ["html"] }],
        defaultPath: "qa-report.html",
      },
      command: "export_qa_report",
      args: { projectId: activeProjectId, lang: i18n.language },
      successKey: "qaPanel.exportSuccess",
      errorKey: "qaPanel.exportError",
    });
  };

  // Real-time QA: invoked as a query keyed on source+target text
  // Uses a simple hash to avoid re-running on identical input
  const hasEmptyTarget = !!activeSegmentId && !(targetText ?? "").trim();
  const { data: qaResult } = useQuery<QaResult>({
    queryKey: ["qa-check", sourceText, targetText, activeProjectId],
    queryFn: () =>
      invoke<QaResult>("qa_check_segment", {
        sourceText: sourceText!,
        targetText: targetText ?? "",
        projectId: activeProjectId,
        segmentId: activeSegmentId,
      }),
    enabled: !!sourceText && !hasEmptyTarget,
    staleTime: 300,
  });

  const displayedQaResult: QaResult | undefined = hasEmptyTarget
    ? { score: 0, errors: [{ type: "empty_translation" }] }
    : qaResult;

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
            {qaReport.criticalCount > 0 && ` · ${qaReport.criticalCount} !`}
          </span>
        )}
        {activeProjectId && (
          <Button
            variant="ghost"
            size="icon"
            className="hit-area-40 relative h-5 w-5 ml-auto"
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

        {activeSegmentId && displayedQaResult && (
          <div className="flex items-center gap-3">
            <QAScoreRing score={displayedQaResult.score} />

            {/* Error list */}
            {displayedQaResult.errors.length === 0 ? (
              <div className="flex items-center gap-1.5 text-xs text-green-400">
                <CheckCircle className="h-3 w-3 shrink-0" />
                <span>{t("qaPanel.ok")}</span>
              </div>
            ) : (
              <ul className="flex-1 space-y-1">
                {displayedQaResult.errors.map((err, i) => (
                  <li
                    key={i}
                    className="flex items-start gap-1.5 rounded bg-muted/30 px-2 py-1.5 text-xs"
                  >
                    {errorIcon(err)}
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
