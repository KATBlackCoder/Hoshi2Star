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
