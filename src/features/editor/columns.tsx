import { createColumnHelper, type ColumnDef } from "@tanstack/react-table";
import { useLayoutEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import type { GlossaryTerm, Segment, SegmentStatus } from "@/lib/types";
import { useGlossaryTerms } from "@/stores/editor";
import { cn } from "@/lib/utils";
import { Play } from "lucide-react";

// ---------------------------------------------------------------------------
// Status badge
// ---------------------------------------------------------------------------

const STATUS_COLORS: Record<SegmentStatus, string> = {
  untranslated: "text-muted-foreground",
  translated: "text-blue-400",
  reviewed: "text-green-400",
  needs_review: "text-yellow-400",
};

function StatusBadge({ status }: { status: SegmentStatus }) {
  const { t } = useTranslation();
  return (
    <span className={cn("text-xs font-medium", STATUS_COLORS[status])}>
      {t(`segmentGrid.status.${status}`)}
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
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useLayoutEffect(() => {
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
      ref={textareaRef}
      id={`target-input-${rowIndex}`}
      className="w-full resize-none bg-transparent text-xs outline-none focus:bg-muted/30 rounded px-1 py-0.5 min-h-8"
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
// Source text highlight (placeholders + glossary terms)
// ---------------------------------------------------------------------------

const PH_RE = /\\[+\-]\w+\[\d+\]|\\[VNPCI]\[\d+\]|\\[G\\$.|!><^{}]|\[%\d+\]/g;

/** Escape special regex characters in a string. */
function escapeRe(s: string) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Split `text` by glossary term sources (sorted by length desc), then apply
 * placeholder highlight to each plain-text segment.
 * Returns a flat array of ReactNode ready to render inside a `<p>`.
 */
function buildHighlightedNodes(
  text: string,
  terms: GlossaryTerm[],
): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];

  function applyPlaceholders(chunk: string, keyBase: string) {
    const parts: React.ReactNode[] = [];
    let last = 0;
    let m: RegExpExecArray | null;
    PH_RE.lastIndex = 0;
    while ((m = PH_RE.exec(chunk)) !== null) {
      if (m.index > last) parts.push(chunk.slice(last, m.index));
      parts.push(
        <mark
          key={`${keyBase}-ph-${m.index}`}
          className="rounded bg-blue-500/20 px-0.5 text-blue-400 font-mono"
        >
          {m[0]}
        </mark>,
      );
      last = m.index + m[0].length;
    }
    if (last < chunk.length) parts.push(chunk.slice(last));
    return parts;
  }

  if (terms.length === 0) {
    return applyPlaceholders(text, "0");
  }

  // Sort terms by length descending to avoid shorter substrings matching first
  const sorted = [...terms].sort(
    (a, b) => b.sourceText.length - a.sourceText.length,
  );
  const pattern = sorted.map((t) => escapeRe(t.sourceText)).join("|");
  const glossaryRe = new RegExp(`(${pattern})`, "g");

  let last = 0;
  let m: RegExpExecArray | null;
  glossaryRe.lastIndex = 0;

  while ((m = glossaryRe.exec(text)) !== null) {
    if (m.index > last) {
      nodes.push(
        ...applyPlaceholders(text.slice(last, m.index), `pre-${m.index}`),
      );
    }
    nodes.push(
      <mark
        key={`g-${m.index}`}
        className="rounded-sm bg-green-200 dark:bg-green-900 px-0.5"
      >
        {m[0]}
      </mark>,
    );
    last = glossaryRe.lastIndex;
  }
  if (last < text.length) {
    nodes.push(...applyPlaceholders(text.slice(last), `tail`));
  }

  return nodes;
}

function SourceCell({ text }: { text: string }) {
  const terms = useGlossaryTerms();
  const nodes = buildHighlightedNodes(text, terms);
  return <p className="text-xs leading-relaxed whitespace-pre-wrap">{nodes}</p>;
}

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
          className="h-3.5 w-3.5 cursor-pointer accent-primary"
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
          className="h-3.5 w-3.5 cursor-pointer accent-primary"
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
          className="flex h-6 w-6 items-center justify-center rounded opacity-0 group-hover:opacity-100 hover:bg-primary/20 text-muted-foreground hover:text-primary transition-all"
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
