import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { SettingsModal } from "@/components/settings/SettingsModal";
import i18n from "@/lib/i18n";
import { DEFAULT_SETTINGS, useSettingsStore } from "@/stores/settings";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

beforeEach(() => {
  useSettingsStore.setState({ settings: { ...DEFAULT_SETTINGS } });
  document.documentElement.classList.add("dark");
  mockIPC((command) => {
    if (command === "get_provider_models") return [];
    throw new Error(`unexpected command: ${command}`);
  });
});

describe("SettingsModal", () => {
  it("separates core settings from developer diagnostics", async () => {
    const user = userEvent.setup();
    render(<SettingsModal open onClose={vi.fn()} />);

    expect(screen.getByRole("dialog")).toBeInTheDocument();
    expect(
      screen.getByRole("tab", { name: i18n.t("settings.tabs.provider") }),
    ).toHaveAttribute("aria-selected", "true");
    expect(
      screen.getByLabelText(i18n.t("settings.llm.urlLabel")),
    ).toBeInTheDocument();
    expect(
      screen.getByLabelText(i18n.t("settings.llm.resourceProfileLabel")),
    ).toBeInTheDocument();

    await user.click(
      screen.getByRole("tab", { name: i18n.t("settings.tabs.languages") }),
    );
    expect(screen.getByRole("tabpanel")).toHaveTextContent(
      i18n.t("settings.translation.section"),
    );

    await user.click(
      screen.getByRole("tab", { name: i18n.t("settings.tabs.appearance") }),
    );
    expect(
      screen.getByRole("button", {
        name: i18n.t("settings.appearance.dark"),
      }),
    ).toHaveAttribute("aria-pressed", "true");

    await user.click(
      screen.getByRole("tab", { name: i18n.t("settings.tabs.developer") }),
    );
    const developerTools = screen.getByRole("checkbox", {
      name: i18n.t("settings.developer.enable"),
    });
    expect(developerTools).not.toBeChecked();
    await user.click(developerTools);
    expect(developerTools).toBeChecked();
  });

  it("restores previews when cancelled", async () => {
    const onClose = vi.fn();
    const user = userEvent.setup();
    render(<SettingsModal open onClose={onClose} />);

    await user.click(
      screen.getByRole("tab", { name: i18n.t("settings.tabs.appearance") }),
    );
    await user.click(
      screen.getByRole("button", {
        name: i18n.t("settings.appearance.light"),
      }),
    );
    expect(document.documentElement).not.toHaveClass("dark");

    await user.click(
      screen.getByRole("button", { name: i18n.t("settings.cancel") }),
    );
    expect(document.documentElement).toHaveClass("dark");
    expect(onClose).toHaveBeenCalledOnce();
  });
});
