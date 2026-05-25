import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { AlertTriangle, CheckCircle } from "lucide-react";
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

function errorLabel(error: QaErrorType): string {
  switch (error.type) {
    case "missing_placeholder":
      return `Placeholder manquant : ${error.placeholder}`;
    case "line_too_long":
      return `Ligne ${error.line} trop longue (${error.length} / ${error.max} chars max)`;
    case "bom_detected":
      return "BOM UTF-8 détecté en début de target";
  }
}

// ---------------------------------------------------------------------------
// Score badge
// ---------------------------------------------------------------------------

function ScoreBadge({ score }: { score: number }) {
  return (
    <span
      className={cn(
        "rounded px-1.5 py-0.5 text-[10px] font-semibold tabular-nums",
        score === 100
          ? "bg-green-500/20 text-green-400"
          : score >= 75
            ? "bg-yellow-500/20 text-yellow-400"
            : "bg-red-500/20 text-red-400",
      )}
    >
      {score}
    </span>
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
  const activeSegmentId = useActiveSegmentId();
  const activeProjectId = useProjectStore((s) => s.activeProjectId);

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
    staleTime: 0, // always fresh
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
      <div className="shrink-0 border-b px-3 py-2 flex items-center justify-between">
        <span className="text-xs font-medium text-muted-foreground select-none">
          QA
        </span>
        {qaReport && qaReport.totalSegments > 0 && (
          <span className="text-[10px] text-muted-foreground tabular-nums">
            {qaReport.okCount}/{qaReport.totalSegments} ok
          </span>
        )}
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        {!activeSegmentId && (
          <p className="py-4 text-center text-xs text-muted-foreground leading-relaxed">
            Sélectionnez un segment
            <br />
            pour voir les erreurs QA
          </p>
        )}

        {activeSegmentId && qaResult && (
          <>
            {/* Score */}
            <div className="mb-2 flex items-center justify-between">
              <span className="text-xs text-muted-foreground">Score</span>
              <ScoreBadge score={qaResult.score} />
            </div>

            {/* Error list */}
            {qaResult.errors.length === 0 ? (
              <div className="flex items-center gap-1.5 py-1 text-xs text-green-400">
                <CheckCircle className="h-3 w-3 shrink-0" />
                <span>Segment OK</span>
              </div>
            ) : (
              <ul className="space-y-1">
                {qaResult.errors.map((err, i) => (
                  <li
                    key={i}
                    className="flex items-start gap-1.5 rounded bg-muted/30 px-2 py-1.5 text-xs"
                  >
                    {errorIcon(err.type)}
                    <span className="leading-snug">{errorLabel(err)}</span>
                  </li>
                ))}
              </ul>
            )}
          </>
        )}
      </div>
    </div>
  );
}
