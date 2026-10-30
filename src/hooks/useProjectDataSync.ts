import { useRef } from "react";
import { EVENTS } from "@/lib/events";
import { invalidateSegmentViews } from "@/lib/queryClient";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { runWroteSegments, type RunSummary } from "@/stores/llm";
import type { SegmentUpdate } from "@/lib/types";
import { refreshProjectData, useProjectStore } from "@/stores/project";

/**
 * The single place that reacts to backend writes made outside the views
 * (translation run, pack import, resync): the active project's files are
 * fetched once, and the cached views derived from the segments go stale.
 * Each view still reloads its own rows (grid, search results).
 */
export function useProjectDataSync(): void {
  // Batches persisted by the current run. A run that fails emits
  // `llm/error` without a summary (and `processed` there counts failed
  // segments too, never written): the batches are counted here instead.
  const runWroteRef = useRef(false);
  const refresh = () => {
    const projectId = useProjectStore.getState().activeProjectId;
    if (projectId) void refreshProjectData(projectId);
    else invalidateSegmentViews();
  };
  useTauriEvent<SegmentUpdate[]>(EVENTS.llmSegmentsUpdated, (updates) => {
    if (updates.length > 0) runWroteRef.current = true;
  });
  useTauriEvent<RunSummary>(EVENTS.llmCompleted, (summary) => {
    runWroteRef.current = false;
    if (runWroteSegments(summary)) refresh();
  });
  useTauriEvent(EVENTS.llmError, () => {
    const wrote = runWroteRef.current;
    runWroteRef.current = false;
    if (wrote) refresh();
  });
  useTauriEvent(EVENTS.projectImportDone, refresh);
  useTauriEvent(EVENTS.projectResynced, refresh);
}
