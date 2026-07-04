import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type {
  SearchScope,
  SegmentSearchHit,
  SegmentSearchResult,
} from "@/lib/types";

/** Per-request batch size — mirrored from Rust `SEARCH_BATCH_SIZE`. */
export const SEARCH_LIMIT = 500;

/**
 * Monotonic guard against out-of-order responses (same pattern as
 * SegmentGrid's loadSeqRef): a stale search must not overwrite the
 * results of a newer one.
 */
let searchSeq = 0;

interface SearchState {
  /** Last committed (executed) query — the input keeps its own draft. */
  query: string;
  scope: SearchScope;
  results: SegmentSearchHit[];
  /**
   * Real match count. Every match is loaded (successive SEARCH_LIMIT
   * batches, same pattern as SegmentGrid's full-file loading), so
   * `results.length < total` only while batches are still arriving.
   */
  total: number;
  status: "idle" | "loading" | "error";
  /** Whether the results view replaces the grid in the centre panel. */
  isActive: boolean;

  runSearch: (
    projectId: string,
    query: string,
    scope: SearchScope,
  ) => Promise<void>;
  /** Re-execute the committed query (after a batch translation completes). */
  rerun: (projectId: string) => Promise<void>;
  /** Hide the results view but keep query/results (row-click navigation). */
  deactivate: () => void;
  /** Full reset (clear button, project switch). Scope is kept. */
  clear: () => void;
}

export const useSearchStore = create<SearchState>()((set, get) => ({
  query: "",
  scope: "both",
  results: [],
  total: 0,
  status: "idle",
  isActive: false,

  runSearch: async (projectId, query, scope) => {
    const q = query.trim();
    if (q.length < 2) return;
    const seq = ++searchSeq;
    set({
      status: "loading",
      query: q,
      scope,
      isActive: true,
      results: [],
      total: 0,
    });
    try {
      // Fetch ALL matches in successive batches; the first one renders
      // immediately, the rest append in the background.
      let all: SegmentSearchHit[] = [];
      for (;;) {
        const result = await invoke<SegmentSearchResult>("search_segments", {
          projectId,
          query: q,
          scope,
          limit: SEARCH_LIMIT,
          offset: all.length,
        });
        if (seq !== searchSeq) return;
        all = all.concat(result.items);
        set({ results: all, total: result.total, status: "idle" });
        if (all.length >= result.total || result.items.length === 0) break;
      }
    } catch {
      if (seq !== searchSeq) return;
      set({ results: [], total: 0, status: "error" });
    }
  },

  rerun: async (projectId) => {
    const { query, scope } = get();
    if (query.length >= 2) await get().runSearch(projectId, query, scope);
  },

  deactivate: () => set({ isActive: false }),

  clear: () =>
    set({ query: "", results: [], total: 0, status: "idle", isActive: false }),
}));

// Selectors
export const useSearchActive = () => useSearchStore((s) => s.isActive);
export const useSearchResults = () => useSearchStore((s) => s.results);
export const useSearchStatus = () => useSearchStore((s) => s.status);
