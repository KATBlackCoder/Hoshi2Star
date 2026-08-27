// Domain types — mirror the Rust structs in commands/project.rs.
// All fields are snake_case to match Tauri's camelCase serialisation rules
// (Tauri converts Rust snake_case → camelCase automatically via serde).

export type SegmentStatus =
  | "untranslated"
  | "translated"
  | "reviewed"
  | "needs_review";

export interface Project {
  id: string;
  name: string;
  engine: string;
  gamePath: string;
  sourceLang: string;
  targetLang: string;
  createdAt: string;
  updatedAt: string;
}

export interface OpenProjectResult {
  project: Project;
  wasRestored: boolean;
}

export interface SourceFile {
  id: string;
  projectId: string;
  fileName: string;
  filePath: string;
  fileType: string;
  translationSecs: number | null;
  /** Segments with status 'translated' (strict — needs_review not included). */
  translatedCount: number;
  /** Segments with status 'needs_review' (remaining review work). */
  needsReviewCount: number;
  totalCount: number;
}

export interface FontScanResult {
  existingFontCount: number;
  totalTranslated: number;
  engine: string;
}

export interface ProjectStats {
  fileCount: number;
  totalSegments: number;
  untranslatedCount: number;
  translatedCount: number;
  needsReviewCount: number;
  /** With the other three counters, the four statuses sum to totalSegments. */
  reviewedCount: number;
}

export interface Segment {
  id: string;
  sourceFileId: string;
  jsonKey: string;
  sourceText: string;
  /** Stable semantic kind. `unknown` means that engine has no context adapter yet. */
  segmentKind: string;
  /** Engine-owned scene boundary used to retrieve safe neighbouring lines. */
  sceneId: string | null;
  /** Zero-based order among extracted segments in the same scene. */
  sequenceIndex: number | null;
  speaker: string | null;
  branchPath: string | null;
  /** Versioned engine-specific facts; never interpreted as another engine's schema. */
  contextJson: string | null;
  targetText: string;
  status: SegmentStatus;
  qaScore: number | null;
  createdAt: string;
  updatedAt: string;
}

/// One segment's `target_text`/`status` as just persisted to the DB.
/// Emitted as part of `h2s://llm/segments-updated` (one event per batch).
export interface SegmentUpdate {
  id: string;
  targetText: string;
  status: SegmentStatus;
}

export interface PaginatedSegments {
  items: Segment[];
  total: number;
  page: number;
  pageSize: number;
}

/** Which segment column(s) a project-wide search matches against. */
export type SearchScope = "both" | "source" | "target";

/** One project-wide search hit — a segment plus its file's name for grouping. */
export interface SegmentSearchHit extends Segment {
  fileName: string;
}

export interface SegmentSearchResult {
  items: SegmentSearchHit[];
  /** Real match count — `items` is capped server-side (500). */
  total: number;
}

// ---------------------------------------------------------------------------
// TM
// ---------------------------------------------------------------------------

export interface TmEntry {
  id: string;
  sourceHash: string;
  sourceText: string;
  targetText: string;
  engine: string;
  langPair: string;
  confidence: number;
  createdAt: string;
}

export interface TmSuggestion {
  entry: TmEntry;
  score: number;
  matchType: "exact" | "fuzzy";
}

// ---------------------------------------------------------------------------
// QA
// ---------------------------------------------------------------------------

export type QaErrorType =
  | { type: "missing_placeholder"; placeholder: string }
  | {
      type: "line_too_long";
      line: number;
      units: number;
      max_units: number;
      char_count: number;
    }
  | { type: "bom_detected" }
  | { type: "empty_translation" }
  | { type: "unchanged_source" }
  | { type: "source_script_remaining"; source_language: string }
  | {
      type: "suspicious_expansion";
      source_chars: number;
      target_chars: number;
    }
  | { type: "context_leak"; neighbor_text: string }
  | { type: "inconsistent_repeated_source"; variants: number }
  | {
      type: "terminology_mismatch";
      entry_id: string;
      source_term: string;
      expected_targets: string[];
      severity: "critical" | "warning" | "info";
    };

export interface QaResult {
  score: number;
  errors: QaErrorType[];
}

export interface QaReport {
  totalSegments: number;
  okCount: number;
  errorCount: number;
  criticalCount: number;
  errorsByType: Record<string, number>;
  terminologyIssues: Array<{
    entryId: string;
    sourceTerm: string;
    expectedTargets: string[];
    severity: "critical" | "warning" | "info";
    occurrences: number;
  }>;
}

// ---------------------------------------------------------------------------
// Terminology library
// ---------------------------------------------------------------------------

export type PartOfSpeech =
  | "noun"
  | "proper_noun"
  | "verb"
  | "adjective"
  | "adverb"
  | "expression"
  | "unknown";
export type TerminologyEntryStatus = "active" | "ignored" | "archived";
export type TerminologyReviewStatus = "proposed" | "approved" | "locked";
export type TerminologyEnforcement = "contextual" | "preferred" | "required";

export interface SegmentTerminologyRule {
  entryId: string;
  source: string;
  target: string;
  semanticType: string;
  partOfSpeech: PartOfSpeech;
  reviewStatus: TerminologyReviewStatus;
  enforcement: TerminologyEnforcement;
  acceptedTargets: string[];
}

export interface TerminologyTranslation {
  id: string;
  targetLanguage: string;
  projectId: string | null;
  targetText: string;
  reviewStatus: TerminologyReviewStatus;
  enforcement: TerminologyEnforcement;
  confidence: number;
  providerId: string | null;
  model: string | null;
  acceptedVariants: string[];
}

export interface TerminologyContext {
  segmentId: string;
  surfaceText: string;
  sourceText: string;
  engineKind: string;
  occurrenceCount: number;
}

export interface TerminologyEntry {
  id: string;
  sourceLanguage: string;
  canonicalText: string;
  normalizedText: string;
  reading: string | null;
  partOfSpeech: PartOfSpeech;
  semanticType: string;
  senseKey: string;
  status: TerminologyEntryStatus;
  origin: "engine" | "lindera" | "manual" | "legacy_glossary" | "import";
  confidence: number;
  occurrenceCount: number;
  translation: TerminologyTranslation | null;
  contexts: TerminologyContext[];
}

export interface PaginatedTerminology {
  items: TerminologyEntry[];
  total: number;
  page: number;
  pageSize: number;
}

export interface TerminologyStats {
  totalEntries: number;
  untranslatedEntries: number;
  proposedTranslations: number;
  approvedTranslations: number;
  lockedTranslations: number;
}

export interface TerminologyScanProgress {
  scanId: string;
  projectId: string;
  processed: number;
  total: number;
  discovered: number;
}

export interface TerminologyScanDone {
  scanId: string;
  projectId: string;
  status: "completed" | "cancelled" | "failed";
  error: string | null;
}

// ---------------------------------------------------------------------------
// .h2s exchange pack (share/resume a translation)
// ---------------------------------------------------------------------------

export interface PackExportSummary {
  segmentCount: number;
  fileCount: number;
  glossaryCount: number;
  tmCount: number;
}

/** Hard reason why a pack cannot be imported (typed by the backend). */
export type ImportBlocker =
  | { type: "notAPack" }
  | { type: "unsupportedVersion"; version: number }
  | { type: "engineMismatch"; packEngine: string; projectEngine: string }
  | { type: "langPairMismatch"; packLangPair: string; projectLangPair: string }
  | { type: "invalidPack"; detail: string };

/** Dry-run result of `preview_h2s_import` — nothing has been written. */
export interface ImportPreview {
  blocker: ImportBlocker | null;
  packGameTitle: string;
  packAppVersion: string;
  packCreatedAt: string;
  titleMismatch: boolean;
  applicable: number;
  identical: number;
  conflicts: number;
  sourceChanged: number;
  orphans: number;
  glossaryCount: number;
  tmCount: number;
}

/** Collision policy of `apply_h2s_import`. */
export type ImportPolicy =
  | "fill"
  | "overwrite_except_reviewed"
  | "overwrite_all";

/** Final report of `apply_h2s_import` (also the `h2s://project/import-done` payload). */
export interface ImportReport {
  applied: number;
  appliedSourceChanged: number;
  skippedConflicts: number;
  skippedSourceChanged: number;
  identical: number;
  orphans: number;
  glossaryAdded: number;
  glossaryConflicts: number;
  tmAdded: number;
  backupPath: string;
}

// ---------------------------------------------------------------------------
// LLM
// ---------------------------------------------------------------------------

export type ResourceProfile = "eco" | "balanced" | "fast";

export interface ProviderConfig {
  providerId?: string;
  url: string;
  model: string;
  apiKey?: string;
  batchSize: number;
  resourceProfile: ResourceProfile;
}

export type ProviderTask = "translate" | "chat";

/** One logical provider call, including local timing and optional API usage. */
export interface ProviderCallMetrics {
  task: ProviderTask;
  model: string;
  inputUnits: number;
  /** Occurrence-scoped terminology hints sent in this exact request. */
  terminologyHints: number;
  promptChars: number;
  promptTokens: number | null;
  completionTokens: number | null;
  totalTokens: number | null;
  durationMs: number;
  attempts: number;
  success: boolean;
}

export interface PipelineBatchMetrics {
  responseFormatRetries: number;
  placeholderRetries: number;
  recursiveSplits: number;
  semanticRejections: number;
}

export type PilotSampleCategory =
  | "dialogue"
  | "choice_branch"
  | "database"
  | "names_ui"
  | "placeholder_multiline"
  | "uncertain";

export interface PilotNeighborLine {
  segmentKind: string;
  speaker: string | null;
  text: string;
}

export interface PilotSegmentContext {
  segmentKind: string;
  sceneId: string | null;
  speaker: string | null;
  branchPath: string | null;
  previous: PilotNeighborLine[];
  following: PilotNeighborLine[];
}

export interface PilotSampleSegment {
  stableKey: string;
  fileName: string;
  jsonKey: string;
  sourceText: string;
  segmentKind: string;
  sceneId: string | null;
  speaker: string | null;
  branchPath: string | null;
  category: PilotSampleCategory;
  hasPlaceholders: boolean;
  context: PilotSegmentContext | null;
}

export interface PilotIsolation {
  personalDatabaseAccessed: boolean;
  gameFilesWritten: number;
  databaseRemoved: boolean;
}

/** Extraction-only pilot preparation; no provider has been contacted yet. */
export interface PilotPreparation {
  engine: "mv_mz";
  sourceLanguage: string;
  targetLanguage: string;
  totalFiles: number;
  totalSegments: number;
  sampleSize: number;
  byKind: Record<string, number>;
  byCategory: Partial<Record<PilotSampleCategory, number>>;
  sample: PilotSampleSegment[];
  isolation: PilotIsolation;
}

export type PilotPhase =
  | "preparing"
  | "baseline"
  | "contextual"
  | "quality_review";

export interface PilotProgressPayload {
  phase: PilotPhase;
  done: number;
  total: number;
}

export type PilotQualityFlag =
  | "empty_translation"
  | "source_script_remaining"
  | "context_leak";

export interface PilotVariantResult {
  translatedText: string;
  qa: QaResult;
  qualityFlags: PilotQualityFlag[];
  needsReview: boolean;
  fromTm: boolean;
}

export interface PilotComparison {
  segment: PilotSampleSegment;
  baseline: PilotVariantResult;
  contextual: PilotVariantResult;
  changed: boolean;
  contextAvailable: boolean;
}

export interface PilotMetricsSummary {
  calls: ProviderCallMetrics[];
  requestCount: number;
  inputUnits: number;
  promptChars: number;
  promptTokens: number | null;
  completionTokens: number | null;
  totalTokens: number | null;
  durationMs: number;
  attempts: number;
}

export interface PilotVariantSummary {
  metrics: PilotMetricsSummary;
  averageQaScore: number;
  needsReviewCount: number;
}

export interface PilotRunReport {
  preparation: PilotPreparation;
  comparisons: PilotComparison[];
  baseline: PilotVariantSummary;
  contextual: PilotVariantSummary;
  changedCount: number;
}
