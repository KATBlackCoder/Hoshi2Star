import { describe, expect, it } from "vitest";
import {
  hasDebugExport,
  hasPilot,
  hasTextCoverage,
  hasTextFit,
  hasTextFitPreview,
} from "./engine";

// Plan DD, L7-2 (F3): each predicate reproduces the engine test it replaced.
const ENGINES = ["mv_mz", "wolf", "vx_ace", "bakin", "", null, undefined];

function supported(predicate: (engine: string | null | undefined) => boolean) {
  return ENGINES.filter((engine) => predicate(engine));
}

describe("engine capabilities", () => {
  it("text coverage: MV/MZ only", () => {
    expect(supported(hasTextCoverage)).toEqual(["mv_mz"]);
  });

  it("text fitting: MZ and Wolf", () => {
    expect(supported(hasTextFit)).toEqual(["mv_mz", "wolf"]);
  });

  it("text fitting preview: MZ only", () => {
    expect(supported(hasTextFitPreview)).toEqual(["mv_mz"]);
  });

  it("pilot: MV/MZ only", () => {
    expect(supported(hasPilot)).toEqual(["mv_mz"]);
  });

  it("debug export: MV/MZ and Wolf", () => {
    expect(supported(hasDebugExport)).toEqual(["mv_mz", "wolf"]);
  });
});
