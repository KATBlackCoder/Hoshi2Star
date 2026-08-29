import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  PaginatedTerminology,
  PartOfSpeech,
  TerminologyEnforcement,
  TerminologyEntry,
  TerminologyEntryStatus,
  GlobalizeTranslationsSummary,
  TerminologyScanDone,
  TerminologyScanProgress,
  TerminologyStats,
  TerminologyTranslation,
  ProviderConfig,
} from "@/lib/types";

export interface TerminologyListInput {
  sourceLanguage: string;
  targetLanguage: string;
  projectId: string | null;
  search: string | null;
  partOfSpeech: PartOfSpeech | null;
  semanticType: string | null;
  status: TerminologyEntryStatus | null;
  page: number;
  pageSize: number;
}

export const terminologyApi = {
  list: (query: TerminologyListInput) =>
    invoke<PaginatedTerminology>("list_terminology", { query }),
  stats: (
    sourceLanguage: string,
    targetLanguage: string,
    projectId: string | null,
  ) =>
    invoke<TerminologyStats>("get_terminology_stats", {
      sourceLanguage,
      targetLanguage,
      projectId,
    }),
  create: (input: {
    sourceLanguage: string;
    canonicalText: string;
    reading: string | null;
    partOfSpeech: PartOfSpeech;
    semanticType: string;
    senseKey: string;
  }) => invoke<TerminologyEntry>("create_terminology_entry", { input }),
  update: (input: {
    id: string;
    canonicalText: string;
    reading: string | null;
    partOfSpeech: PartOfSpeech;
    semanticType: string;
    senseKey: string;
    status: TerminologyEntryStatus;
  }) => invoke<TerminologyEntry>("update_terminology_entry", { input }),
  archive: (entryId: string) =>
    invoke<void>("archive_terminology_entry", { entryId }),
  deleteEntries: (entryIds: string[]) =>
    invoke<number>("delete_terminology_entries", { entryIds }),
  upsertTranslation: (input: {
    entryId: string;
    targetLanguage: string;
    projectId: string | null;
    targetText: string;
    enforcement: TerminologyEnforcement;
    confidence: number;
    providerId: string | null;
    model: string | null;
    acceptedVariants: string[];
  }) =>
    invoke<TerminologyTranslation>("upsert_terminology_translation", { input }),
  startScan: (projectId: string) =>
    invoke<{ scanId: string }>("start_terminology_scan", { projectId }),
  cancelScan: (scanId: string) =>
    invoke<boolean>("cancel_terminology_scan", { scanId }),
  translate: (input: {
    entryIds: string[];
    targetLanguage: string;
    projectId: string | null;
    scope: "project" | "global" | "both";
    providerConfig: ProviderConfig;
  }) => invoke<{ jobId: string }>("translate_terminology_entries", { input }),
  globalize: (input: {
    entryIds: string[];
    targetLanguage: string;
    projectId: string;
  }) =>
    invoke<GlobalizeTranslationsSummary>("globalize_terminology_translations", {
      input,
    }),
  globalizeFiltered: (query: TerminologyListInput) =>
    invoke<GlobalizeTranslationsSummary>(
      "globalize_filtered_terminology_translations",
      { query },
    ),
  onScanProgress: (
    handler: (payload: TerminologyScanProgress) => void,
  ): Promise<UnlistenFn> =>
    listen<TerminologyScanProgress>(
      "h2s://terminology/scan-progress",
      (event) => handler(event.payload),
    ),
  onScanDone: (
    handler: (payload: TerminologyScanDone) => void,
  ): Promise<UnlistenFn> =>
    listen<TerminologyScanDone>("h2s://terminology/scan-done", (event) =>
      handler(event.payload),
    ),
  onTranslateProgress: (
    handler: (payload: {
      jobId: string;
      processed: number;
      total: number;
    }) => void,
  ): Promise<UnlistenFn> =>
    listen("h2s://terminology/translate-progress", (event) =>
      handler(
        event.payload as { jobId: string; processed: number; total: number },
      ),
    ),
  onTranslateDone: (
    handler: (payload: {
      jobId: string;
      summary: { translated: number; scopesWritten: number } | null;
      error: string | null;
    }) => void,
  ): Promise<UnlistenFn> =>
    listen("h2s://terminology/translate-done", (event) =>
      handler(
        event.payload as {
          jobId: string;
          summary: { translated: number; scopesWritten: number } | null;
          error: string | null;
        },
      ),
    ),
};
