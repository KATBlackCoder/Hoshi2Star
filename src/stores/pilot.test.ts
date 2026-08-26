import { beforeEach, describe, expect, it, vi } from "vitest";
import type { PilotRunReport, Project, ProviderConfig } from "@/lib/types";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  unlisten: vi.fn(),
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));

import { usePilotStore } from "@/stores/pilot";

const PROJECT: Project = {
  id: "p1",
  name: "Pilot Game",
  engine: "mv_mz",
  gamePath: "/tmp/pilot-game",
  sourceLang: "ja",
  targetLang: "fr",
  createdAt: "2026-08-26",
  updatedAt: "2026-08-26",
};

const PROVIDER: ProviderConfig = {
  providerId: "ollama",
  url: "http://localhost:11434/v1",
  model: "gemma4:e4b",
  batchSize: 8,
  resourceProfile: "eco",
};

const REPORT = {
  comparisons: [],
  preparation: {
    engine: "mv_mz",
    sourceLanguage: "ja",
    targetLanguage: "fr",
    totalFiles: 1,
    totalSegments: 4,
    sampleSize: 4,
    byKind: {},
    byCategory: {},
    sample: [],
    isolation: {
      personalDatabaseAccessed: false,
      gameFilesWritten: 0,
      databaseRemoved: true,
    },
  },
  baseline: {
    metrics: {
      calls: [],
      requestCount: 0,
      inputUnits: 0,
      promptChars: 0,
      promptTokens: null,
      completionTokens: null,
      totalTokens: null,
      durationMs: 0,
      attempts: 0,
    },
    averageQaScore: 100,
    needsReviewCount: 0,
  },
  contextual: {
    metrics: {
      calls: [],
      requestCount: 0,
      inputUnits: 0,
      promptChars: 0,
      promptTokens: null,
      completionTokens: null,
      totalTokens: null,
      durationMs: 0,
      attempts: 0,
    },
    averageQaScore: 100,
    needsReviewCount: 0,
  },
  changedCount: 0,
} satisfies PilotRunReport;

beforeEach(() => {
  mocks.invoke.mockReset();
  mocks.listen.mockReset();
  mocks.unlisten.mockReset();
  mocks.listen.mockResolvedValue(mocks.unlisten);
  usePilotStore.setState({
    isOpen: false,
    isRunning: false,
    sampleSize: 12,
    progress: null,
    report: null,
    error: null,
    categoryFilter: "all",
    decisions: {},
  });
});

describe("pilot store", () => {
  it("runs the isolated command with project languages and tears down progress", async () => {
    mocks.invoke.mockResolvedValue(REPORT);

    await usePilotStore.getState().runPilot(PROJECT, PROVIDER);

    expect(mocks.listen).toHaveBeenCalledWith(
      "h2s://pilot/progress",
      expect.any(Function),
    );
    expect(mocks.invoke).toHaveBeenCalledWith("run_mv_mz_pilot", {
      gamePath: PROJECT.gamePath,
      sourceLang: "ja",
      targetLang: "fr",
      sampleSize: 12,
      providerConfig: PROVIDER,
    });
    expect(mocks.unlisten).toHaveBeenCalledOnce();
    expect(usePilotStore.getState()).toMatchObject({
      isOpen: true,
      isRunning: false,
      report: REPORT,
    });
  });

  it("captures progress and a provider error without leaving a listener", async () => {
    mocks.listen.mockImplementation(async (_event, callback) => {
      callback({
        payload: { phase: "contextual", done: 2, total: 4 },
      });
      return mocks.unlisten;
    });
    mocks.invoke.mockRejectedValue(new Error("provider offline"));

    await usePilotStore.getState().runPilot(PROJECT, PROVIDER);

    expect(usePilotStore.getState()).toMatchObject({
      isRunning: false,
      error: "provider offline",
      progress: null,
    });
    expect(mocks.unlisten).toHaveBeenCalledOnce();
  });

  it("guards other engines and bounds the sample size", async () => {
    usePilotStore.getState().setSampleSize(200);
    expect(usePilotStore.getState().sampleSize).toBe(50);
    usePilotStore.getState().setSampleSize(1);
    expect(usePilotStore.getState().sampleSize).toBe(4);

    await usePilotStore
      .getState()
      .runPilot({ ...PROJECT, engine: "wolf" }, PROVIDER);
    expect(mocks.invoke).not.toHaveBeenCalled();
  });
});
