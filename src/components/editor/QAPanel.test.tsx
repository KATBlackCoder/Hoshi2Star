import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it } from "vitest";
import { QAPanel } from "@/components/editor/QAPanel";
import i18n from "@/lib/i18n";
import { useEditorStore } from "@/stores/editor";
import { useProjectStore } from "@/stores/project";

const initialEditor = useEditorStore.getState();
const initialProject = useProjectStore.getState();

beforeEach(() => {
  useEditorStore.setState(initialEditor, true);
  useProjectStore.setState(initialProject, true);
  useEditorStore.setState({ activeSegmentId: "s1" });
  useProjectStore.setState({ activeProjectId: "p1" });
});

describe("QAPanel", () => {
  it("marks a selected segment with an empty target as invalid", async () => {
    const commands: string[] = [];
    mockIPC((cmd) => {
      commands.push(cmd);
      if (cmd === "get_qa_report") {
        return {
          totalSegments: 0,
          okCount: 0,
          errorCount: 0,
          criticalCount: 0,
          errorsByType: {},
        };
      }
      throw new Error(`unexpected command: ${cmd}`);
    });
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });

    render(
      <QueryClientProvider client={queryClient}>
        <QAPanel sourceText="星" targetText="   " />
      </QueryClientProvider>,
    );

    expect(
      await screen.findByText(i18n.t("qaPanel.errors.empty_translation")),
    ).toBeInTheDocument();
    expect(screen.getByText("0")).toBeInTheDocument();
    expect(commands).not.toContain("qa_check_segment");
  });

  it("renders terminology severity and sends the exact segment id", async () => {
    let qaArgs: Record<string, unknown> | undefined;
    mockIPC((cmd, args) => {
      if (cmd === "get_qa_report") {
        return {
          totalSegments: 1,
          okCount: 0,
          errorCount: 1,
          criticalCount: 0,
          errorsByType: { terminology_mismatch: 1 },
          terminologyIssues: [],
        };
      }
      if (cmd === "qa_check_segment") {
        qaArgs = args as Record<string, unknown>;
        return {
          score: 100,
          errors: [
            {
              type: "terminology_mismatch",
              entry_id: "e1",
              source_term: "勇者",
              expected_targets: ["Hero"],
              severity: "info",
            },
          ],
        };
      }
      throw new Error(`unexpected command: ${cmd}`);
    });
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    render(
      <QueryClientProvider client={queryClient}>
        <QAPanel sourceText="勇者" targetText="Champion" />
      </QueryClientProvider>,
    );

    expect(await screen.findByText(/Information/)).toBeInTheDocument();
    expect(qaArgs).toMatchObject({ segmentId: "s1", projectId: "p1" });
  });
});
