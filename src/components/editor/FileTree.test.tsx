import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
import i18n from "@/lib/i18n";
import { FileTree } from "@/components/editor/FileTree";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import type { SourceFile } from "@/lib/types";

const initialProject = useProjectStore.getState();
const initialEditor = useEditorStore.getState();

function makeFile(over: Partial<SourceFile>): SourceFile {
  return {
    id: "f1",
    projectId: "p1",
    fileName: "Map001.json",
    filePath: "/tmp/Map001.json",
    fileType: "map",
    translationSecs: null,
    translatedCount: 0,
    needsReviewCount: 0,
    totalCount: 0,
    ...over,
  };
}

beforeEach(() => {
  useProjectStore.setState(initialProject, true);
  useEditorStore.setState(initialEditor, true);
});

describe("FileTree — Option 2 counters (Phase 5)", () => {
  it("shows ✓ N · ⚠ M and hides the inject button while needs_review remains", () => {
    useProjectStore.setState({
      sourceFiles: [
        makeFile({ translatedCount: 5, needsReviewCount: 2, totalCount: 7 }),
      ],
    });
    render(<FileTree />);

    expect(screen.getByText("✓ 5")).toBeInTheDocument();
    expect(screen.getByText("⚠ 2")).toBeInTheDocument();
    // Not strictly complete → no debug-inject button.
    expect(
      screen.queryByTitle(i18n.t("fileTree.debugInject")),
    ).not.toBeInTheDocument();
  });

  it("shows the inject button only when every segment is strictly translated", () => {
    useProjectStore.setState({
      sourceFiles: [
        makeFile({ translatedCount: 7, needsReviewCount: 0, totalCount: 7 }),
      ],
    });
    render(<FileTree />);

    expect(screen.getByText("✓ 7")).toBeInTheDocument();
    expect(
      screen.getByTitle(i18n.t("fileTree.debugInject")),
    ).toBeInTheDocument();
  });

  it("shows no counters on an untouched file", () => {
    useProjectStore.setState({
      sourceFiles: [makeFile({ totalCount: 7 })],
    });
    render(<FileTree />);

    expect(screen.queryByText(/✓/)).not.toBeInTheDocument();
    expect(screen.queryByText(/⚠/)).not.toBeInTheDocument();
  });
});
