import { createColumnHelper, type ColumnDef } from "@tanstack/react-table";
import { memo, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { Segment, SegmentStatus } from "@/lib/types";
import { useActiveEngine } from "@/stores/project";
import { cn } from "@/lib/utils";
import { getPlaceholderRegex } from "@/lib/constants";
import { buildHighlightedNodes } from "@/lib/highlight-utils";
import { Play } from "lucide-react";

// ---------------------------------------------------------------------------
// Status badge
// ---------------------------------------------------------------------------

export const STATUS_STYLES: Record<
  SegmentStatus,
  { label: string; dot: string }
> = {
  untranslated: {
    label: "text-muted-foreground/70",
    dot: "bg-muted-foreground/50",
  },
  translated: {
    label: "text-cyan-600 dark:text-cyan-300",
    dot: "bg-cyan-500 dark:bg-cyan-300 shadow-[0_0_5px_currentColor]",
  },
  reviewed: {
    label: "text-star",
    dot: "rotate-45 rounded-[1px] bg-star shadow-[0_0_5px_var(--star)]",
  },
  needs_review: {
    label: "text-amber-600 dark:text-amber-300",
    dot: "bg-amber-500 dark:bg-amber-300 shadow-[0_0_5px_currentColor]",
  },
};

export const StatusBadge = memo(function StatusBadge({
  status,
}: {
  status: SegmentStatus;
}) {
  const { t } = useTranslation();
  const style = STATUS_STYLES[status];
  return (
    <span
      className={cn(
        "inline-flex min-w-0 max-w-full items-center gap-1.5 text-xs font-medium",
        style.label,
      )}
    >
      <span className={cn("h-1.5 w-1.5 shrink-0 rounded-full", style.dot)} />
      <span className="truncate">{t(`segmentGrid.status.${status}`)}</span>
    </span>
  );
});

// ---------------------------------------------------------------------------
// Editable target cell
// ---------------------------------------------------------------------------

interface EditableCellProps {
  segmentId: string;
  initialValue: string;
  rowIndex: number;
  totalRows: number;
  onSave: (id: string, text: string) => Promise<void>;
  onTabNext: (currentIndex: number) => void;
  ariaLabel: string;
}

const EditableCell = memo(function EditableCell({
  segmentId,
  initialValue,
  rowIndex,
  totalRows,
  onSave,
  onTabNext,
  ariaLabel,
}: EditableCellProps) {
  const [value, setValue] = useState(initialValue);
  const savedRef = useRef(initialValue);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    const el = textareaRef.current;
    if (!el) return;
    el.style.height = "auto";
    el.style.height = `${el.scrollHeight}px`;
  }, [value]);

  // Sync when the row data changes externally (e.g., after a save round-trip)
  if (
    savedRef.current !== initialValue &&
    document.activeElement?.id !== `target-input-${rowIndex}`
  ) {
    savedRef.current = initialValue;
    setValue(initialValue);
  }

  function handleBlur() {
    if (value !== savedRef.current) {
      savedRef.current = value;
      void onSave(segmentId, value);
    }
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLTextAreaElement>) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      if (value !== savedRef.current) {
        savedRef.current = value;
        void onSave(segmentId, value);
      }
    }

    if (e.key === "Tab") {
      e.preventDefault();
      if (value !== savedRef.current) {
        savedRef.current = value;
        void onSave(segmentId, value);
      }
      if (rowIndex < totalRows - 1) {
        onTabNext(rowIndex);
      }
    }
  }

  return (
    <textarea
      ref={textareaRef}
      id={`target-input-${rowIndex}`}
      className="min-h-10 w-full resize-none rounded bg-transparent px-1 py-1 text-xs outline-none focus:bg-muted/30"
      aria-label={ariaLabel}
      value={value}
      rows={1}
      onChange={(e) => setValue(e.target.value)}
      onBlur={handleBlur}
      onKeyDown={handleKeyDown}
      spellCheck={false}
    />
  );
});

// ---------------------------------------------------------------------------
// Source text highlight (engine placeholders only)
// ---------------------------------------------------------------------------

const SourceCell = memo(function SourceCell({ text }: { text: string }) {
  const engine = useActiveEngine();
  const nodes = useMemo(
    () => buildHighlightedNodes(text, [], getPlaceholderRegex(engine)),
    [text, engine],
  );
  return (
    <p className="text-xs leading-relaxed whitespace-pre-wrap break-words min-w-0 max-w-full">
      {nodes}
    </p>
  );
});

// ---------------------------------------------------------------------------
// Column factory
// ---------------------------------------------------------------------------

const helper = createColumnHelper<Segment>();

export interface SegmentColumnMeta {
  totalRows: number;
  onSave: (id: string, text: string) => Promise<void>;
  onTabNext: (currentIndex: number) => void;
  onTranslate: (segmentId: string) => void;
  t: (key: string) => string;
}

export function createSegmentColumns(
  meta: SegmentColumnMeta,
): ColumnDef<Segment>[] {
  return [
    // Checkbox selection column
    helper.display({
      id: "select",
      size: 36,
      header: ({ table }) => (
        <input
          type="checkbox"
          className="size-6 cursor-pointer accent-primary"
          aria-label={meta.t("segmentGrid.selectAll")}
          checked={table.getIsAllPageRowsSelected()}
          ref={(el) => {
            if (el) el.indeterminate = table.getIsSomePageRowsSelected();
          }}
          onChange={table.getToggleAllPageRowsSelectedHandler()}
          onClick={(e) => e.stopPropagation()}
        />
      ),
      cell: ({ row }) => (
        <input
          type="checkbox"
          className="size-6 cursor-pointer accent-primary"
          aria-label={`${meta.t("segmentGrid.selectRow")} ${row.index + 1}`}
          checked={row.getIsSelected()}
          onChange={row.getToggleSelectedHandler()}
          onClick={(e) => e.stopPropagation()}
        />
      ),
    }) as ColumnDef<Segment>,

    helper.display({
      id: "index",
      header: meta.t("segmentGrid.columns.number"),
      size: 56,
      cell: (ctx) => (
        <span className="text-xs text-muted-foreground tabular-nums select-none">
          {ctx.row.index + 1}
        </span>
      ),
    }) as ColumnDef<Segment>,

    helper.accessor("sourceText", {
      header: meta.t("segmentGrid.columns.source"),
      size: 0, // flex
      cell: (ctx) => <SourceCell text={ctx.getValue()} />,
    }) as ColumnDef<Segment>,

    helper.accessor("targetText", {
      header: meta.t("segmentGrid.columns.target"),
      size: 0, // flex
      cell: (ctx) => (
        <EditableCell
          segmentId={ctx.row.original.id}
          initialValue={ctx.getValue()}
          rowIndex={ctx.row.index}
          totalRows={meta.totalRows}
          onSave={meta.onSave}
          onTabNext={meta.onTabNext}
          ariaLabel={`${meta.t("segmentGrid.targetInput")} ${ctx.row.index + 1}`}
        />
      ),
    }) as ColumnDef<Segment>,

    helper.accessor("status", {
      header: meta.t("segmentGrid.columns.status"),
      size: 100,
      cell: (ctx) => <StatusBadge status={ctx.getValue()} />,
    }) as ColumnDef<Segment>,

    helper.accessor("qaScore", {
      header: meta.t("segmentGrid.columns.qa"),
      size: 52,
      cell: (ctx) => {
        const score = ctx.getValue();
        if (score === null)
          return <span className="text-xs text-muted-foreground">—</span>;
        return (
          <span
            className={cn(
              "text-xs tabular-nums font-medium",
              score >= 80
                ? "text-green-400"
                : score >= 50
                  ? "text-yellow-400"
                  : "text-red-400",
            )}
          >
            {score}
          </span>
        );
      },
    }) as ColumnDef<Segment>,

    // Per-row translate button
    helper.display({
      id: "actions",
      size: 36,
      header: () => null,
      cell: (ctx) => (
        <button
          type="button"
          title={meta.t("segmentGrid.translateRow")}
          className="hit-area-40 relative flex h-6 w-6 items-center justify-center rounded opacity-0 text-muted-foreground transition-[background-color,color,opacity] group-hover:opacity-100 hover:bg-primary/20 hover:text-primary focus-visible:opacity-100"
          aria-label={meta.t("segmentGrid.translateRow")}
          onClick={(e) => {
            e.stopPropagation();
            meta.onTranslate(ctx.row.original.id);
          }}
        >
          <Play className="h-3 w-3" />
        </button>
      ),
    }) as ColumnDef<Segment>,
  ];
}
