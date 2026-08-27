import { beforeEach, describe, expect, it } from "vitest";
import { useEditorStore } from "@/stores/editor";

const initialState = useEditorStore.getState();

beforeEach(() => {
  useEditorStore.setState(initialState, true);
});

describe("editor store", () => {
  it("setActiveSegment stores id + source/target texts", () => {
    useEditorStore.getState().setActiveSegment("s1", "ソース", "target");
    const s = useEditorStore.getState();
    expect(s.activeSegmentId).toBe("s1");
    expect(s.activeSegmentSourceText).toBe("ソース");
    expect(s.activeSegmentTargetText).toBe("target");
  });

  it("setActiveFile resets the active segment", () => {
    useEditorStore.getState().setActiveSegment("s1", "src", "tgt");
    useEditorStore.getState().setActiveFile("f2");
    const s = useEditorStore.getState();
    expect(s.activeFileId).toBe("f2");
    expect(s.activeSegmentId).toBeNull();
    expect(s.activeSegmentSourceText).toBeNull();
    expect(s.activeSegmentTargetText).toBeNull();
  });
});
