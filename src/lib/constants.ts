// RPG Maker MV/MZ placeholder pattern — single source of truth.
// Used in columns.tsx (buildHighlightedNodes).
// Reset lastIndex before each exec() call since the flag /g is stateful.
export const PH_RE_SOURCE =
  /\\[+\-]\w+\[\d+\]|\\[VNPCI]\[\d+\]|\\[G\\$.|!><^{}]|\[%\d+\]/g;

/** Returns a fresh RegExp clone so concurrent callers don't share lastIndex. */
export function clonePH_RE(): RegExp {
  return new RegExp(PH_RE_SOURCE.source, PH_RE_SOURCE.flags);
}

// Wolf RPG placeholder pattern — mirrors `RE_WOLF` in
// src-tauri/src/engines/wolf/placeholders.rs (same alternative order,
// minus the trailing literal-newline alternative which has no visual chip).
export const PH_RE_WOLF =
  /\\r\[[^[\],]+,[^[\]]*\]|\\(?:udb|cdb|sdb)\[\d+:\d+:\d+\]|\\sysS\[\d+\]|\\cself\[\d{1,2}\]|\\self\[\d\]|\\sys\[\d+\]|\\space\[\d+\]|\\v\?\[\d+\]|\\(?:sp|mx|my|ax|ay)\[\d+\]|\\-\[\d+\]|\\font\[\d\]|\\[vcsfiVCSFI]\[\d+\]|\\m\[\d+\]|<[LCR]>|\\A[+-]|\\[EN\\!.^><]/g;

/** Returns a fresh RegExp clone so concurrent callers don't share lastIndex. */
export function clonePH_RE_WOLF(): RegExp {
  return new RegExp(PH_RE_WOLF.source, PH_RE_WOLF.flags);
}

/** Returns a fresh placeholder regex matching the given project engine. */
export function getPlaceholderRegex(engine: string): RegExp {
  return engine === "wolf" ? clonePH_RE_WOLF() : clonePH_RE();
}
