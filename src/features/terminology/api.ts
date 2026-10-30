import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { EVENTS } from "@/lib/events";
import type {
  PaginatedTerminology,
  PartOfSpeech,
  TerminologyEnforcement,
  TerminologyEntry,
  TerminologyEntryStatus,
  GlobalizeTranslationsSummary,
  TerminologyScope,
  TerminologyStats,
  TerminologyTranslateDone,
  TerminologyTranslateProgress,
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
    scope: TerminologyScope | "both";
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
  onTranslateProgress: (
    handler: (payload: TerminologyTranslateProgress) => void,
  ): Promise<UnlistenFn> =>
    listen<TerminologyTranslateProgress>(
      EVENTS.terminologyTranslateProgress,
      (event) => handler(event.payload),
    ),
  onTranslateDone: (
    handler: (payload: TerminologyTranslateDone) => void,
  ): Promise<UnlistenFn> =>
    listen<TerminologyTranslateDone>(EVENTS.terminologyTranslateDone, (event) =>
      handler(event.payload),
    ),
};
