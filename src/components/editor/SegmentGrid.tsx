import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  useReactTable,
  getCoreRowModel,
  flexRender,
} from "@tanstack/react-table";
import { useVirtualizer } from "@tanstack/react-virtual";
import { invoke } from "@tauri-apps/api/core";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { createSegmentColumns } from "@/features/editor/columns";
import type { PaginatedSegments, Segment } from "@/lib/types";
import { cn } from "@/lib/utils";

export function SegmentGrid() {
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const activeFileId = useEditorStore((s) => s.activeFileId);
  const setActiveSegment = useEditorStore((s) => s.setActiveSegment);
  const activeSegmentId = useEditorStore((s) => s.activeSegmentId);

  const [segments, setSegments] = useState<Segment[]>([]);
  const [isLoading, setIsLoading] = useState(false);

  // Fetch segments whenever the active file changes
  useEffect(() => {
    if (!activeProjectId || !activeFileId) {
      setSegments([]);
      return;
    }
    setIsLoading(true);
    invoke<PaginatedSegments>("get_segments", {
      projectId: activeProjectId,
      fileId: activeFileId,
      page: 0,
      pageSize: 5000,
    })
      .then((result) => setSegments(result.items))
      .catch(() => setSegments([]))
      .finally(() => setIsLoading(false));
  }, [activeProjectId, activeFileId]);

  // Save a segment translation and update local state
  const handleSave = useCallback(async (id: string, text: string) => {
    const updated = await invoke<Segment>("update_segment", {
      id,
      targetText: text,
    });
    setSegments((prev) => prev.map((s) => (s.id === updated.id ? updated : s)));
  }, []);

  // Tab → scroll to + focus next row
  const parentRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: segments.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 40,
    overscan: 10,
  });

  const handleTabNext = useCallback(
    (currentIndex: number) => {
      const nextIndex = currentIndex + 1;
      if (nextIndex >= segments.length) return;
      virtualizer.scrollToIndex(nextIndex, { align: "auto" });
      requestAnimationFrame(() => {
        const nextInput = document.getElementById(`target-input-${nextIndex}`);
        nextInput?.focus();
      });
    },
    [segments.length, virtualizer],
  );

  const columns = useMemo(
    () =>
      createSegmentColumns({
        totalRows: segments.length,
        onSave: handleSave,
        onTabNext: handleTabNext,
      }),
    [segments.length, handleSave, handleTabNext],
  );

  const table = useReactTable({
    data: segments,
    columns,
    getCoreRowModel: getCoreRowModel(),
  });

  const rows = table.getRowModel().rows;

  // Empty states
  if (!activeFileId) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">
          Sélectionner un fichier dans le panneau gauche
        </p>
      </div>
    );
  }

  if (isLoading) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">Chargement…</p>
      </div>
    );
  }

  if (segments.length === 0) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">
          Aucun segment dans ce fichier
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
                onClick={() => setActiveSegment(row.original.id)}
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

      {/* Footer: segment count */}
      <div className="shrink-0 border-t px-3 py-1.5 text-xs text-muted-foreground">
        {segments.length.toLocaleString()} segments
      </div>
    </div>
  );
}
