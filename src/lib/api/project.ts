import { invoke } from "@tauri-apps/api/core";
import type {
  OpenProjectResult,
  PaginatedSegments,
  Project,
  ProjectStats,
  Segment,
  SegmentSearchResult,
  SearchScope,
  SourceFile,
} from "@/lib/types";

/** Projects, their files and their segments. */
export const projectApi = {
  open: (args: { path: string; sourceLang: string; targetLang: string }) =>
    invoke<OpenProjectResult>("open_project", args),
  list: () => invoke<Project[]>("list_projects"),
  remove: (projectId: string) => invoke("delete_project", { projectId }),
  sourceFiles: (projectId: string) =>
    invoke<SourceFile[]>("get_source_files", { projectId }),
  stats: (projectId: string) =>
    invoke<ProjectStats>("get_project_stats", { projectId }),
  segments: (args: {
    projectId: string;
    fileId: string;
    page: number;
    pageSize: number;
  }) => invoke<PaginatedSegments>("get_segments", args),
  updateSegment: (args: { id: string; targetText: string }) =>
    invoke<Segment>("update_segment", args),
  search: (args: {
    projectId: string;
    query: string;
    scope: SearchScope;
    limit: number;
    offset: number;
  }) => invoke<SegmentSearchResult>("search_segments", args),
};
