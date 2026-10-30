import type React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClientProvider } from "@tanstack/react-query";
import { mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";

// Render counter for App: useAppHandlers is called exactly once per App render.
const renders = vi.hoisted(() => ({ app: 0 }));
vi.mock("@/hooks/useAppHandlers", async (importOriginal) => {
  const actual =
    await importOriginal<typeof import("@/hooks/useAppHandlers")>();
  return {
    ...actual,
    useAppHandlers: () => {
      renders.app += 1;
      return actual.useAppHandlers();
    },
  };
});

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

// jsdom has no ResizeObserver: plain containers stand in for the panels.
vi.mock("@/components/ui/resizable", () => ({
  ResizablePanelGroup: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  ResizablePanel: ({ children }: { children: React.ReactNode }) => (
    <div>{children}</div>
  ),
  ResizableHandle: () => null,
}));

vi.mock("@tauri-apps/plugin-store", () => ({
  load: async () => ({ get: async () => undefined }),
}));

import App from "@/App";
import { PackImportWizard } from "@/components/PackImportWizard";
import i18n from "@/lib/i18n";
import { queryClient } from "@/lib/queryClient";
import { applyResync } from "@/stores/resync";
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { useLlmStore } from "@/stores/llm";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";
import type { ImportPreview, Project, Segment } from "@/lib/types";

const initialProject = useProjectStore.getState();
const initialEditor = useEditorStore.getState();
const initialLlm = useLlmStore.getState();
const initialSettings = useSettingsStore.getState();

const PROJECT = {
  id: "p1",
  name: "Jeu",
  engine: "mv_mz",
  gamePath: "/tmp/p1",
  sourceLang: "ja",
  targetLang: "fr",
  createdAt: "2026-10-29 00:00:00",
  updatedAt: "2026-10-29 00:00:00",
} as Project;

const SEGMENTS = Array.from({ length: 3 }, (_, i) => ({
  id: `s${i + 1}`,
  sourceFileId: "f1",
  jsonKey: `/k/${i}`,
  sourceText: `ソース${i + 1}`,
  targetText: `cible ${i + 1}`,
  status: "translated",
  qaScore: null,
  createdAt: "",
  updatedAt: "",
})) as Segment[];

/** Segments served by `get_segments` (a test may swap them). */
let served: Segment[] = SEGMENTS;
/** IPC calls per command since the last `resetCalls()`. */
let calls: Record<string, number> = {};
function resetCalls() {
  calls = {};
}

function mockAppIpc() {
  mockIPC(
    (cmd, args) => {
      calls[cmd] = (calls[cmd] ?? 0) + 1;
      switch (cmd) {
        case "get_segments":
          return {
            items: served,
            total: served.length,
            page: 0,
            pageSize: 2000,
          };
        case "list_projects":
          return [PROJECT];
        case "update_segment": {
          const { id, targetText } = args as { id: string; targetText: string };
          return { ...served.find((s) => s.id === id)!, targetText };
        }
        // The backend emits its event before the command returns.
        case "resync_project": {
          const { apply } = args as { apply: boolean };
          if (apply) void emit("h2s://project/resynced", {});
          return {
            applied: apply,
            files: [],
            moved: 0,
            added: 0,
            gone: 0,
            orphaned: 0,
            untouchedCategories: [],
            changedSources: 0,
          };
        }
        case "apply_h2s_import":
          void emit("h2s://project/import-done", {});
          return IMPORT_REPORT;
        case "get_source_files":
          return [];
        case "get_project_stats":
          return {
            fileCount: 1,
            totalSegments: 3,
            untranslatedCount: 0,
            translatedCount: 3,
            needsReviewCount: 0,
            reviewedCount: 0,
            resumableCount: 0,
          };
        case "get_qa_report":
          return {
            totalSegments: 3,
            okCount: 3,
            errorCount: 0,
            criticalCount: 0,
            errorsByType: {},
          };
        case "qa_check_segment":
          return { score: 100, errors: [] };
        case "translate_segments":
          return null;
        default:
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}

const IMPORT_REPORT = {
  applied: 2,
  appliedSourceChanged: 0,
  skippedConflicts: 0,
  skippedSourceChanged: 0,
  identical: 0,
  orphans: 0,
  terminologyAdded: 0,
  terminologyUpdated: 0,
  terminologyConflicts: 0,
  tmAdded: 0,
  backupPath: "/tmp/backup.h2s",
};

const IMPORT_PREVIEW: ImportPreview = {
  blocker: null,
  packGameTitle: "Jeu",
  packAppVersion: "0.4.10",
  packCreatedAt: "2026-10-29T10:00:00Z",
  titleMismatch: false,
  applicable: 2,
  identical: 0,
  conflicts: 0,
  sourceChanged: 0,
  orphans: 0,
  terminologyCount: 0,
  terminologyCreates: 0,
  terminologyUpdates: 0,
  terminologyConflicts: 0,
  tmCount: 0,
};

/** The app's own client, so writes can invalidate what the panels show. */
function renderApp(extra?: React.ReactNode) {
  return render(
    <QueryClientProvider client={queryClient}>
      <App />
      {extra}
    </QueryClientProvider>,
  );
}

async function settle() {
  // Let pending IPC promises, listeners and frames resolve.
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 30));
  });
}

beforeEach(() => {
  window.matchMedia ??= ((query: string) => ({
    matches: false,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  })) as typeof window.matchMedia;
  useProjectStore.setState(initialProject, true);
  useEditorStore.setState(initialEditor, true);
  useLlmStore.setState(initialLlm, true);
  useSettingsStore.setState(initialSettings, true);
  useProjectStore.setState({ projects: [PROJECT], activeProjectId: "p1" });
  useEditorStore.setState({ activeFileId: "f1" });
  useUiStore.setState({
    mode: "patch",
    inspectorOpen: true,
    inspectorTab: "qa",
  });
  queryClient.clear();
  served = SEGMENTS;
  resetCalls();
  renders.app = 0;
  mockAppIpc();
});

describe("App re-renders (audit P2: R1, R3, R5)", () => {
  it("does not re-render App on LLM run events", async () => {
    renderApp();
    expect(await screen.findByText("ソース1")).toBeInTheDocument();
    await settle();

    await act(async () => {
      await useLlmStore.getState().startTranslation(["s1"], undefined);
    });
    await settle();
    const before = renders.app;

    await act(async () => {
      await emit("h2s://llm/progress", { done: 1, total: 10 });
      await emit("h2s://llm/metrics", [{ requestId: 1 }]);
      await emit("h2s://llm/pipeline-metrics", { batches: 1 });
      await emit("h2s://llm/progress", { done: 2, total: 10 });
    });
    await settle();

    expect(useLlmStore.getState().translationProgress).toBe(20);
    expect(renders.app - before).toBe(0);
  });

  it("does not re-render App when a grid row is clicked", async () => {
    renderApp();
    const row = await screen.findByText("ソース2");
    await settle();
    const before = renders.app;

    await userEvent.click(row);
    await settle();

    expect(useEditorStore.getState()).toMatchObject({
      activeSegmentId: "s2",
      activeSegmentSourceText: "ソース2",
      activeSegmentTargetText: "cible 2",
    });
    expect(renders.app - before).toBe(0);
  });

  it("does not re-render App when the settings change", async () => {
    renderApp();
    await screen.findByText("ソース1");
    await settle();
    const before = renders.app;

    act(() => {
      useSettingsStore.setState((s) => ({
        settings: { ...s.settings, batchSize: s.settings.batchSize + 1 },
      }));
    });
    await settle();

    expect(renders.app - before).toBe(0);
  });
});

const RUN = {
  count: 2,
  translated: 2,
  needsReview: 0,
  failed: 0,
  qaErrorsByType: {},
  cancelled: false,
  repaired: 0,
};

describe("IPC after a project event (audit P3: I1 to I3)", () => {
  async function renderGrid() {
    renderApp();
    await screen.findByText("ソース1");
    await settle();
    resetCalls();
  }

  it("refreshes files once and skips the dead stats after a resync", async () => {
    await renderGrid();

    await act(async () => {
      await applyResync("p1");
    });
    await settle();

    expect(calls.get_source_files ?? 0).toBe(1);
    expect(calls.get_project_stats ?? 0).toBe(0);
    expect(calls.get_segments ?? 0).toBe(1);
  });

  it("refreshes files once and skips the dead stats after a pack import", async () => {
    renderApp(
      <PackImportWizard
        projectId="p1"
        packPath="/tmp/pack.h2s"
        preview={IMPORT_PREVIEW}
        onClose={() => {}}
      />,
    );
    await screen.findByText("ソース1");
    await settle();
    resetCalls();

    await userEvent.click(screen.getByText(i18n.t("pack.importConfirm")));
    await settle();

    expect(calls.apply_h2s_import).toBe(1);
    expect(calls.get_source_files ?? 0).toBe(1);
    expect(calls.get_project_stats ?? 0).toBe(0);
    expect(calls.get_segments ?? 0).toBe(1);
  });

  it("re-syncs the grid in place after a run that wrote segments", async () => {
    await renderGrid();
    await act(async () => {
      await useLlmStore.getState().startTranslation(["s1", "s2"], undefined);
    });
    const row = screen.getByDisplayValue("cible 1");
    resetCalls();

    await act(async () => {
      await emit("h2s://llm/completed", RUN);
    });
    await settle();

    // Same DOM node: the list was not unmounted behind a loading screen.
    expect(screen.getByDisplayValue("cible 1")).toBe(row);

    expect(calls.get_segments ?? 0).toBe(1);
    expect(calls.get_source_files ?? 0).toBe(1);
    expect(calls.get_project_stats ?? 0).toBe(0);
  });

  it("does not reload anything after a run that wrote nothing", async () => {
    await renderGrid();
    await act(async () => {
      await useLlmStore.getState().startTranslation(["s1"], undefined);
    });
    resetCalls();

    await act(async () => {
      await emit("h2s://llm/completed", {
        ...RUN,
        count: 1,
        translated: 0,
        failed: 1,
      });
    });
    await settle();

    expect(calls.get_segments ?? 0).toBe(0);
    expect(calls.get_source_files ?? 0).toBe(0);
    expect(calls.get_project_stats ?? 0).toBe(0);
  });
});

describe("Library stats (audit P3: I4, I5)", () => {
  async function switchMode(mode: "library" | "patch") {
    act(() => useUiStore.getState().setMode(mode));
    await settle();
  }

  it("fetches the stats again only after a segment was written", async () => {
    useUiStore.setState({ mode: "library" });
    renderApp();
    await screen.findAllByText("Jeu");
    await settle();
    expect(calls.get_project_stats).toBe(1);

    await switchMode("patch");
    await switchMode("library");
    expect(calls.get_project_stats).toBe(1);

    await switchMode("patch");
    const target = await screen.findByDisplayValue("cible 1");
    await userEvent.clear(target);
    await userEvent.type(target, "nouvelle cible");
    await userEvent.tab();
    await settle();
    expect(calls.update_segment).toBe(1);

    await switchMode("library");
    expect(calls.get_project_stats).toBe(2);
  });
});

describe("Inspector queries (audit P3: I6)", () => {
  it("refreshes the project QA report after a segment is saved", async () => {
    renderApp();
    const target = await screen.findByDisplayValue("cible 1");
    await settle();
    const reports = calls.get_qa_report ?? 0;
    expect(reports).toBe(1);

    await userEvent.clear(target);
    await userEvent.type(target, "nouvelle cible");
    await userEvent.tab();
    await settle();

    expect(calls.update_segment).toBe(1);
    expect(calls.get_qa_report).toBe(reports + 1);
  });

  it("checks each segment, even when two share the same texts", async () => {
    served = [
      { ...SEGMENTS[0], targetText: "même" },
      {
        ...SEGMENTS[1],
        sourceText: SEGMENTS[0].sourceText,
        targetText: "même",
      },
    ];
    renderApp();
    const rows = await screen.findAllByText("ソース1");
    await settle();

    await userEvent.click(rows[0]);
    await settle();
    await userEvent.click(rows[1]);
    await settle();

    expect(useEditorStore.getState().activeSegmentId).toBe("s2");
    expect(calls.qa_check_segment).toBe(2);
  });
});
