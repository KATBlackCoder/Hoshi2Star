import { invoke } from "@tauri-apps/api/core";
import type {
  CategoryChange,
  CoverageStatus,
  CoverageView,
} from "@/lib/types";

/** Optional text categories of MV/MZ projects. */
export const coverageApi = {
  scan: (projectId: string) =>
    invoke<CoverageView>("scan_text_coverage", { projectId }),
  apply: (args: { projectId: string; changes: CategoryChange[] }) =>
    invoke<CoverageView>("set_text_categories", args),
  status: (projectId: string) =>
    invoke<CoverageStatus>("get_text_coverage_status", { projectId }),
};
