/**
 * One segment whose original value in the game no longer matches the source
 * recorded at extraction: the game changed since (an update). Rendered from
 * `IntegrityIssue::SourceChanged` in `engines/mv_mz/integrity.rs`, whose line
 * format this parser follows:
 * `<file> <pointer>: source changed since extraction; expected <json>, found <json|nothing>`.
 */
export interface SourceChange {
  file: string;
  pointer: string;
  expected: string;
  /** `null` when the location no longer exists in the game file. */
  found: string | null;
}

const JSON_STRING = String.raw`"(?:[^"\\]|\\.)*"`;
const SOURCE_CHANGE_LINE = new RegExp(
  String.raw`^(.+?) (\/.*?): source changed since extraction; expected (${JSON_STRING}), found (${JSON_STRING}|nothing)$`,
);

export function parseSourceChange(line: string): SourceChange | null {
  const match = SOURCE_CHANGE_LINE.exec(line);
  if (!match) return null;
  try {
    return {
      file: match[1],
      pointer: match[2],
      expected: JSON.parse(match[3]) as string,
      found: match[4] === "nothing" ? null : (JSON.parse(match[4]) as string),
    };
  } catch {
    return null;
  }
}
