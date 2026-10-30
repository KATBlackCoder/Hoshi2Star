/** Names of the events the backend emits (`app.emit` in `src-tauri/`). */
export const EVENTS = {
  llmProgress: "h2s://llm/progress",
  llmCompleted: "h2s://llm/completed",
  llmError: "h2s://llm/error",
  llmCooling: "h2s://llm/cooling",
  llmSegmentsUpdated: "h2s://llm/segments-updated",
  pilotProgress: "h2s://pilot/progress",
  projectImportDone: "h2s://project/import-done",
  projectResynced: "h2s://project/resynced",
  terminologyScanProgress: "h2s://terminology/scan-progress",
  terminologyScanDone: "h2s://terminology/scan-done",
  terminologyTranslateProgress: "h2s://terminology/translate-progress",
  terminologyTranslateDone: "h2s://terminology/translate-done",
} as const;

/** Run `callback` on the next frame (a 16 ms timer where frames are missing). */
export function scheduleFrame(callback: () => void): number {
  return typeof window.requestAnimationFrame === "function"
    ? window.requestAnimationFrame(callback)
    : window.setTimeout(callback, 16);
}

/** Cancel a callback planned by `scheduleFrame`. */
export function cancelFrame(handle: number): void {
  if (typeof window.cancelAnimationFrame === "function") {
    window.cancelAnimationFrame(handle);
  } else {
    window.clearTimeout(handle);
  }
}
