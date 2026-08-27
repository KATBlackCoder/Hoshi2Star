import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
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

import {
  applySegmentUpdates,
  SegmentGrid,
} from "@/components/editor/SegmentGrid";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { useLlmStore } from "@/stores/llm";
import { useUiStore } from "@/stores/ui";
import type { Segment } from "@/lib/types";

const initialProject = useProjectStore.getState();
const initialEditor = useEditorStore.getState();

const SEGMENTS: Segment[] = Array.from({ length: 5 }, (_, i) => ({
  id: `s${i + 1}`,
  sourceFileId: "f1",
  jsonKey: `/k/${i}`,
  sourceText: `ソース${i + 1}`,
  targetText: `target ${i + 1}`,
  status: "translated",
  qaScore: null,
  createdAt: "",
  updatedAt: "",
})) as Segment[];

/**
 * IPC mock: get_segments serves SEGMENTS in server-chosen chunks of 2
 * regardless of the requested pageSize — the loader must keep fetching
 * pages until `total` is reached (Phase 3 fix).
 */
function mockGridIpc(opts?: { failSave?: boolean; failFirstLoad?: boolean }) {
  const calls = { getSegments: 0 };
  mockIPC((cmd, args) => {
    switch (cmd) {
      case "get_segments": {
        calls.getSegments += 1;
        if (opts?.failFirstLoad && calls.getSegments === 1) {
          throw new Error("database unavailable");
        }
        const { page } = args as { page: number };
        const chunk = 2;
        return {
          items: SEGMENTS.slice(page * chunk, page * chunk + chunk),
          total: SEGMENTS.length,
          page,
          pageSize: chunk,
        };
      }
      case "update_segment": {
        if (opts?.failSave) throw new Error("disk full");
        const { id, targetText } = args as { id: string; targetText: string };
        const seg = SEGMENTS.find((s) => s.id === id)!;
        return { ...seg, targetText };
      }
      case "plugin:event|listen":
        return 1;
      case "plugin:event|unlisten":
        return undefined;
      default:
        throw new Error(`unexpected command: ${cmd}`);
    }
  });
  return calls;
}

beforeEach(() => {
  toastError.mockClear();
  useProjectStore.setState(initialProject, true);
  useEditorStore.setState(initialEditor, true);
  useProjectStore.setState({ activeProjectId: "p1" });
  useEditorStore.setState({ activeFileId: "f1" });
  useUiStore.setState({ gridDensity: "comfortable" });
});

describe("SegmentGrid — successive page loading (Phase 3)", () => {
  it("fetches pages until total and shows the real total in the footer", async () => {
    const calls = mockGridIpc();
    render(<SegmentGrid />);

    expect(
      await screen.findByText(i18n.t("segmentGrid.footer", { count: "5" })),
    ).toBeInTheDocument();
    // 5 segments served in chunks of 2 → 3 calls.
    expect(calls.getSegments).toBe(3);
    // Every row is present, not just the first page.
    expect(screen.getByDisplayValue("target 5")).toBeInTheDocument();
  });
});

describe("SegmentGrid — coalesced translation updates", () => {
  it("applies the latest update per stable segment id", () => {
    const updates = new Map([
      [
        "s1",
        { id: "s1", targetText: "first", status: "translated" as const },
      ],
      [
        "s3",
        { id: "s3", targetText: "third", status: "needs_review" as const },
      ],
    ]);

    const result = applySegmentUpdates(SEGMENTS, updates);

    expect(result.map((segment) => segment.id)).toEqual(
      SEGMENTS.map((segment) => segment.id),
    );
    expect(result[0].targetText).toBe("first");
    expect(result[2].targetText).toBe("third");
    expect(result[2].status).toBe("needs_review");
    expect(result[1]).toBe(SEGMENTS[1]);
  });
});

describe("SegmentGrid — selection keyed by segment id + reset (Phase 3)", () => {
  it("translates the checked SEGMENT IDS (not row indexes) and resets on search change", async () => {
    mockGridIpc();
    const startTranslation = vi.fn(async () => {});
    useLlmStore.setState({
      startTranslation,
      providerConfig: {
        url: "u",
        model: "m",
        batchSize: 1,
        resourceProfile: "balanced",
      },
    });
    const user = userEvent.setup();
    render(<SegmentGrid />);
    await screen.findByDisplayValue("target 1");

    const checkboxes = screen.getAllByRole("checkbox");
    // checkboxes[0] = header select-all; rows follow.
    await user.click(checkboxes[1]);
    await user.click(checkboxes[2]);
    const batchLabel = i18n.t("segmentGrid.translateSelected", { count: 2 });
    await user.click(screen.getByText(batchLabel));
    // getRowId: the selection designates segments by id — "0"/"1" would mean
    // the index-keyed regression is back.
    expect(startTranslation).toHaveBeenCalledWith(["s1", "s2"], undefined);

    await user.click(checkboxes[1]);
    await user.click(checkboxes[2]);
    expect(screen.getByText(batchLabel)).toBeInTheDocument();
    await user.type(
      screen.getByPlaceholderText(i18n.t("segmentGrid.searchPlaceholder")),
      "ソース1",
    );
    await waitFor(() =>
      expect(screen.queryByText(batchLabel)).not.toBeInTheDocument(),
    );
  });
});

describe("SegmentGrid — save failure surfaces a toast (Phase 3)", () => {
  it("shows segmentGrid.saveError when update_segment rejects", async () => {
    mockGridIpc({ failSave: true });
    const user = userEvent.setup();
    render(<SegmentGrid />);

    const input = await screen.findByDisplayValue("target 1");
    await user.clear(input);
    await user.type(input, "new text{Enter}");

    await waitFor(() => expect(toastError).toHaveBeenCalled());
    expect(String(toastError.mock.calls[0][0])).toContain(
      i18n.t("segmentGrid.saveError", { error: "" }).slice(0, 20),
    );
  });
});

describe("SegmentGrid — recoverable loading error", () => {
  it("shows the failure and loads the file after retry", async () => {
    mockGridIpc({ failFirstLoad: true });
    const user = userEvent.setup();
    render(<SegmentGrid />);

    const alert = await screen.findByRole("alert");
    expect(alert).toHaveTextContent(i18n.t("segmentGrid.loadError"));
    expect(alert).toHaveTextContent("database unavailable");

    await user.click(
      screen.getByRole("button", { name: i18n.t("segmentGrid.retry") }),
    );
    expect(await screen.findByDisplayValue("target 5")).toBeInTheDocument();
  });
});

describe("SegmentGrid — row density", () => {
  it("switches between comfortable and compact rows", async () => {
    mockGridIpc();
    const user = userEvent.setup();
    render(<SegmentGrid />);
    await screen.findByDisplayValue("target 1");

    const compact = screen.getByRole("button", {
      name: i18n.t("segmentGrid.density.compact"),
    });
    expect(compact).toHaveAttribute("aria-pressed", "false");

    await user.click(compact);
    expect(useUiStore.getState().gridDensity).toBe("compact");
    expect(compact).toHaveAttribute("aria-pressed", "true");
  });
});
