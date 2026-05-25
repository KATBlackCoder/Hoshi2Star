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
  createdAt: string;
  updatedAt: string;
}

export interface SourceFile {
  id: string;
  projectId: string;
  fileName: string;
  filePath: string;
  fileType: string;
}

export interface Segment {
  id: string;
  sourceFileId: string;
  jsonKey: string;
  sourceText: string;
  targetText: string;
  status: SegmentStatus;
  qaScore: number | null;
  createdAt: string;
  updatedAt: string;
}

export interface PaginatedSegments {
  items: Segment[];
  total: number;
  page: number;
  pageSize: number;
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

// ---------------------------------------------------------------------------
// QA
// ---------------------------------------------------------------------------

export type QaErrorType =
  | { type: "missing_placeholder"; placeholder: string }
  | { type: "line_too_long"; line: number; length: number; max: number }
  | { type: "bom_detected" };

export interface QaResult {
  score: number;
  errors: QaErrorType[];
}

export interface QaReport {
  totalSegments: number;
  okCount: number;
  errorCount: number;
  errorsByType: Record<string, number>;
}

// ---------------------------------------------------------------------------
// LLM
// ---------------------------------------------------------------------------

export interface ProviderConfig {
  url: string;
  model: string;
  apiKey?: string;
}
