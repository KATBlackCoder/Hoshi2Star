import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import type { OpenProjectResult, Project, SourceFile } from "@/lib/types";

interface ProjectState {
  projects: Project[];
  activeProjectId: string | null;
  sourceFiles: SourceFile[];

  // Actions
  addProject: (project: Project) => void;
  setActiveProject: (id: string | null) => void;
  setSourceFiles: (files: SourceFile[]) => void;
}

export const useProjectStore = create<ProjectState>()((set) => ({
  projects: [],
  activeProjectId: null,
  sourceFiles: [],

  addProject: (project) =>
    set((state) => ({ projects: [...state.projects, project] })),

  setActiveProject: (id) => set({ activeProjectId: id }),

  setSourceFiles: (files) => set({ sourceFiles: files }),
}));

// Selectors
export const useActiveProject = () =>
  useProjectStore(
    (s) => s.projects.find((p) => p.id === s.activeProjectId) ?? null,
  );

export const useSourceFiles = () => useProjectStore((s) => s.sourceFiles);

// Thunk: open a game folder via Tauri and register the project in the store.
// Returns { project, wasRestored } so callers can show a toast if wasRestored === true.
export async function openProject(
  gamePath: string,
): Promise<OpenProjectResult> {
  const result = await invoke<OpenProjectResult>("open_project", {
    path: gamePath,
  });
  const { project, wasRestored } = result;
  useProjectStore.getState().addProject(project);
  useProjectStore.getState().setActiveProject(project.id);

  const files = await invoke<SourceFile[]>("get_source_files", {
    projectId: project.id,
  });
  useProjectStore.getState().setSourceFiles(files);

  return { project, wasRestored };
}
