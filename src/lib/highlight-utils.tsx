/** Escape special regex characters in a string. */
function escapeRe(s: string) {
  return s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

/**
 * Split `text` by glossary term sources (sorted by length desc), then apply
 * placeholder highlight to each plain-text segment.
 * Returns a flat array of ReactNode ready to render inside a `<p>`.
 *
 * @param phRe - Placeholder regex with global flag. A fresh instance is used
 *               per chunk so callers do not need to reset lastIndex.
 */
export function buildHighlightedNodes(
  text: string,
  glossaryTerms: string[],
  phRe: RegExp,
): React.ReactNode[] {
  const nodes: React.ReactNode[] = [];

  function applyPlaceholders(chunk: string, keyBase: string) {
    const parts: React.ReactNode[] = [];
    const re = new RegExp(phRe.source, phRe.flags);
    let last = 0;
    let m: RegExpExecArray | null;
    while ((m = re.exec(chunk)) !== null) {
      if (m.index > last) parts.push(chunk.slice(last, m.index));
      parts.push(
        <mark
          key={`${keyBase}-ph-${m.index}`}
          className="rounded border border-cyan-600/30 bg-cyan-500/10 px-1 font-mono text-[0.85em] text-cyan-700 dark:border-cyan-300/25 dark:text-cyan-300"
        >
          {m[0]}
        </mark>,
      );
      last = m.index + m[0].length;
    }
    if (last < chunk.length) parts.push(chunk.slice(last));
    return parts;
  }

  if (glossaryTerms.length === 0) {
    return applyPlaceholders(text, "0");
  }

  // Sort terms by length descending to avoid shorter substrings matching first
  const sorted = [...glossaryTerms].sort((a, b) => b.length - a.length);
  const pattern = sorted.map(escapeRe).join("|");
  const glossaryRe = new RegExp(`(${pattern})`, "g");

  let last = 0;
  let m: RegExpExecArray | null;
  glossaryRe.lastIndex = 0;

  while ((m = glossaryRe.exec(text)) !== null) {
    if (m.index > last) {
      nodes.push(
        ...applyPlaceholders(text.slice(last, m.index), `pre-${m.index}`),
      );
    }
    nodes.push(
      <mark
        key={`g-${m.index}`}
        className="border-b border-dashed border-star bg-transparent text-star"
      >
        {m[0]}
      </mark>,
    );
    last = glossaryRe.lastIndex;
  }
  if (last < text.length) {
    nodes.push(...applyPlaceholders(text.slice(last), `tail`));
  }

  return nodes;
}
