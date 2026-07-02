import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import { mockIPC } from "@tauri-apps/api/mocks";

vi.mock("@tauri-apps/plugin-dialog", () => ({
  open: vi.fn(async () => null),
}));

import { ProjectList } from "@/components/editor/ProjectList";
import { useProjectStore } from "@/stores/project";
import type { Project, ProjectStats } from "@/lib/types";

const initialState = useProjectStore.getState();

function makeProject(id: string, name: string): Project {
  return {
    id,
    name,
    engine: "mv_mz",
    gamePath: `/tmp/${id}`,
    createdAt: "2026-07-01 00:00:00",
    updatedAt: "2026-07-01 00:00:00",
  } as Project;
}

const STATS_P2: ProjectStats = {
  fileCount: 1,
  totalSegments: 20,
  untranslatedCount: 3,
  translatedCount: 15,
  needsReviewCount: 1,
  reviewedCount: 1,
};

beforeEach(() => {
  useProjectStore.setState(initialState, true);
});

describe("ProjectList — per-project stats resilience (Phase 3)", () => {
  it("still shows p2's stats bar when p1's get_project_stats fails", async () => {
    useProjectStore.setState({
      projects: [makeProject("p1", "Broken"), makeProject("p2", "Healthy")],
    });

    mockIPC((cmd, args) => {
      if (cmd === "list_projects") return [];
      if (cmd === "get_project_stats") {
        const { projectId } = args as { projectId: string };
        if (projectId === "p1") throw new Error("db locked");
        return STATS_P2;
      }
      throw new Error(`unexpected command: ${cmd}`);
    });

    render(<ProjectList />);

    // p2's bar renders despite p1's failure (Promise.allSettled).
    expect(await screen.findByText("✓ 15")).toBeInTheDocument();
    expect(screen.getByText("◎ 1")).toBeInTheDocument();
    expect(screen.getByText("⚠ 1")).toBeInTheDocument();
    // Only ONE stats bar in the list — p1 has none.
    expect(screen.getAllByText(/^✓ /)).toHaveLength(1);
    expect(screen.getByText("Broken")).toBeInTheDocument();
  });
});
