import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { invalidateSegmentViews } from "@/lib/queryClient";
import { runWroteSegments, type RunSummary } from "@/stores/llm";
import { refreshProjectData, useProjectStore } from "@/stores/project";

/**
 * The single place that reacts to backend writes made outside the views
 * (translation run, pack import, resync): the active project's files are
 * fetched once, and the cached views derived from the segments go stale.
 * Each view still reloads its own rows (grid, search results).
 */
export function useProjectDataSync(): void {
  useEffect(() => {
    const refresh = () => {
      const projectId = useProjectStore.getState().activeProjectId;
      if (projectId) void refreshProjectData(projectId);
      else invalidateSegmentViews();
    };
    const unlisteners = [
      listen<RunSummary>("h2s://llm/completed", (event) => {
        if (runWroteSegments(event.payload)) refresh();
      }),
      listen("h2s://project/import-done", refresh),
      listen("h2s://project/resynced", refresh),
    ];
    return () => {
      for (const unlisten of unlisteners) void unlisten.then((fn) => fn());
    };
  }, []);
}
