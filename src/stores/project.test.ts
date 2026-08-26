import { beforeEach, describe, expect, it, vi } from "vitest";
import { mockIPC } from "@tauri-apps/api/mocks";

const toastSuccess = vi.fn();
vi.mock("sonner", () => ({
  toast: {
    success: (...args: unknown[]) => toastSuccess(...args),
    error: vi.fn(),
  },
}));

import {
  openProject,
  loadAllProjects,
  useProjectStore,
} from "@/stores/project";
import type { Project, ProjectStats } from "@/lib/types";
import { useSettingsStore } from "@/stores/settings";

const initialState = useProjectStore.getState();

const PROJECT: Project = {
  id: "p1",
  name: "Test Game",
  engine: "mv_mz",
  gamePath: "/tmp/game",
  sourceLang: "ja",
  targetLang: "fr",
  createdAt: "2026-07-01",
  updatedAt: "2026-07-01",
} as Project;

const STATS: ProjectStats = {
  fileCount: 1,
  totalSegments: 10,
  untranslatedCount: 8,
  translatedCount: 2,
  needsReviewCount: 0,
  reviewedCount: 0,
};

beforeEach(() => {
  toastSuccess.mockClear();
  useProjectStore.setState(initialState, true);
});

describe("project store — openProject thunk", () => {
  it("registers the project, sets it active and loads files + stats", async () => {
    let openArgs: unknown;
    mockIPC((cmd, args) => {
      switch (cmd) {
        case "open_project":
          openArgs = args;
          return { project: PROJECT, wasRestored: false };
        case "get_source_files":
          return [];
        case "get_project_stats":
          return STATS;
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });

    const result = await openProject("/tmp/game");

    expect(result.project.id).toBe("p1");
    const s = useProjectStore.getState();
    expect(s.projects).toHaveLength(1);
    expect(s.activeProjectId).toBe("p1");
    expect(s.activeProjectStats).toEqual(STATS);
    // Fresh extraction → glossary-extract prompt armed + extraction toast
    expect(s.pendingGlossaryExtract).toBe("p1");
    expect(openArgs as Record<string, unknown>).toMatchObject({
      path: "/tmp/game",
      sourceLang: useSettingsStore.getState().settings.defaultSourceLang,
      targetLang: useSettingsStore.getState().settings.defaultTargetLang,
    });
    expect(toastSuccess).toHaveBeenCalled();
  });

  it("does not arm the glossary prompt when the project was restored", async () => {
    mockIPC((cmd) => {
      switch (cmd) {
        case "open_project":
          return { project: PROJECT, wasRestored: true };
        case "get_source_files":
          return [];
        case "get_project_stats":
          return STATS;
        default:
          throw new Error(`unexpected command: ${cmd}`);
      }
    });

    await openProject("/tmp/game");
    expect(useProjectStore.getState().pendingGlossaryExtract).toBeNull();
  });

  it("updates a restored project without duplicating its library card", async () => {
    useProjectStore.setState({ projects: [PROJECT] });
    mockIPC((cmd) => {
      if (cmd === "open_project") {
        return {
          project: { ...PROJECT, name: "Updated title" },
          wasRestored: true,
        };
      }
      if (cmd === "get_source_files") return [];
      if (cmd === "get_project_stats") return STATS;
      throw new Error(`unexpected command: ${cmd}`);
    });

    await openProject("/tmp/game");

    expect(useProjectStore.getState().projects).toHaveLength(1);
    expect(useProjectStore.getState().projects[0].name).toBe("Updated title");
  });

  it("propagates a backend failure to the caller without touching the store", async () => {
    mockIPC(() => {
      throw new Error("could not identify game engine");
    });

    await expect(openProject("/tmp/nope")).rejects.toThrow();
    expect(useProjectStore.getState().projects).toHaveLength(0);
    expect(useProjectStore.getState().activeProjectId).toBeNull();
  });
});

describe("project store — loadAllProjects", () => {
  it("merges DB projects without duplicating existing ones", async () => {
    useProjectStore.getState().addProject(PROJECT);
    mockIPC((cmd) => {
      if (cmd === "list_projects") {
        return [PROJECT, { ...PROJECT, id: "p2", name: "Other" }];
      }
      throw new Error(`unexpected command: ${cmd}`);
    });

    await loadAllProjects();
    const ids = useProjectStore.getState().projects.map((p) => p.id);
    expect(ids).toEqual(["p1", "p2"]);
  });
});
