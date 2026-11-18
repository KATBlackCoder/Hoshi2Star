import type { Segment, SegmentUpdate } from "@/lib/types";

/**
 * Patch LLM updates into a list of segments (or search hits). Returns the
 * same array when no row is touched, so callers can skip the state update.
 */
export function applySegmentUpdates<T extends Segment>(
  segments: T[],
  updates: ReadonlyMap<string, SegmentUpdate>,
): T[] {
  if (updates.size === 0) return segments;
  let touched = false;
  const next = segments.map((segment) => {
    const update = updates.get(segment.id);
    if (!update) return segment;
    touched = true;
    return { ...segment, targetText: update.targetText, status: update.status };
  });
  return touched ? next : segments;
}
