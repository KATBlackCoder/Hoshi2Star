import { invoke } from "@tauri-apps/api/core";
import type { QaReport, QaResult, SegmentTerminologyRule } from "@/lib/types";

/** QA of one segment or of a project, and the terminology rules it checks. */
export const qaApi = {
  checkSegment: (args: {
    sourceText: string;
    targetText: string;
    projectId: string | null;
    segmentId: string | null;
  }) => invoke<QaResult>("qa_check_segment", args),
  report: (projectId: string) =>
    invoke<QaReport>("get_qa_report", { projectId }),
  segmentTerminology: (args: { segmentId: string; targetLanguage: string }) =>
    invoke<SegmentTerminologyRule[]>("get_segment_terminology", args),
};
