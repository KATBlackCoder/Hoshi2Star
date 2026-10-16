import { beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn() },
}));

import { TextCoverageDialog } from "@/components/TextCoverageDialog";
import { useCoverageStore } from "@/stores/coverage";
import type { CoverageCategory, CoverageView } from "@/lib/types";

function category(overrides: Partial<CoverageCategory>): CoverageCategory {
  return {
    key: "system.equip_types",
    title: "Equipment slot names",
    group: "system",
    state: "verified",
    enabled: false,
    decided: false,
    defaultEnabled: true,
    visibleIn: "Equip screen slot labels",
    consequences: ["Translates the equipTypes list."],
    constraints: [{ rule: "no_escape_codes" }],
    segmentCount: 2,
    translatedCount: 0,
    samples: ["武器", "衣装"],
    references: [],
    referenceCount: 0,
    blockedReason: null,
    ...overrides,
  };
}

const VIEW: CoverageView = {
  supported: true,
  hasDecisions: false,
  reconstructed: true,
  driftedFiles: [],
  pluginsUnreadable: null,
  coreWarnings: [],
  notCovered: [],
  categories: [
    category({}),
    category({
      key: "system.skill_types",
      title: "Skill type names",
      state: "blocked",
      defaultEnabled: false,
      blockedReason: "1 of these texts is used as an identifier.",
      referenceCount: 1,
      references: [
        {
          text: "鞭",
          site: "note",
          location: "Weapons.json /2/note",
          excerpt: "<コマンド:スキル/鞭,48,鞭攻撃,76>",
        },
      ],
    }),
    category({
      key: "keke.skill_links",
      title: "Linked skill names",
      state: "linked",
      defaultEnabled: false,
      consequences: ["Rewrites 25 note tags in the same export."],
    }),
  ],
};

function openWith(view: CoverageView) {
  useCoverageStore.setState({ open: true, loading: false, view, error: null });
}

describe("TextCoverageDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("groups categories by safety and never lets a blocked one be enabled", async () => {
    mockIPC(() => Promise.resolve(null));
    openWith(VIEW);
    render(<TextCoverageDialog projectId="p1" />);

    expect(screen.getByText("Traduction sûre")).toBeInTheDocument();
    expect(screen.getByText("Traduction impossible sans risque")).toBeInTheDocument();
    expect(screen.getByText("Modifications liées nécessaires")).toBeInTheDocument();
    // Known keys are localized; the reconstructed-folder notice is shown.
    expect(screen.getByText("Noms des emplacements d'équipement")).toBeInTheDocument();
    expect(screen.getByText(/contenait déjà un export Hoshi2Star/)).toBeInTheDocument();

    const blocked = screen.getByRole("checkbox", { name: "Noms des types de compétence" });
    expect(blocked).toBeDisabled();
    // Verified + undecided defaults to checked.
    expect(
      screen.getByRole("checkbox", { name: "Noms des emplacements d'équipement" }),
    ).toHaveAttribute("data-state", "checked");
  });

  it("shows where a blocked text is used as an identifier", async () => {
    mockIPC(() => Promise.resolve(null));
    openWith(VIEW);
    render(<TextCoverageDialog projectId="p1" />);

    const row = screen
      .getByRole("checkbox", { name: "Noms des types de compétence" })
      .closest("li")!;
    await userEvent.click(within(row).getByText("Afficher les détails"));

    expect(within(row).getByText("Weapons.json /2/note")).toBeInTheDocument();
    expect(within(row).getByText(/スキル\/鞭/)).toBeInTheDocument();
    expect(within(row).getByText(/balise de note/)).toBeInTheDocument();
  });

  it("saves verified defaults without sending blocked categories", async () => {
    const calls: Array<{ cmd: string; args: unknown }> = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      if (cmd === "set_text_categories") return { ...VIEW, hasDecisions: true };
      if (cmd === "get_source_files") return [];
      if (cmd === "get_project_stats") return null;
      return null;
    });
    openWith(VIEW);
    render(<TextCoverageDialog projectId="p1" />);

    await userEvent.click(screen.getByRole("button", { name: "Enregistrer" }));

    const apply = calls.find((call) => call.cmd === "set_text_categories");
    expect(apply?.args).toEqual({
      projectId: "p1",
      changes: [
        { categoryKey: "system.equip_types", enabled: true, acceptConsequences: false },
        { categoryKey: "keke.skill_links", enabled: false, acceptConsequences: false },
      ],
    });
  });

  it("requires explicit acceptance before enabling a linked category", async () => {
    const calls: Array<{ cmd: string; args: unknown }> = [];
    mockIPC((cmd, args) => {
      calls.push({ cmd, args });
      if (cmd === "set_text_categories") return { ...VIEW, hasDecisions: true };
      return null;
    });
    openWith(VIEW);
    render(<TextCoverageDialog projectId="p1" />);

    await userEvent.click(screen.getByRole("checkbox", { name: "Linked skill names" }));
    await userEvent.click(screen.getByRole("button", { name: "Enregistrer" }));

    // Nothing is written before the consequences are acknowledged.
    expect(calls.some((call) => call.cmd === "set_text_categories")).toBe(false);
    const confirm = screen.getByRole("alertdialog");
    expect(within(confirm).getByText("Activer les modifications liées ?")).toBeInTheDocument();
    expect(
      within(confirm).getByText("Rewrites 25 note tags in the same export."),
    ).toBeInTheDocument();

    await userEvent.click(screen.getByRole("button", { name: "J'ai compris, activer" }));

    const apply = calls.find((call) => call.cmd === "set_text_categories");
    expect(apply?.args).toMatchObject({
      changes: expect.arrayContaining([
        { categoryKey: "keke.skill_links", enabled: true, acceptConsequences: true },
      ]),
    });
  });
});
