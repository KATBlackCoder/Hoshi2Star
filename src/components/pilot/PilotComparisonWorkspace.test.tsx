import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import type { ReactNode } from "react";

vi.mock("@/components/ui/scroll-area", () => ({
  ScrollArea: ({ children }: { children: ReactNode }) => <div>{children}</div>,
}));

import { PilotComparisonWorkspace } from "@/components/pilot/PilotComparisonWorkspace";
import type { PilotRunReport, Project } from "@/lib/types";
import { usePilotStore } from "@/stores/pilot";
import { useProjectStore } from "@/stores/project";
import i18n from "@/lib/i18n";

const PROJECT: Project = {
  id: "p1",
  name: "Moon Test",
  engine: "mv_mz",
  gamePath: "/tmp/moon-test",
  sourceLang: "ja",
  targetLang: "fr",
  createdAt: "2026-08-26",
  updatedAt: "2026-08-26",
};

const METRICS = {
  calls: [],
  requestCount: 1,
  inputUnits: 1,
  promptChars: 100,
  promptTokens: 20,
  completionTokens: 10,
  totalTokens: 30,
  durationMs: 500,
  attempts: 1,
};

const REPORT: PilotRunReport = {
  preparation: {
    engine: "mv_mz",
    sourceLanguage: "ja",
    targetLanguage: "fr",
    totalFiles: 1,
    totalSegments: 1,
    sampleSize: 1,
    byKind: { dialogue: 1 },
    byCategory: { dialogue: 1 },
    sample: [],
    isolation: {
      personalDatabaseAccessed: false,
      gameFilesWritten: 0,
      databaseRemoved: true,
    },
  },
  comparisons: [
    {
      segment: {
        stableKey: "Map001.json::/events/1",
        fileName: "Map001.json",
        jsonKey: "/events/1",
        sourceText: "行こう",
        segmentKind: "dialogue",
        sceneId: "event:1",
        speaker: "勇者",
        branchPath: null,
        category: "dialogue",
        hasPlaceholders: false,
        context: {
          segmentKind: "dialogue",
          sceneId: "event:1",
          speaker: "勇者",
          branchPath: null,
          previous: [],
          following: [],
        },
      },
      baseline: {
        translatedText: "Allons-y.",
        qa: { score: 100, errors: [] },
        qualityFlags: [],
        needsReview: false,
        fromTm: false,
      },
      contextual: {
        translatedText: "On y va !",
        qa: { score: 100, errors: [] },
        qualityFlags: [],
        needsReview: false,
        fromTm: false,
      },
      changed: true,
      contextAvailable: true,
    },
  ],
  baseline: { metrics: METRICS, averageQaScore: 100, needsReviewCount: 0 },
  contextual: { metrics: METRICS, averageQaScore: 100, needsReviewCount: 0 },
  changedCount: 1,
};

beforeEach(() => {
  useProjectStore.setState({
    projects: [PROJECT],
    activeProjectId: PROJECT.id,
  });
  usePilotStore.setState({
    isOpen: true,
    isRunning: false,
    sampleSize: 12,
    progress: null,
    report: null,
    error: null,
    categoryFilter: "all",
    decisions: {},
  });
});

describe("PilotComparisonWorkspace", () => {
  it("shows the conservative setup before any provider call", () => {
    render(<PilotComparisonWorkspace />);

    expect(screen.getByText(i18n.t("pilot.title"))).toBeInTheDocument();
    expect(
      screen.getByRole("spinbutton", { name: i18n.t("pilot.sampleSize") }),
    ).toHaveValue(12);
    expect(
      screen.getByRole("button", { name: i18n.t("pilot.start") }),
    ).toBeInTheDocument();
  });

  it("shows both variants and keeps the human decision local", async () => {
    usePilotStore.setState({ report: REPORT });
    render(<PilotComparisonWorkspace />);

    expect(screen.getByText("Allons-y.")).toBeInTheDocument();
    expect(screen.getByText("On y va !")).toBeInTheDocument();
    expect(screen.getByText("行こう")).toBeInTheDocument();

    await userEvent.click(
      screen.getByRole("button", {
        name: i18n.t("pilot.decision.contextual"),
      }),
    );
    expect(usePilotStore.getState().decisions).toEqual({
      "Map001.json::/events/1": "contextual",
    });
  });
});
