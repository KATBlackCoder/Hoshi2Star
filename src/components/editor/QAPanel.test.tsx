import { renderWithQuery } from "@/test/renderWithQuery";
import { mockIPC } from "@tauri-apps/api/mocks";
import { screen } from "@testing-library/react";
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

    useEditorStore.setState({
      activeSegmentSourceText: "星",
      activeSegmentTargetText: "   ",
    });
    renderWithQuery(<QAPanel />);

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
    useEditorStore.setState({
      activeSegmentSourceText: "勇者",
      activeSegmentTargetText: "Champion",
    });
    renderWithQuery(<QAPanel />);

    expect(await screen.findByText(/Information/)).toBeInTheDocument();
    expect(qaArgs).toMatchObject({ segmentId: "s1", projectId: "p1" });
  });
});

describe("QAPanel — score ring (audit F1)", () => {
  it("shows a score above the critical cut as a warning", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_qa_report") {
        return {
          totalSegments: 1,
          okCount: 0,
          errorCount: 1,
          criticalCount: 0,
          errorsByType: {},
        };
      }
      if (cmd === "qa_check_segment") {
        return { score: 72, errors: [{ type: "bom_detected" }] };
      }
      throw new Error(`unexpected command: ${cmd}`);
    });
    useEditorStore.setState({
      activeSegmentSourceText: "星",
      activeSegmentTargetText: "étoile",
    });
    renderWithQuery(<QAPanel />);

    expect(await screen.findByText("72")).toHaveClass("text-yellow-400");
  });
});

describe("QAPanel — Wolf face prefix (plan FP)", () => {
  it.each([
    ["@1", "", "missing"],
    ["@1", "@1", "moved"],
    ["@1", "@2", "wrong"],
    ["", "@3", "unexpected"],
  ])(
    "labels expected %j / found %j as %s, in red",
    async (expected, found, labelCase) => {
      mockIPC((cmd) => {
        if (cmd === "get_qa_report") {
          return {
            totalSegments: 1,
            okCount: 0,
            errorCount: 1,
            criticalCount: 1,
            errorsByType: { misplaced_face_prefix: 1 },
          };
        }
        if (cmd === "qa_check_segment") {
          return {
            score: 75,
            errors: [{ type: "misplaced_face_prefix", expected, found }],
          };
        }
        throw new Error(`unexpected command: ${cmd}`);
      });
      useEditorStore.setState({
        activeSegmentSourceText: "@1\n「\\cself[8]」を失った。",
        activeSegmentTargetText: "Lost \\cself[8]「@1\n」.",
      });
      renderWithQuery(<QAPanel />);

      const label = await screen.findByText(
        i18n.t(`qaPanel.errors.misplaced_face_prefix.${labelCase}`, {
          expected,
          found,
        }),
      );
      expect(label.textContent).toContain(expected || found);
      expect(label.parentElement?.querySelector("svg")).toHaveClass(
        "text-red-400",
      );
    },
  );
});
