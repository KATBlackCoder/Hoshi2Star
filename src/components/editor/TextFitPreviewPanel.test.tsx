import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { TextFitPreviewPanel } from "@/components/editor/TextFitPreviewPanel";
import i18n from "@/lib/i18n";
import type { Project, TextFitPreview } from "@/lib/types";
import { useEditorStore } from "@/stores/editor";
import { useProjectStore } from "@/stores/project";

const initialEditor = useEditorStore.getState();
const initialProject = useProjectStore.getState();

function project(engine: string): Project {
  return {
    id: "p1",
    name: "Game",
    engine,
    gamePath: "/game",
    sourceLang: "ja",
    targetLang: "en",
    createdAt: "",
    updatedAt: "",
  };
}

const PREVIEW: TextFitPreview = {
  fileName: "Map001.json",
  enabled: true,
  innerWidth: 784,
  innerHeight: 152,
  textX: 4,
  limit: 780,
  fontSize: 26,
  item: {
    pointer: "/events/1/pages/0/list/0",
    description: false,
    face: true,
    lines: ["/events/1/pages/0/list/1/parameters/0"],
    segmentIds: ["s1"],
    texts: ["Hello there, traveller."],
    sourceHash: "h",
    override: null,
    fitted: {
      outcome: { kind: "rewrapped" },
      values: ["Hello there,\ntraveller."],
      pages: [
        {
          lines: [
            {
              runs: [
                {
                  text: "Hello there,",
                  x: 164,
                  width: 156,
                  fontSize: 26,
                  color: 0,
                  icon: false,
                },
              ],
              y: 0,
              height: 36,
              right: 320,
              overflow: false,
              clipped: false,
            },
            {
              runs: [
                {
                  text: "traveller.",
                  x: 164,
                  width: 130,
                  fontSize: 26,
                  color: 2,
                  icon: false,
                },
              ],
              y: 36,
              height: 36,
              right: 294,
              overflow: false,
              clipped: false,
            },
          ],
        },
      ],
    },
  },
};

function renderPanel() {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });
  render(
    <QueryClientProvider client={queryClient}>
      <TextFitPreviewPanel />
    </QueryClientProvider>,
  );
}

beforeEach(() => {
  useEditorStore.setState(initialEditor, true);
  useProjectStore.setState(initialProject, true);
  useEditorStore.setState({
    activeSegmentId: "s1",
    activeSegmentTargetText: "Hello there, traveller.",
  });
  useProjectStore.setState({
    activeProjectId: "p1",
    projects: [project("mv_mz")],
  });
});

describe("TextFitPreviewPanel", () => {
  it("draws the laid-out message with its portrait area", async () => {
    mockIPC((cmd) => {
      if (cmd === "preview_text_fit") return PREVIEW;
      throw new Error(`unexpected command: ${cmd}`);
    });
    renderPanel();

    expect(await screen.findByText("Hello there,")).toBeInTheDocument();
    expect(screen.getByText("traveller.")).toHaveAttribute("fill", "#ff784c");
    expect(
      screen.getByText(i18n.t("textFit.outcome.rewrapped")),
    ).toBeInTheDocument();
    expect(
      screen.getByText(i18n.t("textFit.preview.portrait")),
    ).toBeInTheDocument();
    expect(
      screen.queryByText(i18n.t("textFit.preview.off")),
    ).not.toBeInTheDocument();
  });

  it("stores a per-text choice", async () => {
    let choiceArgs: unknown;
    mockIPC((cmd, args) => {
      if (cmd === "preview_text_fit") return PREVIEW;
      if (cmd === "set_text_fit_override") {
        choiceArgs = args;
        return PREVIEW;
      }
      throw new Error(`unexpected command: ${cmd}`);
    });
    renderPanel();

    await userEvent.click(
      await screen.findByRole("checkbox", {
        name: i18n.t("textFit.preview.asTranslated"),
      }),
    );
    await waitFor(() =>
      expect(choiceArgs).toEqual({
        projectId: "p1",
        segmentId: "s1",
        choice: { lineBreaks: null, overflow: null, disabled: true },
      }),
    );
  });

  it("says when fitting is off and when the game is not MZ", async () => {
    mockIPC((cmd) =>
      cmd === "preview_text_fit" ? { ...PREVIEW, enabled: false } : null,
    );
    renderPanel();
    expect(
      await screen.findByText(i18n.t("textFit.preview.off")),
    ).toBeInTheDocument();

    useProjectStore.setState({ projects: [project("wolf")] });
    expect(
      await screen.findByText(i18n.t("textFit.preview.mzOnly")),
    ).toBeInTheDocument();
  });
});
