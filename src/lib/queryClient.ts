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
 * Segments were written (save, run, import, resync…): mark the cached views
 * derived from them stale. Shown views refetch once; hidden ones (the
 * library's stats) refetch the next time they are shown, not before.
 */
export function invalidateSegmentViews(): void {
  for (const queryKey of Object.values(queryKeys)) {
    void queryClient.invalidateQueries({ queryKey });
  }
}
