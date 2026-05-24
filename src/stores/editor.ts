import { create } from "zustand";

interface EditorState {
  activeFileId: string | null;
  activeSegmentId: string | null;

  // Actions
  setActiveFile: (id: string | null) => void;
  setActiveSegment: (id: string | null) => void;
}

export const useEditorStore = create<EditorState>()((set) => ({
  activeFileId: null,
  activeSegmentId: null,

  setActiveFile: (id) => set({ activeFileId: id, activeSegmentId: null }),
  setActiveSegment: (id) => set({ activeSegmentId: id }),
}));

// Selectors
export const useActiveFileId = () => useEditorStore((s) => s.activeFileId);
export const useActiveSegmentId = () =>
  useEditorStore((s) => s.activeSegmentId);
