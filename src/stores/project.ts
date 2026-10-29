import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { t } from "i18next";
import { toast } from "sonner";
import type {
  OpenProjectResult,
  Project,
  ProjectStats,
  SourceFile,
} from "@/lib/types";
import { invalidateSegmentViews } from "@/lib/queryClient";
import { applyPostOpenCoverage, openCoverageDialog } from "@/stores/coverage";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";

interface ProjectState {
  projects: Project[];
  activeProjectId: string | null;
  sourceFiles: SourceFile[];

  // Actions
  addProject: (project: Project) => void;
  setActiveProject: (id: string | null) => void;
  setSourceFiles: (files: SourceFile[]) => void;
  removeProject: (id: string) => void;
}

export const useProjectStore = create<ProjectState>()((set) => ({
  projects: [],
  activeProjectId: null,
  sourceFiles: [],

  addProject: (project) =>
    set((state) => {
      const existingIndex = state.projects.findIndex(
        (candidate) => candidate.id === project.id,
      );
      if (existingIndex === -1) {
        return { projects: [...state.projects, project] };
      }

      return {
        projects: state.projects.map((candidate, index) =>
          index === existingIndex ? project : candidate,
        ),
      };
    }),

  setActiveProject: (id) => set({ activeProjectId: id }),

  setSourceFiles: (files) => set({ sourceFiles: files }),

  removeProject: (id) =>
    set((state) => ({ projects: state.projects.filter((p) => p.id !== id) })),

}));

// Selectors
export const useActiveProject = () =>
  useProjectStore(
    (s) => s.projects.find((p) => p.id === s.activeProjectId) ?? null,
  );

/** Narrow selector — returns only the engine string, avoids object re-creation on unrelated store updates. */
export const useActiveEngine = () =>
  useProjectStore(
    (s) => s.projects.find((p) => p.id === s.activeProjectId)?.engine ?? "",
  );

export const useSourceFiles = () => useProjectStore((s) => s.sourceFiles);

export const useActiveLangPair = () =>
  useProjectStore((state) => {
    const project = state.projects.find(
      (candidate) => candidate.id === state.activeProjectId,
    );
    return project
      ? `${project.sourceLang}-${project.targetLang}`
      : `${useSettingsStore.getState().settings.defaultSourceLang}-${useSettingsStore.getState().settings.defaultTargetLang}`;
  });

export function getProjectLangPair(projectId: string): string {
  const project = useProjectStore
    .getState()
    .projects.find((candidate) => candidate.id === projectId);
  if (project) return `${project.sourceLang}-${project.targetLang}`;
  const settings = useSettingsStore.getState().settings;
  return `${settings.defaultSourceLang}-${settings.defaultTargetLang}`;
}

// Thunk: open a game folder via Tauri and register the project in the store.
export async function openProject(
  gamePath: string,
): Promise<OpenProjectResult> {
  const { defaultSourceLang, defaultTargetLang } =
    useSettingsStore.getState().settings;
  const result = await invoke<OpenProjectResult>("open_project", {
    path: gamePath,
    sourceLang: defaultSourceLang,
    targetLang: defaultTargetLang,
  });
  const { project, wasRestored } = result;
  useProjectStore.getState().addProject(project);
  useProjectStore.getState().setActiveProject(project.id);
  useUiStore.getState().setMode("patch");
  // A fresh extraction may rewrite a known project: its cached views go stale.
  invalidateSegmentViews();
  // The stats only feed the extraction notice: not fetched for a restore.
  const [files, stats] = await Promise.all([
    invoke<SourceFile[]>("get_source_files", { projectId: project.id }),
    wasRestored
      ? null
      : invoke<ProjectStats>("get_project_stats", { projectId: project.id }),
  ]);
  useProjectStore.getState().setSourceFiles(files);

  if (!stats) {
    toast.success(t("project.restored"));
  } else {
    toast.success(
      t("project.extracted", {
        total: stats.totalSegments,
        translated: stats.translatedCount,
      }),
    );
  }

  if (project.engine === "mv_mz") {
    // Optional plugin/system text: offered, never enabled behind the user's
    // back. Best-effort, never blocks opening.
    try {
      const outcome = await applyPostOpenCoverage(project.id, wasRestored);
      if (outcome.kind !== "none") {
        toast.info(
          outcome.kind === "available"
            ? t("coverage.available", { count: outcome.categories })
            : t("coverage.undecided"),
          {
            duration: 15000,
            action: {
              label: t("coverage.reviewAction"),
              onClick: () => void openCoverageDialog(project.id),
            },
          },
        );
      }
    } catch (err) {
      toast.warning(t("coverage.scanFailed", { error: String(err) }));
    }
  }

  return { project, wasRestored };
}

// Thunk: segments were written — reload the source files (per-file counts)
// and mark the cached views derived from the segments stale.
// Best-effort: a failed refresh must never surface as an error toast.
export async function refreshProjectData(projectId: string): Promise<void> {
  invalidateSegmentViews();
  try {
    const files = await invoke<SourceFile[]>("get_source_files", { projectId });
    useProjectStore.getState().setSourceFiles(files);
  } catch {
    // silent — stale counters are acceptable, a toast here would be noise
  }
}

// Thunk: load all known projects from the DB into the store.
export async function loadAllProjects(): Promise<void> {
  const projects = await invoke<Project[]>("list_projects");
  // Merge: keep the active project unchanged, add any missing ones
  const existing = useProjectStore.getState().projects;
  const existingIds = new Set(existing.map((p) => p.id));
  const missing = projects.filter((p) => !existingIds.has(p.id));
  // Same list: keep the array, so subscribers do not render for nothing.
  if (missing.length === 0) return;
  useProjectStore.setState({ projects: [...existing, ...missing] });
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
