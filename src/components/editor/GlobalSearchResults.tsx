import { useEffect, useMemo, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useVirtualizer } from "@tanstack/react-virtual";
import { listen } from "@tauri-apps/api/event";
import {
  ChevronDown,
  ChevronRight,
  FileText,
  Loader2,
  Play,
} from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { StatusBadge } from "@/features/editor/columns";
import { useEditorStore } from "@/stores/editor";
import { useLlmStore, useIsTranslating } from "@/stores/llm";
import { refreshProjectData, useProjectStore } from "@/stores/project";
import { useSearchStore } from "@/stores/search";
import type { SegmentSearchHit, SegmentUpdate } from "@/lib/types";
import { cn } from "@/lib/utils";

/** One row of the results list: a file separator or an actual hit. */
type ResultRow =
  | { kind: "header"; fileId: string; fileName: string; count: number }
  | { kind: "hit"; hit: SegmentSearchHit };

/**
 * Project-wide search results — replaces SegmentGrid in the centre panel
 * while a global search is active. Hits are grouped by file (separator
 * rows, collapsible, with a sticky replica of the current file's header
 * while scrolling), selectable across files, and batch-translated via the
 * same explicit-ids path as the grid selection. Clicking a row navigates
 * to the segment in the normal grid view — unless a selection is in
 * progress, in which case clicking toggles the row instead.
 */
export function GlobalSearchResults() {
  const { t } = useTranslation();
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const setActiveFile = useEditorStore((s) => s.setActiveFile);
  const setActiveSegment = useEditorStore((s) => s.setActiveSegment);
  const { startTranslation, providerConfig } = useLlmStore();
  const isTranslating = useIsTranslating();

  const query = useSearchStore((s) => s.query);
  const results = useSearchStore((s) => s.results);
  const total = useSearchStore((s) => s.total);
  const status = useSearchStore((s) => s.status);
  const rerun = useSearchStore((s) => s.rerun);
  const deactivate = useSearchStore((s) => s.deactivate);

  // Selection + collapsed state are local: keyed by segment/file id, wiped
  // whenever a NEW search starts (new query, scope switch, post-translation
  // re-search — all pass through status "loading"). Keyed on status rather
  // than results so the background batches appending to a large result set
  // don't clear a selection the user is already making.
  const [selected, setSelected] = useState<Record<string, boolean>>({});
  const [collapsed, setCollapsed] = useState<Record<string, boolean>>({});
  useEffect(() => {
    if (status === "loading") {
      setSelected({});
      setCollapsed({});
    }
  }, [status]);

  const activeProjectIdRef = useRef(activeProjectId);
  activeProjectIdRef.current = activeProjectId;

  // SegmentGrid is unmounted while this view is shown, so mirror its LLM
  // listeners here: progressive in-memory patch, then full refresh on done.
  useEffect(() => {
    const unlisten = listen<SegmentUpdate[]>(
      "h2s://llm/segments-updated",
      (event) => {
        const updates = new Map(event.payload.map((u) => [u.id, u]));
        if (updates.size === 0) return;
        useSearchStore.setState((s) => ({
          results: s.results.map((h) => {
            const u = updates.get(h.id);
            return u ? { ...h, targetText: u.targetText, status: u.status } : h;
          }),
        }));
      },
    );
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, []);

  useEffect(() => {
    const unlisten = listen("h2s://llm/completed", () => {
      const pid = activeProjectIdRef.current;
      if (!pid) return;
      void rerun(pid);
      void refreshProjectData(pid);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [rerun]);

  // Hit ids per file — drives the per-file select-all checkbox and the
  // header hit counts (which must stay stable while a file is collapsed).
  const hitIdsByFile = useMemo(() => {
    const m = new Map<string, string[]>();
    for (const h of results) {
      const arr = m.get(h.sourceFileId);
      if (arr) arr.push(h.id);
      else m.set(h.sourceFileId, [h.id]);
    }
    return m;
  }, [results]);

  // Flat union array — results arrive sorted by file_name from SQL, so a
  // header is pushed every time the file changes. Hits of collapsed files
  // are skipped (their header stays, selection is unaffected).
  const rows = useMemo<ResultRow[]>(() => {
    const out: ResultRow[] = [];
    let currentFile: string | null = null;
    for (const hit of results) {
      if (hit.sourceFileId !== currentFile) {
        currentFile = hit.sourceFileId;
        out.push({
          kind: "header",
          fileId: hit.sourceFileId,
          fileName: hit.fileName,
          count: hitIdsByFile.get(hit.sourceFileId)?.length ?? 0,
        });
      }
      if (!collapsed[hit.sourceFileId]) out.push({ kind: "hit", hit });
    }
    return out;
  }, [results, collapsed, hitIdsByFile]);

  const fileCount = hitIdsByFile.size;

  const selectedIds = useMemo(
    () => Object.keys(selected).filter((id) => selected[id]),
    [selected],
  );
  const allSelected =
    results.length > 0 && selectedIds.length === results.length;

  const parentRef = useRef<HTMLDivElement>(null);
  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 40,
    overscan: 5,
  });

  function handleRowClick(hit: SegmentSearchHit) {
    // Selection mode: while at least one row is checked, clicking a row
    // extends/reduces the selection instead of navigating away — a stray
    // click can no longer throw the user out of the results view.
    if (selectedIds.length > 0) {
      setSelected((prev) => ({ ...prev, [hit.id]: !prev[hit.id] }));
      return;
    }
    // Order matters: setActiveFile resets activeSegmentId to null.
    setActiveFile(hit.sourceFileId);
    setActiveSegment(hit.id, hit.sourceText, hit.targetText);
    deactivate();
  }

  function handleTranslateSelected() {
    if (!providerConfig.model.trim()) {
      toast.error(t("segmentGrid.noModelConfigured"));
      return;
    }
    void startTranslation(selectedIds, undefined);
    setSelected({});
  }

  function toggleFileSelection(fileId: string) {
    const fileIds = hitIdsByFile.get(fileId) ?? [];
    const allFileSelected =
      fileIds.length > 0 && fileIds.every((id) => selected[id]);
    setSelected((prev) => {
      const next = { ...prev };
      for (const id of fileIds) next[id] = !allFileSelected;
      return next;
    });
  }

  /** Shared header bar — rendered both as a virtual row and as the sticky
   *  replica pinned on top of the scroll area. */
  function renderFileHeaderContent(
    fileId: string,
    fileName: string,
    count: number,
  ) {
    const fileIds = hitIdsByFile.get(fileId) ?? [];
    const allFileSelected =
      fileIds.length > 0 && fileIds.every((id) => selected[id]);
    const someFileSelected = fileIds.some((id) => selected[id]);
    const isCollapsed = collapsed[fileId] ?? false;
    return (
      <div className="flex items-center border-b bg-muted/40 py-1 text-xs font-semibold">
        {/* Same w-9 column as hit rows so the checkboxes align. */}
        <div className="flex w-9 shrink-0 items-center justify-center px-3">
          <Checkbox
            title={t("projectSearch.selectFile", { fileName })}
            checked={
              allFileSelected
                ? true
                : someFileSelected
                  ? "indeterminate"
                  : false
            }
            onCheckedChange={() => toggleFileSelection(fileId)}
          />
        </div>
        <div className="flex min-w-0 items-center gap-2">
          <FileText className="h-3.5 w-3.5 shrink-0 text-muted-foreground" />
          <span className="truncate">{fileName}</span>
          <span className="font-normal text-muted-foreground">({count})</span>
        </div>
        <button
          type="button"
          title={t(
            isCollapsed
              ? "projectSearch.expandFile"
              : "projectSearch.collapseFile",
            { fileName },
          )}
          className="ml-auto shrink-0 px-3 py-1 text-muted-foreground hover:text-foreground"
          onClick={() =>
            setCollapsed((prev) => ({ ...prev, [fileId]: !isCollapsed }))
          }
        >
          {isCollapsed ? (
            <ChevronRight className="h-3.5 w-3.5" />
          ) : (
            <ChevronDown className="h-3.5 w-3.5" />
          )}
        </button>
      </div>
    );
  }

  if (status === "loading") {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">
          {t("projectSearch.loading")}
        </p>
      </div>
    );
  }

  if (status === "error") {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-destructive">{t("projectSearch.error")}</p>
      </div>
    );
  }

  if (results.length === 0) {
    return (
      <div className="flex h-full items-center justify-center">
        <p className="text-sm text-muted-foreground">
          {t("projectSearch.noResults", { query })}
        </p>
      </div>
    );
  }

  const virtualItems = virtualizer.getVirtualItems();

  // Sticky header: while the first visible row is a hit, pin a replica of
  // its file's header on top of the scroll area. When a real header row is
  // itself first visible, the replica hides (no doubling) — visually the
  // next file's bar replaces the pinned one as it scrolls in.
  const firstVisibleRow =
    virtualItems.length > 0 ? rows[virtualItems[0].index] : null;
  const stickyHeader =
    firstVisibleRow && firstVisibleRow.kind === "hit"
      ? {
          fileId: firstVisibleRow.hit.sourceFileId,
          fileName: firstVisibleRow.hit.fileName,
          count:
            hitIdsByFile.get(firstVisibleRow.hit.sourceFileId)?.length ?? 0,
        }
      : null;

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Toolbar: select-all + summary + translate */}
      <div className="shrink-0 border-b px-3 py-1.5 flex items-center gap-3">
        <Checkbox
          title={t("projectSearch.selectAll")}
          checked={
            allSelected
              ? true
              : selectedIds.length > 0
                ? "indeterminate"
                : false
          }
          onCheckedChange={() =>
            setSelected(
              allSelected
                ? {}
                : Object.fromEntries(results.map((h) => [h.id, true])),
            )
          }
        />
        <span className="text-xs text-muted-foreground">
          {t("projectSearch.summary", {
            count: total,
            files: fileCount,
          })}
          {results.length < total && (
            <span className="ml-2 inline-flex items-center gap-1">
              <Loader2 className="h-3 w-3 animate-spin" />
              {t("projectSearch.loadingProgress", {
                loaded: results.length,
                total,
              })}
            </span>
          )}
        </span>

        {selectedIds.length >= 1 && (
          <Button
            size="sm"
            variant="outline"
            className="h-7 gap-1.5 text-xs ml-auto"
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

      {/* Virtual body + sticky file header replica */}
      <div className="relative flex-1 overflow-hidden">
        {stickyHeader && (
          <div
            data-testid="sticky-file-header"
            className="absolute inset-x-0 top-0 z-10 bg-background shadow-sm"
          >
            {renderFileHeaderContent(
              stickyHeader.fileId,
              stickyHeader.fileName,
              stickyHeader.count,
            )}
          </div>
        )}
        <div ref={parentRef} className="h-full overflow-auto">
          <div
            style={{ height: virtualizer.getTotalSize(), position: "relative" }}
          >
            {virtualItems.map((virtualRow) => {
              const row = rows[virtualRow.index];
              const baseStyle = {
                position: "absolute" as const,
                top: 0,
                left: 0,
                right: 0,
                transform: `translateY(${virtualRow.start}px)`,
              };

              if (row.kind === "header") {
                return (
                  <div
                    key={`header-${row.fileId}`}
                    data-index={virtualRow.index}
                    ref={virtualizer.measureElement}
                    style={baseStyle}
                  >
                    {renderFileHeaderContent(
                      row.fileId,
                      row.fileName,
                      row.count,
                    )}
                  </div>
                );
              }

              const { hit } = row;
              return (
                <div
                  key={hit.id}
                  data-index={virtualRow.index}
                  ref={virtualizer.measureElement}
                  style={baseStyle}
                  onClick={() => handleRowClick(hit)}
                  className="group flex border-b border-border/50 hover:bg-accent/30 transition-colors cursor-pointer"
                >
                  <div className="flex w-9 shrink-0 items-center justify-center px-3 py-2">
                    <Checkbox
                      checked={selected[hit.id] ?? false}
                      onCheckedChange={() =>
                        setSelected((prev) => ({
                          ...prev,
                          [hit.id]: !prev[hit.id],
                        }))
                      }
                      onClick={(e) => e.stopPropagation()}
                    />
                  </div>
                  <div className="flex w-40 shrink-0 items-start px-3 py-2">
                    <span className="truncate font-mono text-[10px] text-muted-foreground">
                      {hit.jsonKey}
                    </span>
                  </div>
                  <div className="flex flex-1 min-w-0 items-start px-3 py-2">
                    <span className="text-xs whitespace-pre-wrap break-words">
                      {hit.sourceText}
                    </span>
                  </div>
                  <div className="flex flex-1 min-w-0 items-start px-3 py-2">
                    <span className="text-xs whitespace-pre-wrap break-words text-muted-foreground">
                      {hit.targetText}
                    </span>
                  </div>
                  <div className="flex w-28 shrink-0 items-center px-3 py-2">
                    <StatusBadge status={hit.status} />
                  </div>
                  <div className="flex w-14 shrink-0 items-center justify-center px-3 py-2">
                    <span
                      className={cn(
                        "text-xs tabular-nums",
                        hit.qaScore === null
                          ? "text-muted-foreground"
                          : hit.qaScore < 100
                            ? "text-amber-600 dark:text-amber-300 font-medium"
                            : "text-muted-foreground",
                      )}
                    >
                      {hit.qaScore === null ? "—" : hit.qaScore}
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        </div>
      </div>
    </div>
  );
}
