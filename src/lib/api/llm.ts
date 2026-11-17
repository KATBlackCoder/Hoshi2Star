import { invoke } from "@tauri-apps/api/core";
import type { PilotRunReport, ProviderConfig } from "@/lib/types";

/** LLM runs, the provider's models and the MV/MZ pilot. */
export const llmApi = {
  /** Start a translation run (`translate_*` commands). */
  run: (command: string, args: Record<string, unknown>) =>
    invoke(command, args),
  cancel: () => invoke("cancel_translation"),
  providerModels: (providerConfig: ProviderConfig) =>
    invoke<string[]>("get_provider_models", { providerConfig }),
  runPilot: (args: {
    gamePath: string;
    sourceLang: string;
    targetLang: string;
    sampleSize: number;
    providerConfig: ProviderConfig;
  }) => invoke<PilotRunReport>("run_mv_mz_pilot", args),
};
