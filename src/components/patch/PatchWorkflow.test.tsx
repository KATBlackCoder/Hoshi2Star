import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { PatchWorkflow } from "@/components/patch/PatchWorkflow";
import { useProjectStore } from "@/stores/project";
import { useUiStore } from "@/stores/ui";

beforeEach(() => {
  useProjectStore.setState({ activeProjectId: null, projects: [] });
  useUiStore.setState({
    mode: "patch",
    inspectorOpen: false,
    inspectorTab: "tm",
  });
});

describe("PatchWorkflow", () => {
  it("keeps every gate unavailable until a project is open", () => {
    render(<PatchWorkflow onExport={vi.fn()} />);
    expect(screen.getByRole("navigation", { name: "Portes du patch" })).toBeInTheDocument();
    expect(screen.getAllByRole("button")).toHaveLength(5);
    expect(screen.getAllByRole("button").every((button) => button.hasAttribute("disabled"))).toBe(true);
  });

  it("routes terms, review and export through existing application actions", async () => {
    const onExport = vi.fn();
    useProjectStore.setState({ activeProjectId: "project-1" });
    render(<PatchWorkflow onExport={onExport} />);

    await userEvent.click(screen.getByRole("button", { name: /Termes/ }));
    expect(useUiStore.getState().mode).toBe("terminology");

    await userEvent.click(screen.getByRole("button", { name: /Révision/ }));
    expect(useUiStore.getState()).toMatchObject({
      inspectorOpen: true,
      inspectorTab: "qa",
    });

    await userEvent.click(screen.getByRole("button", { name: /Export/ }));
    expect(onExport).toHaveBeenCalledOnce();
  });
});
