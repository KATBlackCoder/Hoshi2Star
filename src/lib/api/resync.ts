import { invoke } from "@tauri-apps/api/core";
import type { ResyncSummary } from "@/lib/types";

/** Realignment after a game update (MV/MZ). */
export const resyncApi = {
  run: (args: { projectId: string; apply: boolean }) =>
    invoke<ResyncSummary>("resync_project", args),
};
