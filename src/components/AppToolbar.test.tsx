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
        onDebugExport={vi.fn()}
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

function renderToolbar(onDebugExport = vi.fn()) {
  render(
    <AppToolbar
      onOpenAbout={vi.fn()}
      onTranslate={vi.fn()}
      onTranslateAll={vi.fn()}
      onExportAll={vi.fn()}
      onOpenPilot={vi.fn()}
      onDebugExport={onDebugExport}
      isExporting={false}
    />,
  );
  return onDebugExport;
}

const debugButton = () =>
  screen.queryByRole("button", { name: i18n.t("debugExport.toolbar") });

describe("AppToolbar debug export", () => {
  it("stays hidden until diagnostics are enabled", () => {
    renderToolbar();
    expect(debugButton()).not.toBeInTheDocument();
  });

  it.each(["mv_mz", "wolf"])(
    "is offered for %s projects with diagnostics enabled",
    (engine) => {
      useProjectStore.setState({ projects: [{ ...PROJECT, engine }] });
      useSettingsStore.setState({
        settings: { ...DEFAULT_SETTINGS, developerTools: true },
      });
      const onDebugExport = renderToolbar();

      const button = debugButton();
      expect(button).toBeInTheDocument();
      act(() => button?.click());
      expect(onDebugExport).toHaveBeenCalledTimes(1);
    },
  );

  it("is not offered for other engines", () => {
    useProjectStore.setState({ projects: [{ ...PROJECT, engine: "vx_ace" }] });
    useSettingsStore.setState({
      settings: { ...DEFAULT_SETTINGS, developerTools: true },
    });
    renderToolbar();
    expect(debugButton()).not.toBeInTheDocument();
  });
});
