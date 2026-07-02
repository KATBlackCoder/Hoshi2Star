/**
 * Shared glyph + text colour for per-status count summaries — the "✓ N ⚠ M"
 * chips rendered in the FileTree rows and the ProjectList stats bar.
 *
 * This is deliberately distinct from `STATUS_STYLES` (the segment-grid status
 * badge, which uses a cyan/gold glow palette): the summary chips use the flatter
 * green/blue/amber convention. Both call sites read the classes from here so the
 * green ✓ and amber ⚠ can't drift apart. Classes are kept verbatim.
 */
export const STATUS_SUMMARY = {
  translated: { glyph: "✓", className: "text-green-400/80" },
  reviewed: { glyph: "◎", className: "text-blue-400/80" },
  needsReview: { glyph: "⚠", className: "text-amber-400/80" },
  untranslated: { glyph: "○", className: "" },
} as const;
