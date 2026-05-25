import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { BookOpen } from "lucide-react";
import {
  useActiveSegmentId,
  useActiveSegmentSourceText,
} from "@/stores/editor";
import type { TmEntry } from "@/lib/types";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function ConfidenceBadge({ value }: { value: number }) {
  const pct = Math.round(value * 100);
  return (
    <span
      className={cn(
        "shrink-0 rounded px-1.5 py-0.5 text-[10px] font-semibold tabular-nums",
        pct >= 90
          ? "bg-green-500/20 text-green-400"
          : pct >= 70
            ? "bg-yellow-500/20 text-yellow-400"
            : "bg-red-500/20 text-red-400",
      )}
    >
      {pct}%
    </span>
  );
}

// ---------------------------------------------------------------------------
// TMPanel
// ---------------------------------------------------------------------------

interface TMPanelProps {
  /** Called when user clicks a suggestion to apply it to the active segment. */
  onApply?: (targetText: string) => void;
}

export function TMPanel({ onApply }: TMPanelProps) {
  const { t } = useTranslation();
  const activeSegmentId = useActiveSegmentId();
  const sourceText = useActiveSegmentSourceText();

  const { data: suggestions = [], isLoading } = useQuery<TmEntry[]>({
    queryKey: ["tm-suggestions", sourceText],
    queryFn: () =>
      invoke<TmEntry[]>("get_tm_suggestions", {
        sourceText: sourceText!,
        langPair: "ja-en",
      }),
    enabled: !!sourceText,
    staleTime: 1000 * 60, // TM changes rarely during a session
  });

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Header */}
      <div className="shrink-0 border-b px-3 py-2 text-xs font-medium text-muted-foreground select-none flex items-center gap-1.5">
        <BookOpen className="h-3 w-3" />
        {t("tmPanel.title")}
      </div>

      <div className="flex-1 overflow-y-auto p-2">
        {!activeSegmentId && (
          <p className="py-4 text-center text-xs text-muted-foreground leading-relaxed">
            {t("tmPanel.empty")}
          </p>
        )}

        {activeSegmentId && isLoading && (
          <p className="py-4 text-center text-xs text-muted-foreground">
            {t("tmPanel.searching")}
          </p>
        )}

        {activeSegmentId && !isLoading && suggestions.length === 0 && (
          <p className="py-4 text-center text-xs text-muted-foreground leading-relaxed">
            {t("tmPanel.noMatch")}
          </p>
        )}

        {suggestions.map((entry) => (
          <button
            key={entry.id}
            type="button"
            onClick={() => onApply?.(entry.targetText)}
            className="mb-1.5 w-full rounded border border-border/50 bg-muted/20 p-2 text-left hover:bg-accent/40 transition-colors"
          >
            <div className="mb-1 flex items-center justify-between gap-2">
              <span className="truncate text-[10px] text-muted-foreground">
                {entry.sourceText}
              </span>
              <ConfidenceBadge value={entry.confidence} />
            </div>
            <p className="text-xs leading-relaxed">{entry.targetText}</p>
          </button>
        ))}
      </div>
    </div>
  );
}
