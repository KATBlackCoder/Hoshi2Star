import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  SegmentSearchBar,
  type QaFilter,
} from "@/components/editor/SegmentSearchBar";
import {
  useReactTable,
  getCoreRowModel,
  flexRender,
  type RowSelectionState,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { useLlmStore, useIsTranslating } from "@/stores/llm";
import { createSegmentColumns, STATUS_STYLES } from "@/features/editor/columns";
import type {
  GlossaryTerm,
  PaginatedSegments,
  Segment,
  SegmentUpdate,
  SourceFile,
} from "@/lib/types";
import { cn } from "@/lib/utils";
import { Play, RefreshCw } from "lucide-react";
import { Button } from "@/components/ui/button";
import { toast } from "sonner";

interface SegmentGridProps {
  highlightPlaceholders?: boolean;
}

export function SegmentGrid({
  highlightPlaceholders = false,
}: SegmentGridProps) {
  void highlightPlaceholders; // consumed by columns in future — prop reserved for F2
  const { t, i18n } = useTranslation();
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const setSourceFiles = useProjectStore((s) => s.setSourceFiles);
  const activeFileId = useEditorStore((s) => s.activeFileId);
  const setActiveSegment = useEditorStore((s) => s.setActiveSegment);
  const activeSegmentId = useEditorStore((s) => s.activeSegmentId);
  const setGlossaryTerms = useEditorStore((s) => s.setGlossaryTerms);
  const { startTranslation, providerConfig } = useLlmStore();
  const isTranslating = useIsTranslating();

  const activeSegmentIdRef = useRef(activeSegmentId);
  activeSegmentIdRef.current = activeSegmentId;

  const [segments, setSegments] = useState<Segment[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [qaFilter, setQaFilter] = useState<QaFilter>("all");
  const [searchQuery, setSearchQuery] = useState("");
  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});

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

  const reloadSourceFiles = useCallback(
    (projectId: string) => {
      invoke<SourceFile[]>("get_source_files", { projectId })
        .then(setSourceFiles)
        .catch(() => {});
    },
    [setSourceFiles],
  );

  useEffect(() => {
    if (!activeProjectId || !activeFileId) {
      setSegments([]);
      return;
    }
    loadSegments(activeProjectId, activeFileId);
  }, [activeProjectId, activeFileId, loadSegments]);

  // Reset filter + selection when switching files
  useEffect(() => {
    setQaFilter("all");
    setSearchQuery("");
    setRowSelection({});
  }, [activeFileId]);

  const filteredSegments = useMemo(() => {
    let result = segments;

    switch (qaFilter) {
      case "errors":
        result = result.filter(
          (s) =>
            s.qaScore !== null && s.qaScore !== undefined && s.qaScore < 100,
        );
        break;
      case "critical":
        result = result.filter(
          (s) =>
            s.qaScore !== null && s.qaScore !== undefined && s.qaScore < 70,
        );
        break;
      case "untranslated":
        result = result.filter((s) => s.status === "untranslated");
        break;
      case "needs_review":
        result = result.filter((s) => s.status === "needs_review");
        break;
    }

    if (searchQuery.trim() !== "") {
      const q = searchQuery.toLowerCase();
      result = result.filter(
        (s) =>
          s.sourceText.toLowerCase().includes(q) ||
          s.targetText.toLowerCase().includes(q),
      );
    }

    return result;
  }, [segments, qaFilter, searchQuery]);

  const statusCounts = useMemo(() => {
    const counts: Record<Segment["status"], number> = {
      untranslated: 0,
      translated: 0,
      reviewed: 0,
      needs_review: 0,
    };
    for (const s of segments) counts[s.status]++;
    return counts;
  }, [segments]);

  // Patch translated segments into the in-memory list as each batch is
  // persisted, so the table updates progressively during long translations
  // instead of waiting for the whole project to finish.
  useEffect(() => {
    const unlisten = listen<SegmentUpdate[]>(
      "h2s://llm/segments-updated",
      (event) => {
        const updates = new Map(event.payload.map((u) => [u.id, u]));
        if (updates.size === 0) return;
        setSegments((prev) =>
          prev.map((s) => {
            const u = updates.get(s.id);
            return u ? { ...s, targetText: u.targetText, status: u.status } : s;
          }),
        );
      },
    );
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  // Re-fetch segments + source files when LLM pipeline completes
  useEffect(() => {
    const unlisten = listen("h2s://llm/completed", () => {
      const pid = activeProjectIdRef.current;
      const fid = activeFileIdRef.current;
      if (pid && fid) loadSegments(pid, fid);
      if (pid) reloadSourceFiles(pid);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [loadSegments, reloadSourceFiles]);

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

  const handleSave = useCallback(
    async (id: string, text: string) => {
      const updated = await invoke<Segment>("update_segment", {
        id,
        targetText: text,
      });
      setSegments((prev) =>
        prev.map((s) => (s.id === updated.id ? updated : s)),
      );
      if (id === activeSegmentIdRef.current) {
        setActiveSegment(id, updated.sourceText, updated.targetText);
      }
    },
    [setActiveSegment],
  );

  // Translate a single segment directly (no config modal — uses current providerConfig)
  const handleTranslate = useCallback(
    (segmentId: string) => {
      if (!providerConfig.model.trim()) {
        toast.error(t("segmentGrid.noModelConfigured"));
        return;
      }
      void startTranslation([segmentId], undefined);
    },
    [startTranslation, providerConfig.model, t],
  );

  const needsReviewIds = useMemo(
    () => segments.filter((s) => s.status === "needs_review").map((s) => s.id),
    [segments],
  );

  // Translate selected rows
  const selectedIds = useMemo(
    () =>
      Object.keys(rowSelection)
        .map((idx) => filteredSegments[Number(idx)]?.id)
        .filter(Boolean),
    [rowSelection, filteredSegments],
  );

  function handleTranslateSelected() {
    if (!providerConfig.model.trim()) {
      toast.error(t("segmentGrid.noModelConfigured"));
      return;
    }
    void startTranslation(selectedIds, undefined);
    setRowSelection({});
  }

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
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [filteredSegments.length],
  );

  const columns = useMemo(
    () =>
      createSegmentColumns({
        totalRows: filteredSegments.length,
        onSave: handleSave,
        onTabNext: handleTabNext,
        onTranslate: handleTranslate,
        t,
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [
      filteredSegments.length,
      handleSave,
      handleTabNext,
      handleTranslate,
      i18n.language,
    ],
  );

  const table = useReactTable({
    data: filteredSegments,
    columns,
    getCoreRowModel: getCoreRowModel(),
    enableRowSelection: true,
    state: { rowSelection },
    onRowSelectionChange: setRowSelection,
  });

  const rows = table.getRowModel().rows;

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 40,
    overscan: 5,
  });

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
      <div className="flex shrink-0 border-b bg-muted/30 text-[10px] font-semibold uppercase tracking-[0.12em] text-muted-foreground/80">
        {table.getHeaderGroups().map((hg) =>
          hg.headers.map((header) => (
            <div
              key={header.id}
              className={cn(
                "flex items-center px-3 py-2 select-none",
                header.id === "select" && "w-9 shrink-0 justify-center",
                header.id === "index" && "w-14 shrink-0",
                header.id === "sourceText" && "flex-1 min-w-0",
                header.id === "targetText" && "flex-1 min-w-0",
                header.id === "status" && "w-24 shrink-0",
                header.id === "qaScore" && "w-14 shrink-0",
                header.id === "actions" && "w-9 shrink-0",
              )}
            >
              {flexRender(header.column.columnDef.header, header.getContext())}
            </div>
          )),
        )}
      </div>

      {/* Search bar + QA filter */}
      <SegmentSearchBar
        searchQuery={searchQuery}
        onSearchChange={setSearchQuery}
        qaFilter={qaFilter}
        onQaFilterChange={setQaFilter}
        shownCount={filteredSegments.length}
        totalCount={segments.length}
      />

      {/* Batch action toolbar */}
      {(needsReviewIds.length > 0 || selectedIds.length >= 2) && (
        <div className="shrink-0 border-b px-3 py-1.5 flex items-center gap-2">
          {needsReviewIds.length > 0 && (
            <Button
              size="sm"
              variant="outline"
              className="h-7 gap-1.5 text-xs text-yellow-400 border-yellow-400/40 hover:bg-yellow-400/10"
              disabled={isTranslating}
              onClick={() => {
                if (!providerConfig.model.trim()) {
                  toast.error(t("segmentGrid.noModelConfigured"));
                  return;
                }
                void startTranslation(needsReviewIds, undefined);
              }}
            >
              <RefreshCw className="h-3 w-3" />
              {t("segmentGrid.retranslateNeedsReview", {
                count: needsReviewIds.length,
              })}
            </Button>
          )}

          {selectedIds.length >= 2 && (
            <Button
              size="sm"
              variant="outline"
              className="h-7 gap-1.5 text-xs"
              disabled={isTranslating}
              onClick={handleTranslateSelected}
            >
              <Play className="h-3 w-3" />
              {t("segmentGrid.translateSelected", {
                count: selectedIds.length,
              })}
            </Button>
          )}
        </div>
      )}

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
                  "group flex border-b border-border/50 hover:bg-accent/30 transition-colors cursor-pointer",
                  activeSegmentId === row.original.id && "bg-accent/50",
                )}
              >
                {row.getVisibleCells().map((cell) => (
                  <div
                    key={cell.id}
                    className={cn(
                      "flex items-start px-3 py-2 min-h-10",
                      cell.column.id === "select" &&
                        "w-9 shrink-0 justify-center items-center",
                      cell.column.id === "index" && "w-14 shrink-0 justify-end",
                      cell.column.id === "sourceText" && "flex-1 min-w-0",
                      cell.column.id === "targetText" && "flex-1 min-w-0",
                      cell.column.id === "status" &&
                        "w-24 shrink-0 items-center",
                      cell.column.id === "qaScore" &&
                        "w-14 shrink-0 justify-center items-center",
                      cell.column.id === "actions" &&
                        "w-9 shrink-0 justify-center items-center",
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

      {/* Footer: segment count (filtered / total) + status recap */}
      <div className="shrink-0 border-t px-3 py-1.5 flex items-center gap-4 text-xs text-muted-foreground">
        <span>
          {qaFilter === "all"
            ? t("segmentGrid.footer", {
                count: segments.length.toLocaleString(),
              })
            : t("segmentGrid.footerFiltered", {
                shown: filteredSegments.length.toLocaleString(),
                total: segments.length.toLocaleString(),
              })}
        </span>
        {segments.length > 0 && (
          <div className="flex items-center gap-3">
            {(
              [
                "translated",
                "reviewed",
                "needs_review",
                "untranslated",
              ] as const
            ).map(
              (status) =>
                statusCounts[status] > 0 && (
                  <span
                    key={status}
                    className={cn(
                      "inline-flex items-center gap-1.5 font-medium",
                      STATUS_STYLES[status].label,
                    )}
                  >
                    <span
                      className={cn(
                        "h-1.5 w-1.5 shrink-0 rounded-full",
                        STATUS_STYLES[status].dot,
                      )}
                    />
                    {statusCounts[status].toLocaleString()}{" "}
                    {t(`segmentGrid.status.${status}`)}
                  </span>
                ),
            )}
          </div>
        )}
      </div>
    </div>
  );
}
