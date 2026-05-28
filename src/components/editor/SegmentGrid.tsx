import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useTranslation } from "react-i18next";
import {
  useReactTable,
  getCoreRowModel,
  flexRender,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { createSegmentColumns } from "@/features/editor/columns";
import type { GlossaryTerm, PaginatedSegments, Segment } from "@/lib/types";
import { cn } from "@/lib/utils";

interface SegmentGridProps {
  highlightPlaceholders?: boolean;
}

export function SegmentGrid({
  highlightPlaceholders = false,
}: SegmentGridProps) {
  void highlightPlaceholders; // consumed by columns in future — prop reserved for F2
  const { t, i18n } = useTranslation();
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const activeFileId = useEditorStore((s) => s.activeFileId);
  const setActiveSegment = useEditorStore((s) => s.setActiveSegment);
  const activeSegmentId = useEditorStore((s) => s.activeSegmentId);
  const setGlossaryTerms = useEditorStore((s) => s.setGlossaryTerms);

  // Ref so handleSave (useCallback) can read the active id without re-registering
  const activeSegmentIdRef = useRef(activeSegmentId);
  activeSegmentIdRef.current = activeSegmentId;

  const [segments, setSegments] = useState<Segment[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [qaFilter, setQaFilter] = useState<
    "all" | "errors" | "critical" | "untranslated" | "needs_review"
  >("all");

  // Keep latest ids in a ref so the completed-listener can use them without
  // being re-registered on every render.
  const activeProjectIdRef = useRef(activeProjectId);
  const activeFileIdRef = useRef(activeFileId);
  activeProjectIdRef.current = activeProjectId;
  activeFileIdRef.current = activeFileId;

  const loadSegments = useCallback((projectId: string, fileId: string) => {
    setIsLoading(true);
    invoke<PaginatedSegments>("get_segments", {
      projectId,
      fileId,
      page: 0,
      pageSize: 5000,
    })
      .then((result) => setSegments(result.items))
      .catch(() => setSegments([]))
      .finally(() => setIsLoading(false));
  }, []);

  // Fetch segments whenever the active file changes
  useEffect(() => {
    if (!activeProjectId || !activeFileId) {
      setSegments([]);
      return;
    }
    loadSegments(activeProjectId, activeFileId);
  }, [activeProjectId, activeFileId, loadSegments]);

  // Reset filter when switching files
  useEffect(() => {
    setQaFilter("all");
  }, [activeFileId]);

  const filteredSegments = useMemo(() => {
    switch (qaFilter) {
      case "errors":
        return segments.filter(
          (s) =>
            s.qaScore !== null && s.qaScore !== undefined && s.qaScore < 100,
        );
      case "critical":
        return segments.filter(
          (s) =>
            s.qaScore !== null && s.qaScore !== undefined && s.qaScore < 70,
        );
      case "untranslated":
        return segments.filter((s) => s.status === "untranslated");
      case "needs_review":
        return segments.filter((s) => s.status === "needs_review");
      default:
        return segments;
    }
  }, [segments, qaFilter]);

  // Re-fetch when LLM pipeline completes so translated segments appear immediately
  useEffect(() => {
    const unlisten = listen("h2s://llm/completed", () => {
      const pid = activeProjectIdRef.current;
      const fid = activeFileIdRef.current;
      if (pid && fid) loadSegments(pid, fid);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [loadSegments]);

  // Load glossary terms when the active project changes
  useEffect(() => {
    if (!activeProjectId) {
      setGlossaryTerms([]);
      return;
    }
    invoke<GlossaryTerm[]>("get_glossary", {
      projectId: activeProjectId,
      langPair: "ja-en",
    })
      .then(setGlossaryTerms)
      .catch(() => setGlossaryTerms([]));
  }, [activeProjectId, setGlossaryTerms]);

  // Refresh glossary terms after auto-extraction completes
  useEffect(() => {
    const unlisten = listen<{ projectId: string; terms: GlossaryTerm[] }>(
      "h2s://glossary/extraction-done",
      (event) => {
        if (event.payload.projectId === activeProjectIdRef.current) {
          invoke<GlossaryTerm[]>("get_glossary", {
            projectId: event.payload.projectId,
            langPair: "ja-en",
          })
            .then(setGlossaryTerms)
            .catch(() => {});
        }
      },
    );
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [setGlossaryTerms]);

  // Save a segment translation and update local state
  const handleSave = useCallback(
    async (id: string, text: string) => {
      const updated = await invoke<Segment>("update_segment", {
        id,
        targetText: text,
      });
      setSegments((prev) =>
        prev.map((s) => (s.id === updated.id ? updated : s)),
      );
      // Keep QA panel fresh if the saved segment is currently active
      if (id === activeSegmentIdRef.current) {
        setActiveSegment(id, updated.sourceText, updated.targetText);
      }
    },
    [setActiveSegment],
  );

  // Tab → scroll to + focus next row
  const parentRef = useRef<HTMLDivElement>(null);

  const handleTabNext = useCallback(
    (currentIndex: number) => {
      const nextIndex = currentIndex + 1;
      if (nextIndex >= filteredSegments.length) return;
      virtualizer.scrollToIndex(nextIndex, { align: "auto" });
      requestAnimationFrame(() => {
        const nextInput = document.getElementById(`target-input-${nextIndex}`);
        nextInput?.focus();
      });
    },
    // virtualizer is stable by ref — safe to omit from deps
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [filteredSegments.length],
  );

  const columns = useMemo(
    () =>
      createSegmentColumns({
        totalRows: filteredSegments.length,
        onSave: handleSave,
        onTabNext: handleTabNext,
        t,
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [filteredSegments.length, handleSave, handleTabNext, i18n.language],
  );

  const table = useReactTable({
    data: filteredSegments,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  const rows = table.getRowModel().rows;

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 40,
    overscan: 10,
  });

  // Empty states
  if (!activeFileId) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">
          {t("segmentGrid.empty")}
        </p>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">
          {t("segmentGrid.loading")}
        </p>
      </div>
    );
  }

  if (segments.length === 0) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">
          {t("segmentGrid.noSegments")}
        </p>
      </div>
    );
  }

  const virtualItems = virtualizer.getVirtualItems();

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Header */}
      <div className="flex shrink-0 border-b bg-muted/30 text-xs font-medium text-muted-foreground">
        {table.getHeaderGroups().map((hg) =>
          hg.headers.map((header) => (
            <div
              key={header.id}
              className={cn(
                "flex items-center px-3 py-2 select-none",
                header.id === "index" && "w-14 shrink-0",
                header.id === "sourceText" && "flex-1 min-w-0",
                header.id === "targetText" && "flex-1 min-w-0",
                header.id === "status" && "w-24 shrink-0",
                header.id === "qaScore" && "w-14 shrink-0",
              )}
            >
              {flexRender(header.column.columnDef.header, header.getContext())}
            </div>
          )),
        )}
      </div>

      {/* Toolbar: QA filter */}
      <div className="shrink-0 border-b px-3 py-1.5 flex items-center gap-2">
        <Select
          value={qaFilter}
          onValueChange={(v) =>
            setQaFilter(
              v as
                | "all"
                | "errors"
                | "critical"
                | "untranslated"
                | "needs_review",
            )
          }
        >
          <SelectTrigger className="h-7 w-48 text-xs">
            <SelectValue />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">{t("segmentGrid.filterAll")}</SelectItem>
            <SelectItem value="errors">
              {t("segmentGrid.filterQaErrors")}
            </SelectItem>
            <SelectItem value="critical">
              {t("segmentGrid.filterQaCritical")}
            </SelectItem>
            <SelectItem value="untranslated">
              {t("segmentGrid.filterUntranslated")}
            </SelectItem>
            <SelectItem value="needs_review">
              {t("segmentGrid.filterNeedsReview")}
            </SelectItem>
          </SelectContent>
        </Select>
      </div>

      {/* Virtual body */}
      <div ref={parentRef} className="flex-1 overflow-auto">
        <div
          style={{ height: virtualizer.getTotalSize(), position: "relative" }}
        >
          {virtualItems.map((virtualRow) => {
            const row = rows[virtualRow.index];
            return (
              <div
                key={row.id}
                data-index={virtualRow.index}
                ref={virtualizer.measureElement}
                style={{
                  position: "absolute",
                  top: 0,
                  left: 0,
                  right: 0,
                  transform: `translateY(${virtualRow.start}px)`,
                }}
                onClick={() =>
                  setActiveSegment(
                    row.original.id,
                    row.original.sourceText,
                    row.original.targetText,
                  )
                }
                className={cn(
                  "flex border-b border-border/50 hover:bg-accent/30 transition-colors cursor-pointer",
                  activeSegmentId === row.original.id && "bg-accent/50",
                )}
              >
                {row.getVisibleCells().map((cell) => (
                  <div
                    key={cell.id}
                    className={cn(
                      "flex items-start px-3 py-2 min-h-10",
                      cell.column.id === "index" && "w-14 shrink-0 justify-end",
                      cell.column.id === "sourceText" && "flex-1 min-w-0",
                      cell.column.id === "targetText" && "flex-1 min-w-0",
                      cell.column.id === "status" &&
                        "w-24 shrink-0 items-center",
                      cell.column.id === "qaScore" &&
                        "w-14 shrink-0 justify-center items-center",
                    )}
                  >
                    {flexRender(cell.column.columnDef.cell, cell.getContext())}
                  </div>
                ))}
              </div>
            );
          })}
        </div>
      </div>

      {/* Footer: segment count (filtered / total) */}
      <div className="shrink-0 border-t px-3 py-1.5 text-xs text-muted-foreground">
        {qaFilter === "all"
          ? t("segmentGrid.footer", {
              count: segments.length.toLocaleString(),
            })
          : t("segmentGrid.footerFiltered", {
              shown: filteredSegments.length.toLocaleString(),
              total: segments.length.toLocaleString(),
            })}
      </div>
    </div>
  );
}
