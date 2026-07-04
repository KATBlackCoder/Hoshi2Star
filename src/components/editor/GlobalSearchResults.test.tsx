import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import i18n from "@/lib/i18n";

// jsdom has no layout: replace the virtualizer with a render-everything stub.
vi.mock("@tanstack/react-virtual", () => ({
  useVirtualizer: (opts: { count: number }) => ({
    getVirtualItems: () =>
      Array.from({ length: opts.count }, (_, i) => ({
        index: i,
        key: i,
        start: i * 40,
      })),
    getTotalSize: () => opts.count * 40,
    measureElement: () => {},
    scrollToIndex: () => {},
  }),
}));

const toastError = vi.fn();
vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: (...args: unknown[]) => toastError(...args),
  },
}));

import { GlobalSearchResults } from "@/components/editor/GlobalSearchResults";
import { useEditorStore } from "@/stores/editor";
import { useLlmStore } from "@/stores/llm";
import { useProjectStore } from "@/stores/project";
import { useSearchStore } from "@/stores/search";
import type { SegmentSearchHit } from "@/lib/types";

const initialProject = useProjectStore.getState();
const initialEditor = useEditorStore.getState();
const initialSearch = useSearchStore.getState();

function makeHit(
  id: string,
  fileId: string,
  fileName: string,
): SegmentSearchHit {
  return {
    id,
    sourceFileId: fileId,
    jsonKey: `/k/${id}`,
    sourceText: `ソース ${id}`,
    targetText: `target ${id}`,
    status: "translated",
    qaScore: null,
    createdAt: "",
    updatedAt: "",
    fileName,
  };
}

// f1 hits are contiguous, then f2 — mirrors the SQL ORDER BY file_name.
const HITS = [
  makeHit("s1", "f1", "Actors.json"),
  makeHit("s2", "f1", "Actors.json"),
  makeHit("s3", "f2", "Map001.json"),
];

beforeEach(() => {
  toastError.mockClear();
  useProjectStore.setState(initialProject, true);
  useEditorStore.setState(initialEditor, true);
  useSearchStore.setState(initialSearch, true);
  useProjectStore.setState({ activeProjectId: "p1" });
  useSearchStore.setState({
    query: "hero",
    results: HITS,
    total: HITS.length,
    status: "idle",
    isActive: true,
  });
  mockIPC((cmd) => {
    switch (cmd) {
      case "plugin:event|listen":
        return 1;
      case "plugin:event|unlisten":
        return undefined;
      default:
        throw new Error(`unexpected command: ${cmd}`);
    }
  });
});

describe("GlobalSearchResults — grouping by file", () => {
  it("renders one header per file and hits grouped under it", () => {
    render(<GlobalSearchResults />);

    expect(screen.getByText("Actors.json")).toBeInTheDocument();
    expect(screen.getByText("Map001.json")).toBeInTheDocument();
    expect(screen.getByText("ソース s1")).toBeInTheDocument();
    expect(screen.getByText("ソース s2")).toBeInTheDocument();
    expect(screen.getByText("ソース s3")).toBeInTheDocument();

    // Per-file hit counts on the headers.
    expect(screen.getByText("(2)")).toBeInTheDocument();
    expect(screen.getByText("(1)")).toBeInTheDocument();
  });
});

describe("GlobalSearchResults — cross-file selection + batch translate", () => {
  it("translates the checked segment ids from different files", async () => {
    const startTranslation = vi.fn(async () => {});
    useLlmStore.setState({
      startTranslation,
      providerConfig: { url: "u", model: "m", batchSize: 1 },
    });
    const user = userEvent.setup();
    render(<GlobalSearchResults />);

    const checkboxes = screen.getAllByRole("checkbox");
    // Render order: [0] toolbar select-all, [1] header Actors.json, [2] s1,
    // [3] s2, [4] header Map001.json, [5] s3.
    await user.click(checkboxes[2]); // s1 (f1)
    await user.click(checkboxes[5]); // s3 (f2)

    await user.click(
      screen.getByText(i18n.t("segmentGrid.translateSelected", { count: 2 })),
    );

    expect(startTranslation).toHaveBeenCalledWith(["s1", "s3"], undefined);
  });

  it("file header checkbox selects/deselects all hits of that file (tri-state)", async () => {
    const startTranslation = vi.fn(async () => {});
    useLlmStore.setState({
      startTranslation,
      providerConfig: { url: "u", model: "m", batchSize: 1 },
    });
    const user = userEvent.setup();
    render(<GlobalSearchResults />);

    const actorsHeader = screen.getByTitle(
      i18n.t("projectSearch.selectFile", { fileName: "Actors.json" }),
    );

    // Check the Actors.json header → its 2 hits are selected, not Map001's.
    await user.click(actorsHeader);
    await user.click(
      screen.getByText(i18n.t("segmentGrid.translateSelected", { count: 2 })),
    );
    expect(startTranslation).toHaveBeenCalledWith(["s1", "s2"], undefined);

    // Selection was cleared by the translate action; check one hit only →
    // the header goes indeterminate (Radix: aria-checked="mixed"), then
    // header click completes the file.
    const checkboxes = screen.getAllByRole("checkbox");
    await user.click(checkboxes[2]); // s1 only
    expect(actorsHeader).toHaveAttribute("aria-checked", "mixed");
    await user.click(actorsHeader);
    expect(
      screen.getByText(i18n.t("segmentGrid.translateSelected", { count: 2 })),
    ).toBeInTheDocument();

    // Unchecking the header empties the file's selection entirely.
    await user.click(actorsHeader);
    expect(screen.queryByText(/Traduire/)).not.toBeInTheDocument();
  });

  it("select-all checks every hit across files", async () => {
    const startTranslation = vi.fn(async () => {});
    useLlmStore.setState({
      startTranslation,
      providerConfig: { url: "u", model: "m", batchSize: 1 },
    });
    const user = userEvent.setup();
    render(<GlobalSearchResults />);

    await user.click(screen.getByTitle(i18n.t("projectSearch.selectAll")));
    await user.click(
      screen.getByText(i18n.t("segmentGrid.translateSelected", { count: 3 })),
    );

    expect(startTranslation).toHaveBeenCalledWith(
      ["s1", "s2", "s3"],
      undefined,
    );
  });
});

describe("GlobalSearchResults — row click navigation", () => {
  it("activates the hit's file + segment and hides the results view", async () => {
    const user = userEvent.setup();
    render(<GlobalSearchResults />);

    await user.click(screen.getByText("ソース s3"));

    expect(useEditorStore.getState().activeFileId).toBe("f2");
    expect(useEditorStore.getState().activeSegmentId).toBe("s3");
    expect(useSearchStore.getState().isActive).toBe(false);
  });

  it("selection mode: with ≥1 checked, row click toggles instead of navigating", async () => {
    useLlmStore.setState({
      startTranslation: vi.fn(async () => {}),
      providerConfig: { url: "u", model: "m", batchSize: 1 },
    });
    const user = userEvent.setup();
    render(<GlobalSearchResults />);

    // Enter selection mode by checking s1.
    const checkboxes = screen.getAllByRole("checkbox");
    await user.click(checkboxes[2]); // s1

    // Clicking s3's TEXT now toggles it — no navigation.
    await user.click(screen.getByText("ソース s3"));
    expect(useEditorStore.getState().activeFileId).toBeNull();
    expect(useSearchStore.getState().isActive).toBe(true);
    expect(
      screen.getByText(i18n.t("segmentGrid.translateSelected", { count: 2 })),
    ).toBeInTheDocument();

    // Clicking it again deselects it.
    await user.click(screen.getByText("ソース s3"));
    expect(
      screen.getByText(i18n.t("segmentGrid.translateSelected", { count: 1 })),
    ).toBeInTheDocument();
  });
});

describe("GlobalSearchResults — collapse/expand per file", () => {
  it("collapses a file's hits (header and count stay), then expands them back", async () => {
    const user = userEvent.setup();
    render(<GlobalSearchResults />);

    await user.click(
      screen.getByTitle(
        i18n.t("projectSearch.collapseFile", { fileName: "Actors.json" }),
      ),
    );

    // Actors' hits hidden, header + count still there, Map001 untouched.
    expect(screen.queryByText("ソース s1")).not.toBeInTheDocument();
    expect(screen.queryByText("ソース s2")).not.toBeInTheDocument();
    expect(screen.getByText("ソース s3")).toBeInTheDocument();
    expect(screen.getByText("Actors.json")).toBeInTheDocument();
    expect(screen.getByText("(2)")).toBeInTheDocument();

    await user.click(
      screen.getByTitle(
        i18n.t("projectSearch.expandFile", { fileName: "Actors.json" }),
      ),
    );
    expect(screen.getByText("ソース s1")).toBeInTheDocument();
    expect(screen.getByText("ソース s2")).toBeInTheDocument();
  });

  it("collapsing does not touch the selection (header select still works)", async () => {
    useLlmStore.setState({
      startTranslation: vi.fn(async () => {}),
      providerConfig: { url: "u", model: "m", batchSize: 1 },
    });
    const user = userEvent.setup();
    render(<GlobalSearchResults />);

    // Select all of Actors.json, then collapse it: selection must survive.
    await user.click(
      screen.getByTitle(
        i18n.t("projectSearch.selectFile", { fileName: "Actors.json" }),
      ),
    );
    await user.click(
      screen.getByTitle(
        i18n.t("projectSearch.collapseFile", { fileName: "Actors.json" }),
      ),
    );
    expect(
      screen.getByText(i18n.t("segmentGrid.translateSelected", { count: 2 })),
    ).toBeInTheDocument();
  });
});

describe("GlobalSearchResults — background batch loading indicator", () => {
  it("shows loaded/total while batches are still arriving, hides once complete", () => {
    useSearchStore.setState({ total: 700 });
    const { rerender } = render(<GlobalSearchResults />);

    expect(
      screen.getByText(
        i18n.t("projectSearch.loadingProgress", { loaded: 3, total: 700 }),
      ),
    ).toBeInTheDocument();

    // All batches arrived → indicator gone.
    useSearchStore.setState({ total: HITS.length });
    rerender(<GlobalSearchResults />);
    expect(
      screen.queryByText(
        i18n.t("projectSearch.loadingProgress", { loaded: 3, total: 3 }),
      ),
    ).not.toBeInTheDocument();
  });
});
