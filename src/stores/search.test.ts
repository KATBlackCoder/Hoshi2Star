import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockIPC } from "@tauri-apps/api/mocks";
import { SEARCH_LIMIT, useSearchStore } from "@/stores/search";
import type { SegmentSearchHit } from "@/lib/types";

const initialSearch = useSearchStore.getState();

const HIT: SegmentSearchHit = {
  id: "s1",
  sourceFileId: "f1",
  jsonKey: "/k/0",
  sourceText: "かっこいい",
  segmentKind: "unknown",
  sceneId: null,
  sequenceIndex: null,
  speaker: null,
  branchPath: null,
  contextJson: null,
  targetText: "so cool",
  status: "translated",
  qaScore: null,
  createdAt: "",
  updatedAt: "",
  fileName: "Actors.json",
};

beforeEach(() => {
  useSearchStore.setState(initialSearch, true);
});

describe("search store — runSearch", () => {
  it("fills results/total and activates the view on success", async () => {
    const invokeSpy = vi.fn();
    mockIPC((cmd, args) => {
      invokeSpy(cmd, args);
      if (cmd === "search_segments") return { items: [HIT], total: 1 };
      throw new Error(`unexpected command: ${cmd}`);
    });

    await useSearchStore.getState().runSearch("p1", "  cool  ", "both");

    const s = useSearchStore.getState();
    expect(s.results).toEqual([HIT]);
    expect(s.total).toBe(1);
    expect(s.status).toBe("idle");
    expect(s.isActive).toBe(true);
    expect(s.query).toBe("cool"); // trimmed
    expect(invokeSpy).toHaveBeenCalledWith("search_segments", {
      projectId: "p1",
      query: "cool",
      scope: "both",
      limit: SEARCH_LIMIT,
      offset: 0,
    });
  });

  it("fetches ALL matches in successive batches (server pages of N)", async () => {
    // Server serves 2 hits per request regardless of the asked limit —
    // the loop must keep fetching with a growing offset until total.
    const all = ["a", "b", "c", "d", "e"].map((id) => ({
      ...HIT,
      id,
    }));
    const offsets: number[] = [];
    mockIPC((cmd, args) => {
      if (cmd !== "search_segments") throw new Error(`unexpected: ${cmd}`);
      const { offset } = args as { offset: number };
      offsets.push(offset);
      return { items: all.slice(offset, offset + 2), total: all.length };
    });

    await useSearchStore.getState().runSearch("p1", "cool", "both");

    const s = useSearchStore.getState();
    expect(s.results.map((h) => h.id)).toEqual(["a", "b", "c", "d", "e"]);
    expect(s.total).toBe(5);
    expect(s.status).toBe("idle");
    // 5 hits in server pages of 2 → offsets 0, 2, 4.
    expect(offsets).toEqual([0, 2, 4]);
  });

  it("does not invoke for a query shorter than 2 chars", async () => {
    const invokeSpy = vi.fn();
    mockIPC((cmd) => {
      invokeSpy(cmd);
      return { items: [], total: 0 };
    });

    await useSearchStore.getState().runSearch("p1", " a ", "both");

    expect(invokeSpy).not.toHaveBeenCalled();
    expect(useSearchStore.getState().isActive).toBe(false);
  });

  it("sets status=error and empties results when the IPC call fails", async () => {
    mockIPC(() => {
      throw new Error("db locked");
    });

    await useSearchStore.getState().runSearch("p1", "cool", "both");

    const s = useSearchStore.getState();
    expect(s.status).toBe("error");
    expect(s.results).toEqual([]);
    expect(s.total).toBe(0);
    expect(s.isActive).toBe(true); // view stays open to show the error state
  });
});

describe("search store — deactivate / clear", () => {
  it("deactivate hides the view but keeps query and results", () => {
    useSearchStore.setState({
      query: "cool",
      results: [HIT],
      total: 1,
      isActive: true,
    });

    useSearchStore.getState().deactivate();

    const s = useSearchStore.getState();
    expect(s.isActive).toBe(false);
    expect(s.query).toBe("cool");
    expect(s.results).toEqual([HIT]);
  });

  it("clear resets everything except the scope", () => {
    useSearchStore.setState({
      query: "cool",
      scope: "target",
      results: [HIT],
      total: 1,
      status: "error",
      isActive: true,
    });

    useSearchStore.getState().clear();

    const s = useSearchStore.getState();
    expect(s.query).toBe("");
    expect(s.results).toEqual([]);
    expect(s.total).toBe(0);
    expect(s.status).toBe("idle");
    expect(s.isActive).toBe(false);
    expect(s.scope).toBe("target"); // kept
  });
});
