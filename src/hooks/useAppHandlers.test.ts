import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { mockIPC } from "@tauri-apps/api/mocks";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

import { useAppHandlers } from "@/hooks/useAppHandlers";
import { useProjectStore } from "@/stores/project";
import type { ProjectStats } from "@/lib/types";

const initialProject = useProjectStore.getState();

function statsWith(untranslated: number): ProjectStats {
  return {
    fileCount: 1,
    totalSegments: 10,
    untranslatedCount: untranslated,
    translatedCount: 10 - untranslated,
    needsReviewCount: 0,
    reviewedCount: 0,
  };
}

function mockStats(untranslated: number) {
  mockIPC((cmd) => {
    switch (cmd) {
      case "get_project_stats":
        return statsWith(untranslated);
      case "plugin:event|listen":
        return 1;
      case "plugin:event|unlisten":
        return undefined;
      default:
        throw new Error(`unexpected command: ${cmd}`);
    }
  });
}

beforeEach(() => {
  useProjectStore.setState(initialProject, true);
  useProjectStore.setState({ activeProjectId: "p1" });
});

describe("useAppHandlers — export gate", () => {
  it("blocks the export dialog while untranslated segments remain", async () => {
    mockStats(3);
    const { result } = renderHook(() => useAppHandlers());

    await act(async () => {
      await result.current.handleExportAll();
    });

    await waitFor(() => expect(result.current.exportDialog).toBe("blocked"));
    expect(result.current.exportStats?.untranslatedCount).toBe(3);
  });

  it("opens the confirm dialog when everything is translated", async () => {
    mockStats(0);
    const { result } = renderHook(() => useAppHandlers());

    await act(async () => {
      await result.current.handleExportAll();
    });

    await waitFor(() => expect(result.current.exportDialog).toBe("confirm"));
  });
});
