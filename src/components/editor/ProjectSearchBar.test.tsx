import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import i18n from "@/lib/i18n";

import { ProjectSearchBar } from "@/components/editor/ProjectSearchBar";
import { useProjectStore } from "@/stores/project";
import { useSearchStore } from "@/stores/search";

const initialProject = useProjectStore.getState();
const initialSearch = useSearchStore.getState();

beforeEach(() => {
  useProjectStore.setState(initialProject, true);
  useSearchStore.setState(initialSearch, true);
  useProjectStore.setState({ activeProjectId: "p1" });
});

describe("ProjectSearchBar", () => {
  it("does not search on Enter when the query is shorter than 2 chars", async () => {
    const invokeSpy = vi.fn();
    mockIPC((cmd) => {
      invokeSpy(cmd);
      return { items: [], total: 0 };
    });
    const user = userEvent.setup();
    render(<ProjectSearchBar />);

    const input = screen.getByPlaceholderText(
      i18n.t("projectSearch.placeholder"),
    );
    await user.type(input, "a{Enter}");

    expect(invokeSpy).not.toHaveBeenCalled();
    expect(useSearchStore.getState().isActive).toBe(false);
  });

  it("searches with scope 'both' on Enter", async () => {
    const invokeSpy = vi.fn();
    mockIPC((cmd, args) => {
      invokeSpy(cmd, args);
      if (cmd === "search_segments") return { items: [], total: 0 };
      throw new Error(`unexpected command: ${cmd}`);
    });
    const user = userEvent.setup();
    render(<ProjectSearchBar />);

    const input = screen.getByPlaceholderText(
      i18n.t("projectSearch.placeholder"),
    );
    await user.type(input, "hero{Enter}");

    expect(invokeSpy).toHaveBeenCalledWith(
      "search_segments",
      expect.objectContaining({
        projectId: "p1",
        query: "hero",
        scope: "both",
      }),
    );
    expect(useSearchStore.getState().isActive).toBe(true);
  });

  it("clears the store when the X button is clicked", async () => {
    mockIPC((cmd) => {
      if (cmd === "search_segments") return { items: [], total: 0 };
      return undefined;
    });
    const user = userEvent.setup();
    render(<ProjectSearchBar />);

    const input = screen.getByPlaceholderText(
      i18n.t("projectSearch.placeholder"),
    );
    await user.type(input, "hero{Enter}");
    expect(useSearchStore.getState().isActive).toBe(true);

    await user.click(screen.getByTitle(i18n.t("projectSearch.clear")));

    const s = useSearchStore.getState();
    expect(s.isActive).toBe(false);
    expect(s.query).toBe("");
    expect((input as HTMLInputElement).value).toBe("");
  });
});
