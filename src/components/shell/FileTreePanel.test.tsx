import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";
import { FileTreePanel } from "@/components/shell/FileTreePanel";
import i18n from "@/lib/i18n";

vi.mock("@/components/editor/FileTree", () => ({
  FileTree: () => <p>Arbre test</p>,
}));
vi.mock("@/components/editor/ProjectSearchBar", () => ({
  ProjectSearchBar: () => <p>Recherche test</p>,
}));

describe("FileTreePanel", () => {
  it("hides its content when collapsed and exposes an expand action", async () => {
    const onToggle = vi.fn();
    const { rerender } = render(
      <FileTreePanel expanded activeProjectId="p1" onToggle={onToggle} />,
    );

    expect(screen.getByText("Arbre test")).toBeInTheDocument();
    await userEvent.click(
      screen.getByRole("button", { name: i18n.t("fileTree.collapse") }),
    );
    expect(onToggle).toHaveBeenCalledOnce();

    rerender(
      <FileTreePanel
        expanded={false}
        activeProjectId="p1"
        onToggle={onToggle}
      />,
    );
    expect(screen.queryByText("Arbre test")).not.toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: i18n.t("fileTree.expand") }),
    ).toBeInTheDocument();
  });
});
