import { EVENTS } from "@/lib/events";
import { invalidateSegmentViews } from "@/lib/queryClient";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { runWroteSegments, type RunSummary } from "@/stores/llm";
import { refreshProjectData, useProjectStore } from "@/stores/project";

/**
 * The single place that reacts to backend writes made outside the views
 * (translation run, pack import, resync): the active project's files are
 * fetched once, and the cached views derived from the segments go stale.
 * Each view still reloads its own rows (grid, search results).
 */
export function useProjectDataSync(): void {
  const refresh = () => {
    const projectId = useProjectStore.getState().activeProjectId;
    if (projectId) void refreshProjectData(projectId);
    else invalidateSegmentViews();
  };
  useTauriEvent<RunSummary>(EVENTS.llmCompleted, (summary) => {
    if (runWroteSegments(summary)) refresh();
  });
  useTauriEvent(EVENTS.projectImportDone, refresh);
  useTauriEvent(EVENTS.projectResynced, refresh);
}
