import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { t } from "i18next";
import { toast } from "sonner";
import type { OpenProjectResult, Project, SourceFile } from "@/lib/types";

interface ProjectState {
  projects: Project[];
  activeProjectId: string | null;
  sourceFiles: SourceFile[];
  /** project.id en attente de réponse utilisateur (oui/non) — null si aucun */
  pendingGlossaryExtract: string | null;
  /** true pendant que extract_glossary_terms tourne en arrière-plan */
  isExtractingGlossary: boolean;

  // Actions
  addProject: (project: Project) => void;
  setActiveProject: (id: string | null) => void;
  setSourceFiles: (files: SourceFile[]) => void;
  removeProject: (id: string) => void;
  setPendingGlossaryExtract: (id: string | null) => void;
  setExtractingGlossary: (v: boolean) => void;
}

export const useProjectStore = create<ProjectState>()((set) => ({
  projects: [],
  activeProjectId: null,
  sourceFiles: [],
  pendingGlossaryExtract: null,
  isExtractingGlossary: false,

  addProject: (project) =>
    set((state) => ({ projects: [...state.projects, project] })),

  setActiveProject: (id) => set({ activeProjectId: id }),

  setSourceFiles: (files) => set({ sourceFiles: files }),

  removeProject: (id) =>
    set((state) => ({ projects: state.projects.filter((p) => p.id !== id) })),

  setPendingGlossaryExtract: (id) => set({ pendingGlossaryExtract: id }),

  setExtractingGlossary: (v) => set({ isExtractingGlossary: v }),
}));

// Selectors
export const useActiveProject = () =>
  useProjectStore(
    (s) => s.projects.find((p) => p.id === s.activeProjectId) ?? null,
  );

export const useSourceFiles = () => useProjectStore((s) => s.sourceFiles);

export const usePendingGlossaryExtract = () =>
  useProjectStore((s) => s.pendingGlossaryExtract);

export const useIsExtractingGlossary = () =>
  useProjectStore((s) => s.isExtractingGlossary);

// Thunk: open a game folder via Tauri and register the project in the store.
export async function openProject(
  gamePath: string,
): Promise<OpenProjectResult> {
  const result = await invoke<OpenProjectResult>("open_project", {
    path: gamePath,
  });
  const { project, wasRestored } = result;
  useProjectStore.getState().addProject(project);
  useProjectStore.getState().setActiveProject(project.id);
  if (!wasRestored) {
    useProjectStore.getState().setPendingGlossaryExtract(project.id);
  }

  const files = await invoke<SourceFile[]>("get_source_files", {
    projectId: project.id,
  });
  useProjectStore.getState().setSourceFiles(files);

  if (wasRestored) {
    toast.success(t("project.restored"));
  }

  return { project, wasRestored };
}

// Thunk: load all known projects from the DB into the store.
export async function loadAllProjects(): Promise<void> {
  const projects = await invoke<Project[]>("list_projects");
  // Merge: keep the active project unchanged, add any missing ones
  const existing = useProjectStore.getState().projects;
  const existingIds = new Set(existing.map((p) => p.id));
  const merged = [
    ...existing,
    ...projects.filter((p) => !existingIds.has(p.id)),
  ];
  useProjectStore.setState({ projects: merged });
}

// Thunk: delete a project from DB + manifest, remove from store.
export async function deleteProject(projectId: string): Promise<void> {
  await invoke("delete_project", { projectId });
  const state = useProjectStore.getState();
  state.removeProject(projectId);
  // If the deleted project was active, clear editor state
  if (state.activeProjectId === projectId) {
    useProjectStore.setState({
      activeProjectId: null,
      sourceFiles: [],
    });
  }
}
