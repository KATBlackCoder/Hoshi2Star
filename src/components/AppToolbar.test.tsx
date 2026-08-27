import { act, render, screen } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { AppToolbar } from "@/components/AppToolbar";
import type { Project } from "@/lib/types";
import i18n from "@/lib/i18n";
import { usePilotStore } from "@/stores/pilot";
import { useProjectStore } from "@/stores/project";
import { DEFAULT_SETTINGS, useSettingsStore } from "@/stores/settings";

vi.mock("@tauri-apps/plugin-dialog", () => ({ open: vi.fn() }));
vi.mock("@/components/UpdateBadge", () => ({ UpdateBadge: () => null }));
vi.mock("@/components/PackExportDialog", () => ({
  PackExportDialog: () => null,
}));
vi.mock("@/components/PackImportWizard", () => ({
  PackImportWizard: () => null,
}));

const PROJECT: Project = {
  id: "p1",
  name: "MV Test",
  engine: "mv_mz",
  gamePath: "/tmp/mv-test",
  sourceLang: "ja",
  targetLang: "fr",
  createdAt: "2026-08-26",
  updatedAt: "2026-08-26",
};

beforeEach(() => {
  useProjectStore.setState({
    projects: [PROJECT],
    activeProjectId: PROJECT.id,
  });
  useSettingsStore.setState({ settings: { ...DEFAULT_SETTINGS } });
  usePilotStore.setState({ isOpen: false, isRunning: false });
});

describe("AppToolbar developer tools", () => {
  it("keeps the A/B pilot hidden until diagnostics are enabled", () => {
    render(
      <AppToolbar
        onOpenAbout={vi.fn()}
        onTranslate={vi.fn()}
        onTranslateAll={vi.fn()}
        onExportAll={vi.fn()}
        onOpenPilot={vi.fn()}
        isExporting={false}
      />,
    );

    expect(
      screen.queryByRole("button", { name: i18n.t("pilot.toolbar") }),
    ).not.toBeInTheDocument();

    act(() => {
      useSettingsStore.setState({
        settings: { ...DEFAULT_SETTINGS, developerTools: true },
      });
    });

    expect(
      screen.getByRole("button", { name: i18n.t("pilot.toolbar") }),
    ).toBeInTheDocument();
  });
});
