import type React from "react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
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
import { useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { useLlmStore } from "@/stores/llm";
import { useSettingsStore } from "@/stores/settings";
import { useUiStore } from "@/stores/ui";
import type { Project, Segment } from "@/lib/types";

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

function mockAppIpc() {
  mockIPC(
    (cmd) => {
      switch (cmd) {
        case "get_segments":
          return {
            items: SEGMENTS,
            total: SEGMENTS.length,
            page: 0,
            pageSize: 2000,
          };
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

function renderApp() {
  const client = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  return render(
    <QueryClientProvider client={client}>
      <App />
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
