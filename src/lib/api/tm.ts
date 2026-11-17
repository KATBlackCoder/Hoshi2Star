import { invoke } from "@tauri-apps/api/core";
import type { TmSuggestion } from "@/lib/types";

/** Translation memory. */
export const tmApi = {
  suggestions: (args: { sourceText: string; langPair: string }) =>
    invoke<TmSuggestion[]>("get_tm_suggestions", args),
};
