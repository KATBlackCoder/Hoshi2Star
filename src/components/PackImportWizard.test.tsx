import { beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";

const toastError = vi.fn();
vi.mock("sonner", () => ({
  toast: {
    success: vi.fn(),
    error: (...args: unknown[]) => toastError(...args),
  },
}));

import { PackImportWizard } from "@/components/PackImportWizard";
import type { ImportPreview, ImportReport } from "@/lib/types";

function makePreview(overrides: Partial<ImportPreview> = {}): ImportPreview {
  return {
    blocker: null,
    packGameTitle: "Some Game",
    packAppVersion: "0.4.9",
    packCreatedAt: "2026-07-07T10:00:00Z",
    titleMismatch: false,
    applicable: 5,
    identical: 10,
    conflicts: 2,
    sourceChanged: 0,
    orphans: 1,
    terminologyCount: 0,
    terminologyCreates: 0,
    terminologyUpdates: 0,
    terminologyConflicts: 0,
    tmCount: 0,
    ...overrides,
  };
}

const report: ImportReport = {
  applied: 5,
  appliedSourceChanged: 0,
  skippedConflicts: 2,
  skippedSourceChanged: 0,
  identical: 10,
  orphans: 1,
  terminologyAdded: 0,
  terminologyUpdated: 0,
  terminologyConflicts: 0,
  tmAdded: 0,
  backupPath: "/data/backups/backup-avant-import-x.h2s",
};

describe("PackImportWizard", () => {
  // jsdom lacks the pointer-capture / scroll APIs Radix Select relies on.
  beforeAll(() => {
    window.HTMLElement.prototype.hasPointerCapture = vi.fn();
    window.HTMLElement.prototype.releasePointerCapture = vi.fn();
    window.HTMLElement.prototype.scrollIntoView = vi.fn();
  });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("shows the dry-run counts then the final report after apply", async () => {
    const invoked: Array<{ cmd: string; args: unknown }> = [];
    mockIPC((cmd, args) => {
      invoked.push({ cmd, args });
      if (cmd === "apply_h2s_import") return Promise.resolve(report);
      return Promise.resolve(null);
    });

    render(
      <PackImportWizard
        projectId="p1"
        packPath="/tmp/pack.h2s"
        preview={makePreview()}
        onClose={() => {}}
      />,
    );

    // Dry-run counts are visible before anything is written
    expect(screen.getByText("Importer un pack (.h2s)")).toBeInTheDocument();
    expect(
      screen.getByText("Applicables (cible locale vide)"),
    ).toBeInTheDocument();
    expect(screen.getByText("5")).toBeInTheDocument();

    await userEvent.click(screen.getByText("Appliquer"));

    // apply_h2s_import called with the safest default policy
    const apply = invoked.find((c) => c.cmd === "apply_h2s_import");
    expect(apply).toBeDefined();
    expect(apply!.args).toMatchObject({
      projectId: "p1",
      packPath: "/tmp/pack.h2s",
      policy: "fill",
      applySourceChanged: false,
    });

    // Wizard switched to the final report
    expect(await screen.findByText("Import terminé")).toBeInTheDocument();
    expect(screen.getByText("Segments appliqués")).toBeInTheDocument();
    expect(screen.getByText(/backup-avant-import-x\.h2s/)).toBeInTheDocument();
  });

  it("blocks apply for 'overwrite everything' until the confirmation is checked", async () => {
    mockIPC(() => Promise.resolve(null));
    render(
      <PackImportWizard
        projectId="p1"
        packPath="/tmp/pack.h2s"
        preview={makePreview()}
        onClose={() => {}}
      />,
    );

    // Radix Select in jsdom: interact via the hidden native select fallback
    // is unreliable — drive the trigger + option instead.
    await userEvent.click(screen.getByRole("combobox"));
    await userEvent.click(
      await screen.findByRole("option", { name: "Tout écraser" }),
    );

    const apply = screen.getByText("Appliquer").closest("button")!;
    expect(apply).toBeDisabled();

    await userEvent.click(screen.getByRole("checkbox"));
    expect(apply).toBeEnabled();
  });

  it("shows terminology creations, updates and locked conflicts before apply", () => {
    mockIPC(() => Promise.resolve(null));
    render(
      <PackImportWizard
        projectId="p1"
        packPath="/tmp/pack.h2s"
        preview={makePreview({
          terminologyCount: 6,
          terminologyCreates: 3,
          terminologyUpdates: 2,
          terminologyConflicts: 1,
        })}
        onClose={() => {}}
      />,
    );

    expect(
      screen.getByText("Conséquences terminologiques"),
    ).toBeInTheDocument();
    expect(screen.getByText("Nouveaux termes")).toBeInTheDocument();
    expect(screen.getByText("Termes mis à jour")).toBeInTheDocument();
    expect(
      screen.getByText("Conflits verrouillés (conservés)"),
    ).toBeInTheDocument();
  });

  it("shows a dedicated message when the pack is refused (engine mismatch)", () => {
    mockIPC(() => Promise.resolve(null));
    render(
      <PackImportWizard
        projectId="p1"
        packPath="/tmp/pack.h2s"
        preview={makePreview({
          blocker: {
            type: "engineMismatch",
            packEngine: "wolf",
            projectEngine: "mv_mz",
          },
        })}
        onClose={() => {}}
      />,
    );

    expect(
      screen.getByText("Ce pack ne peut pas être importé"),
    ).toBeInTheDocument();
    expect(screen.getByText(/wolf/)).toBeInTheDocument();
    expect(screen.queryByText("Appliquer")).not.toBeInTheDocument();
  });
});
