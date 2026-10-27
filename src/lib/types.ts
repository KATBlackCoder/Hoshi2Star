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

/** A line of engine or plugin code that reads the game's language setting. */
export interface LocaleUse {
  location: string;
  code: string;
}

/** Optional switch of System.json `locale` to the target language (MV 1.6+ / MZ). */
export interface LocaleOption {
  current: string;
  target: string;
  consequences: string[];
  uses: LocaleUse[];
}

/** What a Wolf RPG export will do (`preflight_export`, plan RW). */
export interface WolfExportPreflight {
  /** Archives the game reads first, delivered rebuilt (`Data.wolf`…). */
  archives: string[];
  /** Wolf 2.x (Shift-JIS) game: accents are folded (é → e). */
  shiftJis: boolean;
  /** Translations holding characters Shift-JIS cannot encode. */
  accentedTexts: number;
  /** The game folder path a Wolf 2.x game cannot start from. */
  pathWarning: string | null;
}

/** Options offered before an export (`preflight_export`). */
export interface ExportPreflight {
  /** `null` when the game has no language setting or it already matches. */
  locale: LocaleOption | null;
  /** Wolf RPG projects only. */
  wolf?: WolfExportPreflight | null;
}

export interface ProjectStats {
  fileCount: number;
  totalSegments: number;
  untranslatedCount: number;
  translatedCount: number;
  needsReviewCount: number;
  /** With the other three counters, the four statuses sum to totalSegments. */
  reviewedCount: number;
  /** Segments "Translate all" will pick up, legacy source fallbacks included. */
  resumableCount: number;
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
  /** Optional text category (`null` = core extraction). */
  textCategory?: string | null;
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
    }
  | { type: "constraint_violation"; rule: string }
  | { type: "missing_format_specifier"; placeholder: string };

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
export type TerminologyEnforcement = "contextual" | "preferred" | "required";

export interface SegmentTerminologyRule {
  entryId: string;
  source: string;
  target: string;
  semanticType: string;
  partOfSpeech: PartOfSpeech;
  enforcement: TerminologyEnforcement;
  acceptedTargets: string[];
}

export interface TerminologyTranslation {
  id: string;
  targetLanguage: string;
  projectId: string | null;
  targetText: string;
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
  hasProjectTranslation: boolean;
  hasGlobalTranslation: boolean;
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
  projectTranslations: number;
  globalTranslations: number;
}

export interface GlobalizeTranslationsSummary {
  copied: number;
  alreadyGlobal: number;
  conflicts: number;
  skipped: number;
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
  terminologyCount: number;
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
  terminologyCount: number;
  terminologyCreates: number;
  terminologyUpdates: number;
  terminologyConflicts: number;
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
  terminologyAdded: number;
  terminologyUpdated: number;
  terminologyConflicts: number;
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
  semanticRetries: number;
  semanticRecoveries: number;
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

// ---------------------------------------------------------------------------
// Optional text coverage (MV/MZ plugin & system UI text)
// ---------------------------------------------------------------------------

export type CategoryState = "verified" | "linked" | "blocked";

/** `get_text_coverage_status`: answered without scanning the game. */
export interface CoverageStatus {
  supported: boolean;
  hasDecisions: boolean;
}
export type CategoryGroup =
  | "system"
  | "map"
  | "plugin"
  | "plugin_command"
  | "note_tag";
export type ReferenceSite =
  | "note"
  | "script"
  | "plugin_command"
  | "plugin_command_arg"
  | "plugin_param"
  | "plugin_source";

export interface CoverageReference {
  text: string;
  site: ReferenceSite;
  location: string;
  excerpt: string;
}

export interface CoverageConstraint {
  rule: string;
}

export interface CoverageCategory {
  key: string;
  title: string;
  group: CategoryGroup;
  state: CategoryState;
  enabled: boolean;
  decided: boolean;
  defaultEnabled: boolean;
  visibleIn: string;
  consequences: string[];
  constraints: CoverageConstraint[];
  segmentCount: number;
  translatedCount: number;
  samples: string[];
  references: CoverageReference[];
  referenceCount: number;
  blockedReason: string | null;
}

export interface CoverageCoreWarning {
  segmentKind: string;
  source: string;
  references: CoverageReference[];
}

export interface CoverageNotCovered {
  plugin: string;
  field: string;
  sample: string;
  reason: string;
  hint: string | null;
}

export interface CoverageView {
  supported: boolean;
  hasDecisions: boolean;
  reconstructed: boolean;
  driftedFiles: { file: string; pointers: number }[];
  pluginsUnreadable: string | null;
  categories: CoverageCategory[];
  coreWarnings: CoverageCoreWarning[];
  notCovered: CoverageNotCovered[];
}

export interface CategoryChange {
  categoryKey: string;
  enabled: boolean;
  acceptConsequences?: boolean;
}

// ---------------------------------------------------------------------------
// Text fitting (RPG Maker MZ)
// ---------------------------------------------------------------------------

export type LineBreaks = "reflow" | "keep";
export type FitOverflow = "new_page" | "shrink" | "leave";

export interface TextFitSettings {
  enabledDialogue: boolean;
  enabledDescriptions: boolean;
  lineBreaks: LineBreaks;
  overflow: FitOverflow;
  minFontSize: number;
  /** Dialogue font size emitted at every page start (`null` = the game's). */
  dialogueFontSize: number | null;
}

export interface TextFitGeometry {
  fontSize: number;
  fontFile: string | null;
  boxWidth: number;
  lineHeight: number;
  message: { innerWidth: number; innerHeight: number };
  help: { textX: number; textWidth: number; innerHeight: number };
}

export interface TextFitFont {
  file: string | null;
  /** Widths come from the game's font file (otherwise estimated). */
  loaded: boolean;
  error: string | null;
}

export interface TextFitLayoutWarning {
  plugin: string;
  hooks: string[];
}

export interface TextFitView {
  supported: boolean;
  reason: string | null;
  settings: TextFitSettings;
  geometry: TextFitGeometry | null;
  font: TextFitFont | null;
  warnings: TextFitLayoutWarning[];
  notes: string[];
}

export type FitOutcome =
  | { kind: "fits" }
  | { kind: "rewrapped" }
  | { kind: "newPage"; pages: number }
  | { kind: "shrunk"; size: number }
  | { kind: "tooLong"; reason: string }
  | { kind: "skipped"; reason: string };

export interface FitCounts {
  total: number;
  fits: number;
  rewrapped: number;
  newPage: number;
  shrunk: number;
  tooLong: number;
  skipped: number;
}

export interface FitRow {
  fileId: string;
  fileName: string;
  pointer: string;
  description: boolean;
  segmentId: string | null;
  outcome: FitOutcome;
  sample: string;
}

export interface FitScan {
  dialogue: FitCounts;
  descriptions: FitCounts;
  rows: FitRow[];
  /** Characters the game font lacks (drawn with a fallback font). */
  missingChars: Array<{ character: string; count: number }>;
}

export interface FitRunView {
  text: string;
  x: number;
  width: number;
  fontSize: number;
  color: number;
  icon: boolean;
}

export interface FitLineView {
  runs: FitRunView[];
  y: number;
  height: number;
  right: number;
  overflow: boolean;
  clipped: boolean;
}

export interface FitPageView {
  lines: FitLineView[];
}

export interface FitOverride {
  lineBreaks: LineBreaks | null;
  overflow: FitOverflow | null;
  disabled: boolean;
}

export interface FitItem {
  pointer: string;
  description: boolean;
  face: boolean;
  lines: string[];
  segmentIds: Array<string | null>;
  texts: string[];
  sourceHash: string;
  override: FitOverride | null;
  fitted: {
    outcome: FitOutcome;
    values: Array<string | null>;
    pages: FitPageView[];
  };
}

export interface TextFitPreview {
  fileName: string;
  item: FitItem;
  /** Fitting is on for this kind of text (otherwise: what it would do). */
  enabled: boolean;
  innerWidth: number;
  innerHeight: number;
  textX: number;
  limit: number;
  fontSize: number;
}

// ---------------------------------------------------------------------------
// Realignment after a game update (commands/resync.rs)
// ---------------------------------------------------------------------------

export interface ResyncFile {
  file: string;
  moved: number;
  added: number;
  gone: number;
  changedSources: number;
  orphaned: number;
  newFile: boolean;
}

export interface GoneTranslation {
  file: string;
  key: string;
  source: string;
  target: string;
}

export interface ResyncSummary {
  /** `false` for a preview: nothing was written. */
  applied: boolean;
  files: ResyncFile[];
  moved: number;
  added: number;
  gone: number;
  changedSources: number;
  orphaned: number;
  goneTranslations: GoneTranslation[];
  /** Enabled categories no longer offered (or now blocked): left as they are. */
  untouchedCategories: string[];
}
