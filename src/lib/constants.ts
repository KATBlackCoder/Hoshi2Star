// RPG Maker MV/MZ placeholder pattern — single source of truth.
// Used in App.tsx (HighlightedSource) and columns.tsx (buildHighlightedNodes).
// Reset lastIndex before each exec() call since the flag /g is stateful.
export const PH_RE_SOURCE =
  /\\[+\-]\w+\[\d+\]|\\[VNPCI]\[\d+\]|\\[G\\$.|!><^{}]|\[%\d+\]/g;

/** Returns a fresh RegExp clone so concurrent callers don't share lastIndex. */
export function clonePH_RE(): RegExp {
  return new RegExp(PH_RE_SOURCE.source, PH_RE_SOURCE.flags);
}
