import { useCallback, useEffect, useRef } from "react";
import { useTauriEvent } from "@/hooks/useTauriEvent";
import { cancelFrame, EVENTS, scheduleFrame } from "@/lib/events";
import type { SegmentUpdate } from "@/lib/types";

/**
 * Coalesce the `segments-updated` batches of a run: the updates received
 * during a frame are applied once, by `apply`, at the next frame (latest
 * update per segment). Returns `drop`, which forgets the pending updates;
 * they are also dropped on unmount.
 */
export function useCoalescedSegmentUpdates(
  apply: (batch: ReadonlyMap<string, SegmentUpdate>) => void,
): () => void {
  const pendingRef = useRef(new Map<string, SegmentUpdate>());
  const frameRef = useRef<number | null>(null);

  const drop = useCallback(() => {
    pendingRef.current.clear();
    if (frameRef.current !== null) {
      cancelFrame(frameRef.current);
      frameRef.current = null;
    }
  }, []);

  useTauriEvent<SegmentUpdate[]>(EVENTS.llmSegmentsUpdated, (updates) => {
    const pending = pendingRef.current;
    for (const update of updates) pending.set(update.id, update);
    if (frameRef.current !== null || pending.size === 0) return;
    frameRef.current = scheduleFrame(() => {
      frameRef.current = null;
      if (pending.size === 0) return;
      const batch = new Map(pending);
      pending.clear();
      apply(batch);
    });
  });
  useEffect(() => drop, [drop]);

  return drop;
}
