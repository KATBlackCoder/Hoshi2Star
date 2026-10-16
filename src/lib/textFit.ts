import type { TFunction } from "i18next";
import type { FitOutcome } from "@/lib/types";

/** Human-readable outcome of fitting one text. */
export function outcomeLabel(t: TFunction, outcome: FitOutcome): string {
  switch (outcome.kind) {
    case "newPage":
      return t("textFit.outcome.newPage", { pages: outcome.pages });
    case "shrunk":
      return t("textFit.outcome.shrunk", { size: outcome.size });
    case "tooLong":
      return t("textFit.outcome.tooLong", { reason: outcome.reason });
    case "skipped":
      return t("textFit.outcome.skipped", { reason: outcome.reason });
    default:
      return t(`textFit.outcome.${outcome.kind}`);
  }
}
