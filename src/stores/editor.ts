import { create } from "zustand";

interface EditorState {
  activeFileId: string | null;
  activeSegmentId: string | null;
  /** Source text of the currently selected segment — for TM / QA panels. */
  activeSegmentSourceText: string | null;
  /** Target text of the currently selected segment — for live QA check. */
  activeSegmentTargetText: string | null;

  // Actions
  setActiveFile: (id: string | null) => void;
  setActiveSegment: (
    id: string | null,
    sourceText?: string | null,
    targetText?: string | null,
  ) => void;
}

export const useEditorStore = create<EditorState>()((set) => ({
  activeFileId: null,
  activeSegmentId: null,
  activeSegmentSourceText: null,
  activeSegmentTargetText: null,

  setActiveFile: (id) =>
    set({
      activeFileId: id,
      activeSegmentId: null,
      activeSegmentSourceText: null,
      activeSegmentTargetText: null,
    }),

  setActiveSegment: (id, sourceText = null, targetText = null) =>
    set({
      activeSegmentId: id,
      activeSegmentSourceText: sourceText,
      activeSegmentTargetText: targetText,
    }),
}));

// Selectors
export const useActiveFileId = () => useEditorStore((s) => s.activeFileId);
export const useActiveSegmentId = () =>
  useEditorStore((s) => s.activeSegmentId);
export const useActiveSegmentSourceText = () =>
  useEditorStore((s) => s.activeSegmentSourceText);
export const useActiveSegmentTargetText = () =>
  useEditorStore((s) => s.activeSegmentTargetText);
