import { beforeEach, describe, expect, it } from "vitest";
import { render, screen } from "@testing-library/react";
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
  it("shows ✓ N · ⚠ M while needs_review remains", () => {
    useProjectStore.setState({
      sourceFiles: [
        makeFile({ translatedCount: 5, needsReviewCount: 2, totalCount: 7 }),
      ],
    });
    render(<FileTree />);

    expect(screen.getByText("✓ 5")).toBeInTheDocument();
    expect(screen.getByText("⚠ 2")).toBeInTheDocument();
  });

  it("shows the translated count when every segment is translated", () => {
    useProjectStore.setState({
      sourceFiles: [
        makeFile({ translatedCount: 7, needsReviewCount: 0, totalCount: 7 }),
      ],
    });
    render(<FileTree />);

    expect(screen.getByText("✓ 7")).toBeInTheDocument();
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
