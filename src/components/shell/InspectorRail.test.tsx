import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { InspectorRail } from "@/components/shell/InspectorRail";
import { useUiStore } from "@/stores/ui";

vi.mock("@/components/editor/TMPanel", () => ({
  TMPanel: () => <p>Mémoire test</p>,
}));
vi.mock("@/components/editor/QAPanel", () => ({
  QAPanel: () => <p>QA test</p>,
}));
vi.mock("@/components/editor/TerminologyInspector", () => ({
  TerminologyInspector: () => <p>Terminologie test</p>,
}));

beforeEach(() => {
  useUiStore.setState({ inspectorOpen: true, inspectorTab: "qa" });
});

describe("InspectorRail", () => {
  const props = {
    projectId: "p1",
    langPair: "ja-fr",
    sourceText: "星",
    targetText: "étoile",
  };

  it("shows one accessible tool panel at a time", async () => {
    render(<InspectorRail {...props} />);

    expect(screen.getByRole("tab", { name: "QA" })).toHaveAttribute(
      "aria-selected",
      "true",
    );
    expect(screen.getByRole("tabpanel")).toHaveTextContent("QA test");

    await userEvent.click(screen.getByRole("tab", { name: "Terminologie" }));
    expect(screen.getByRole("tabpanel")).toHaveTextContent("Terminologie test");
    expect(useUiStore.getState().inspectorTab).toBe("terminology");
  });

  it("collapses and reopens directly on the requested tool", async () => {
    render(<InspectorRail {...props} />);

    await userEvent.click(
      screen.getByRole("button", { name: "Fermer l’inspecteur" }),
    );
    expect(useUiStore.getState().inspectorOpen).toBe(false);
    expect(screen.queryByRole("tablist")).not.toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "Mémoire" }));
    expect(useUiStore.getState()).toMatchObject({
      inspectorOpen: true,
      inspectorTab: "tm",
    });
    expect(screen.getByRole("tabpanel")).toHaveTextContent("Mémoire test");
  });
});
