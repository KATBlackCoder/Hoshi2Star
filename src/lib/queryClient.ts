import { QueryClient } from "@tanstack/react-query";

/**
 * The app's single React Query client. Exported so that stores and event
 * listeners can invalidate cached views after a write, outside components.
 */
export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 30, // 30 s — IPC data doesn't go stale quickly
      retry: 1,
    },
  },
});

/** Query-key roots of the cached views derived from the segments. */
export const queryKeys = {
  projectStats: ["project-stats"],
  qaReport: ["qa-report"],
  tmSuggestions: ["tm-suggestions"],
} as const;

/**
 * Segments were written in bulk (run, import, resync, coverage, opening):
 * mark the cached views derived from them stale, at once. Shown views
 * refetch once; hidden ones (the library's stats) refetch the next time they
 * are shown, not before.
 */
export function invalidateSegmentViews(): void {
  cancelQaReportRefresh();
  for (const queryKey of Object.values(queryKeys)) {
    void queryClient.invalidateQueries({ queryKey });
  }
}

/** Quiet time after the last save before the project QA report refreshes. */
export const QA_REPORT_REFRESH_DELAY_MS = 1_500;

let qaReportRefresh: ReturnType<typeof setTimeout> | null = null;

function cancelQaReportRefresh(): void {
  if (qaReportRefresh === null) return;
  clearTimeout(qaReportRefresh);
  qaReportRefresh = null;
}

/**
 * One segment was saved from the editor. Stats and TM suggestions go stale at
 * once; the project QA report (a QA pass over the whole project, backend
 * side) refreshes once the saves stop, not after each Tab.
 */
export function invalidateAfterSegmentSave(): void {
  void queryClient.invalidateQueries({ queryKey: queryKeys.projectStats });
  void queryClient.invalidateQueries({ queryKey: queryKeys.tmSuggestions });
  cancelQaReportRefresh();
  qaReportRefresh = setTimeout(() => {
    qaReportRefresh = null;
    void queryClient.invalidateQueries({ queryKey: queryKeys.qaReport });
  }, QA_REPORT_REFRESH_DELAY_MS);
}
