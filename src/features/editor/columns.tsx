import { createColumnHelper, type ColumnDef } from "@tanstack/react-table";
import { useRef, useState } from "react";
import type { Segment, SegmentStatus } from "@/lib/types";
import { cn } from "@/lib/utils";

// ---------------------------------------------------------------------------
// Status badge
// ---------------------------------------------------------------------------

const STATUS_LABELS: Record<SegmentStatus, string> = {
  untranslated: "Non traduit",
  translated: "Traduit",
  reviewed: "Relu",
  needs_review: "À relire",
};

const STATUS_COLORS: Record<SegmentStatus, string> = {
  untranslated: "text-muted-foreground",
  translated: "text-blue-400",
  reviewed: "text-green-400",
  needs_review: "text-yellow-400",
};

function StatusBadge({ status }: { status: SegmentStatus }) {
  return (
    <span className={cn("text-xs font-medium", STATUS_COLORS[status])}>
      {STATUS_LABELS[status]}
    </span>
  );
}

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
}

function EditableCell({
  segmentId,
  initialValue,
  rowIndex,
  totalRows,
  onSave,
  onTabNext,
}: EditableCellProps) {
  const [value, setValue] = useState(initialValue);
  const savedRef = useRef(initialValue);

  // Sync when the row data changes externally (e.g., after a save round-trip)
  if (
    savedRef.current !== initialValue &&
    document.activeElement?.id !== `target-input-${rowIndex}`
  ) {
    savedRef.current = initialValue;
    // eslint-disable-next-line react-compiler/react-compiler -- controlled sync
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
      id={`target-input-${rowIndex}`}
      className="w-full resize-none bg-transparent text-xs outline-none focus:bg-muted/30 rounded px-1 py-0.5 min-h-[2rem]"
      value={value}
      rows={1}
      onChange={(e) => setValue(e.target.value)}
      onBlur={handleBlur}
      onKeyDown={handleKeyDown}
      spellCheck={false}
    />
  );
}

// ---------------------------------------------------------------------------
// Column factory
// ---------------------------------------------------------------------------

const helper = createColumnHelper<Segment>();

export interface SegmentColumnMeta {
  totalRows: number;
  onSave: (id: string, text: string) => Promise<void>;
  onTabNext: (currentIndex: number) => void;
}

export function createSegmentColumns(
  meta: SegmentColumnMeta,
): ColumnDef<Segment>[] {
  return [
    helper.display({
      id: "index",
      header: "#",
      size: 56,
      cell: (ctx) => (
        <span className="text-xs text-muted-foreground tabular-nums select-none">
          {ctx.row.index + 1}
        </span>
      ),
    }) as ColumnDef<Segment>,

    helper.accessor("sourceText", {
      header: "Source",
      size: 0, // flex
      cell: (ctx) => (
        <p className="text-xs leading-relaxed whitespace-pre-wrap">
          {ctx.getValue()}
        </p>
      ),
    }) as ColumnDef<Segment>,

    helper.accessor("targetText", {
      header: "Cible",
      size: 0, // flex
      cell: (ctx) => (
        <EditableCell
          segmentId={ctx.row.original.id}
          initialValue={ctx.getValue()}
          rowIndex={ctx.row.index}
          totalRows={meta.totalRows}
          onSave={meta.onSave}
          onTabNext={meta.onTabNext}
        />
      ),
    }) as ColumnDef<Segment>,

    helper.accessor("status", {
      header: "Statut",
      size: 100,
      cell: (ctx) => <StatusBadge status={ctx.getValue()} />,
    }) as ColumnDef<Segment>,

    helper.accessor("qaScore", {
      header: "QA",
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
  ];
}
