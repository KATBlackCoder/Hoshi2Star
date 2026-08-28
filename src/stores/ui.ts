import { create } from "zustand";
import { persist } from "zustand/middleware";

export type AppMode = "library" | "patch" | "terminology" | "player" | "images";
export type InspectorTab = "tm" | "qa" | "terminology";
export type GridDensity = "compact" | "comfortable";

interface UiState {
  mode: AppMode;
  inspectorOpen: boolean;
  inspectorTab: InspectorTab;
  fileTreeOpen: boolean;
  gridDensity: GridDensity;
  setMode: (mode: AppMode) => void;
  toggleInspector: () => void;
  openInspector: (tab: InspectorTab) => void;
  setInspectorTab: (tab: InspectorTab) => void;
  setFileTreeOpen: (open: boolean) => void;
  toggleFileTree: () => void;
  setGridDensity: (density: GridDensity) => void;
}

export const useUiStore = create<UiState>()(
  persist(
    (set) => ({
      mode: "library",
      inspectorOpen: true,
      inspectorTab: "qa",
      fileTreeOpen: true,
      gridDensity: "comfortable",
      setMode: (mode) => set({ mode }),
      toggleInspector: () =>
        set((state) => ({ inspectorOpen: !state.inspectorOpen })),
      openInspector: (inspectorTab) =>
        set({ inspectorOpen: true, inspectorTab }),
      setInspectorTab: (inspectorTab) => set({ inspectorTab }),
      setFileTreeOpen: (fileTreeOpen) => set({ fileTreeOpen }),
      toggleFileTree: () =>
        set((state) => ({ fileTreeOpen: !state.fileTreeOpen })),
      setGridDensity: (gridDensity) => set({ gridDensity }),
    }),
    {
      name: "hoshi2star-ui",
      partialize: (state) => ({
        inspectorOpen: state.inspectorOpen,
        inspectorTab: state.inspectorTab,
        fileTreeOpen: state.fileTreeOpen,
        gridDensity: state.gridDensity,
      }),
    },
  ),
);

export const useAppMode = () => useUiStore((state) => state.mode);
