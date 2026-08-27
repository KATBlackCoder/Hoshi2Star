import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { ModeNavigation } from "@/components/shell/ModeNavigation";
import { useUiStore } from "@/stores/ui";

beforeEach(() => useUiStore.setState({ mode: "library" }));

describe("ModeNavigation", () => {
  it("marks and changes the current mode", async () => {
    render(<ModeNavigation onOpenSettings={vi.fn()} />);
    expect(
      screen.getByRole("button", { name: "Bibliothèque" }),
    ).toHaveAttribute("aria-current", "page");
    await userEvent.click(screen.getByRole("button", { name: "Images" }));
    expect(useUiStore.getState().mode).toBe("images");
  });

  it("supports horizontal arrow navigation", async () => {
    render(<ModeNavigation onOpenSettings={vi.fn()} />);
    const library = screen.getByRole("button", { name: "Bibliothèque" });
    library.focus();
    await userEvent.keyboard("{ArrowRight}");
    expect(useUiStore.getState().mode).toBe("patch");
  });

  it("opens the terminology workspace as its own mode", async () => {
    render(<ModeNavigation onOpenSettings={vi.fn()} />);
    await userEvent.click(screen.getByRole("button", { name: "Terminologie" }));
    expect(useUiStore.getState().mode).toBe("terminology");
  });
});
