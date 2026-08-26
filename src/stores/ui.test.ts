import { beforeEach, describe, expect, it } from "vitest";
import { useUiStore } from "@/stores/ui";

beforeEach(() => {
  localStorage.removeItem("hoshi2star-ui");
  useUiStore.setState({
    mode: "library",
    inspectorOpen: true,
    inspectorTab: "qa",
    fileTreeOpen: true,
    gridDensity: "comfortable",
  });
});

describe("ui store", () => {
  it("switches workspace mode", () => {
    useUiStore.getState().setMode("images");
    expect(useUiStore.getState().mode).toBe("images");
  });

  it("keeps inspector state independent from the mode", () => {
    useUiStore.getState().toggleInspector();
    useUiStore.getState().setInspectorTab("glossary");
    expect(useUiStore.getState()).toMatchObject({
      inspectorOpen: false,
      inspectorTab: "glossary",
    });
  });

  it("remembers patch workspace layout preferences", () => {
    useUiStore.getState().toggleFileTree();
    useUiStore.getState().setGridDensity("compact");

    expect(useUiStore.getState()).toMatchObject({
      fileTreeOpen: false,
      gridDensity: "compact",
    });
    expect(localStorage.getItem("hoshi2star-ui")).toContain('"compact"');
  });
});
