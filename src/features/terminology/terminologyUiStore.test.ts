import { beforeEach, describe, expect, it } from "vitest";
import { useTerminologyUiStore } from "./terminologyUiStore";

beforeEach(() =>
  useTerminologyUiStore.setState({
    search: "",
    partOfSpeech: "all",
    semanticType: "",
    status: "active",
    scope: "project",
    page: 0,
    selectedIds: [],
  }),
);

describe("terminology UI store", () => {
  it("stores filters and selection but no server entries", () => {
    const state = useTerminologyUiStore.getState();
    state.setSearch("勇者");
    state.setPartOfSpeech("noun");
    state.toggleSelected("e1");
    expect(useTerminologyUiStore.getState()).toMatchObject({
      search: "勇者",
      partOfSpeech: "noun",
      selectedIds: ["e1"],
      page: 0,
    });
    expect(useTerminologyUiStore.getState()).not.toHaveProperty("entries");
  });

  it("resets pagination when scope or filters change", () => {
    useTerminologyUiStore.getState().setPage(4);
    useTerminologyUiStore.getState().setScope("global");
    expect(useTerminologyUiStore.getState()).toMatchObject({
      page: 0,
      scope: "global",
    });
  });
});
