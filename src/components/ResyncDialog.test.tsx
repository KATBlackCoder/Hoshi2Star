import { beforeEach, describe, expect, it } from "vitest";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";

import { ResyncDialog } from "@/components/ResyncDialog";
import { ExportIntegrityDialog } from "@/components/ExportIntegrityDialog";
import { openResyncDialog, useResyncStore } from "@/stores/resync";
import type { ResyncSummary } from "@/lib/types";

const PREVIEW: ResyncSummary = {
  applied: false,
  files: [
    {
      file: "Map001.json",
      moved: 3,
      added: 1,
      gone: 1,
      changedSources: 1,
      orphaned: 1,
      newFile: false,
    },
    {
      file: "Map002.json",
      moved: 0,
      added: 2,
      gone: 0,
      changedSources: 0,
      orphaned: 0,
      newFile: true,
    },
  ],
  moved: 3,
  added: 3,
  gone: 1,
  changedSources: 1,
  orphaned: 1,
  goneTranslations: [
    {
      file: "Map001.json",
      key: "/events/1/pages/0/list/3/parameters/0",
      source: "こんばんは",
      target: "Good evening",
    },
  ],
};

describe("ResyncDialog", () => {
  beforeEach(() => {
    useResyncStore.setState({
      open: false,
      loading: false,
      summary: null,
      error: null,
    });
  });

  it("previews, then applies only on request", async () => {
    const calls: unknown[] = [];
    mockIPC((cmd, args) => {
      if (cmd === "resync_project") {
        calls.push(args);
        const apply = (args as { apply: boolean }).apply;
        return { ...PREVIEW, applied: apply };
      }
      if (cmd === "get_source_files") return [];
      if (cmd === "get_project_stats") return null;
      return null;
    });
    render(<ResyncDialog projectId="p1" />);

    await openResyncDialog("p1");
    expect(await screen.findByText(/Aperçu/)).toBeInTheDocument();
    expect(
      screen.getByText(/3 déplacée\(s\), 3 nouvelle\(s\)/),
    ).toBeInTheDocument();
    expect(
      screen.getByText(/Map002\.json \(nouveau fichier\)/),
    ).toBeInTheDocument();
    expect(screen.getByText(/こんばんは → Good evening/)).toBeInTheDocument();
    expect(calls).toEqual([{ projectId: "p1", apply: false }]);

    await userEvent.click(screen.getByRole("button", { name: "Appliquer" }));
    await waitFor(() =>
      expect(screen.getByText(/Réalignement appliqué/)).toBeInTheDocument(),
    );
    expect(calls).toEqual([
      { projectId: "p1", apply: false },
      { projectId: "p1", apply: true },
    ]);
    expect(
      screen.queryByRole("button", { name: "Appliquer" }),
    ).not.toBeInTheDocument();
  });

  it("has nothing to apply when the project already matches the game", async () => {
    mockIPC((cmd) =>
      cmd === "resync_project"
        ? { ...PREVIEW, files: [], moved: 0, added: 0, gone: 0 }
        : null,
    );
    render(<ResyncDialog projectId="p1" />);
    await openResyncDialog("p1");
    expect(await screen.findByText(/rien à réaligner/)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Appliquer" })).toBeDisabled();
  });
});

describe("ExportIntegrityDialog — realignment", () => {
  const CHANGED =
    'data/Map001.json /events/1/pages/0/list/3/parameters/0: source changed since extraction; expected "どこ", found "Actor1"';

  it("offers to realign when the game changed", async () => {
    let resynced = 0;
    let closed = 0;
    render(
      <ExportIntegrityDialog
        issues={[CHANGED]}
        onClose={() => closed++}
        onResync={() => resynced++}
      />,
    );
    await userEvent.click(
      screen.getByRole("button", { name: "Réaligner le projet…" }),
    );
    expect(resynced).toBe(1);
    expect(closed).toBe(1);
  });

  it("does not offer it for other integrity problems", () => {
    render(
      <ExportIntegrityDialog
        issues={["data/System.json /gameTitle: unplanned text change"]}
        onClose={() => {}}
        onResync={() => {}}
      />,
    );
    expect(
      screen.queryByRole("button", { name: "Réaligner le projet…" }),
    ).not.toBeInTheDocument();
  });
});
