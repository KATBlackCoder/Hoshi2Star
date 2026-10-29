/**
 * QA score thresholds — the single source for every screen that colours or
 * filters a segment by its score (grid column and filters, search results,
 * inspector ring). The score is 100 minus the penalties of the findings
 * (`core/qa.rs`).
 */

/** Below this, a score is critical: the cut of the HTML QA report (`core/report.rs`). */
export const QA_SCORE_CRITICAL_BELOW = 70;

/** `ok`: no finding (100). `warning`: findings above the critical cut. */
export type QaScoreLevel = "ok" | "warning" | "critical";

export function qaScoreLevel(score: number): QaScoreLevel {
  if (score >= 100) return "ok";
  return score >= QA_SCORE_CRITICAL_BELOW ? "warning" : "critical";
}
