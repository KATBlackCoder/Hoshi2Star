import { describe, expect, it } from "vitest";
import { QA_SCORE_CRITICAL_BELOW, qaScoreLevel } from "@/lib/qaScore";

describe("qaScoreLevel (audit F1: one source for the QA thresholds)", () => {
  it("is ok only at 100, critical below the report's cut", () => {
    expect(QA_SCORE_CRITICAL_BELOW).toBe(70);
    expect(qaScoreLevel(100)).toBe("ok");
    expect(qaScoreLevel(99)).toBe("warning");
    expect(qaScoreLevel(70)).toBe("warning");
    expect(qaScoreLevel(69)).toBe("critical");
    expect(qaScoreLevel(0)).toBe("critical");
  });
});
