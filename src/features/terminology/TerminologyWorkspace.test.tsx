import type { ReactNode } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

import { TerminologyWorkspace } from "./TerminologyWorkspace";
import type {
  PaginatedTerminology,
  Project,
  TerminologyEntry,
  TerminologyStats,
} from "@/lib/types";
import { useProjectStore } from "@/stores/project";
import { useTerminologyUiStore } from "./terminologyUiStore";

const PROJECT: Project = {
  id: "p1",
  name: "Moon",
  engine: "mv_mz",
  gamePath: "/tmp/moon",
  sourceLang: "ja",
  targetLang: "en",
  createdAt: "",
  updatedAt: "",
};
const ENTRY: TerminologyEntry = {
  id: "e1",
  sourceLanguage: "ja",
  canonicalText: "勇者",
  normalizedText: "勇者",
  reading: "ユウシャ",
  partOfSpeech: "proper_noun",
  semanticType: "character",
  senseKey: "",
  status: "active",
  origin: "engine",
  confidence: 1,
  occurrenceCount: 12,
  hasProjectTranslation: true,
  hasGlobalTranslation: false,
  translation: {
    id: "t1",
    targetLanguage: "en",
    projectId: "p1",
    targetText: "Hero",
    enforcement: "required",
    confidence: 1,
    providerId: null,
    model: null,
    acceptedVariants: [],
  },
  contexts: [],
};
const SECOND_ENTRY: TerminologyEntry = {
  ...ENTRY,
  id: "e2",
  canonicalText: "魔王",
  normalizedText: "魔王",
  reading: "マオウ",
  occurrenceCount: 4,
  translation: null,
};
const STATS: TerminologyStats = {
  totalEntries: 1,
  untranslatedEntries: 0,
  projectTranslations: 1,
  globalTranslations: 0,
};

function wrapper({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider
      client={
        new QueryClient({ defaultOptions: { queries: { retry: false } } })
      }
    >
      {children}
    </QueryClientProvider>
  );
}

beforeEach(() => {
  useProjectStore.setState({ projects: [PROJECT], activeProjectId: "p1" });
  useTerminologyUiStore.setState({
    search: "",
    partOfSpeech: "all",
    semanticType: "",
    status: "active",
    scope: "project",
    page: 0,
    selectedIds: [],
  });
});

describe("TerminologyWorkspace", () => {
  it("shows the empty state and keeps scan explicit", async () => {
    mockIPC((cmd) =>
      cmd === "list_terminology"
        ? ({
            items: [],
            total: 0,
            page: 0,
            pageSize: 100,
          } satisfies PaginatedTerminology)
        : cmd === "get_terminology_stats"
          ? STATS
          : null,
    );
    render(<TerminologyWorkspace />, { wrapper });
    expect(
      await screen.findByText("Aucun terme dans cette vue"),
    ).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: "Analyser le vocabulaire" }),
    ).toBeEnabled();
  });

  it("renders one bounded server page and supports selection and pagination", async () => {
    const calls: unknown[] = [];
    mockIPC((cmd, args) => {
      if (cmd === "get_terminology_stats") return STATS;
      if (cmd === "list_terminology") {
        calls.push(args);
        return { items: [ENTRY], total: 201, page: 0, pageSize: 100 };
      }
      return null;
    });
    render(<TerminologyWorkspace />, { wrapper });
    expect(await screen.findByText("Hero")).toBeInTheDocument();
    await userEvent.click(
      screen.getByRole("checkbox", { name: "Sélectionner 勇者" }),
    );
    expect(
      screen.getByText("Termes : 201 · Sélection : 1"),
    ).toBeInTheDocument();
    await userEvent.click(
      screen.getByRole("button", { name: "Page suivante" }),
    );
    await waitFor(() =>
      expect(
        calls.some((call) => JSON.stringify(call).includes('"page":1')),
      ).toBe(true),
    );
  });

  it("selects the visible page and exposes a confirmed bulk delete", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_terminology_stats") return STATS;
      if (cmd === "list_terminology") {
        return {
          items: [ENTRY, SECOND_ENTRY],
          total: 2,
          page: 0,
          pageSize: 100,
        };
      }
      return null;
    });
    render(<TerminologyWorkspace />, { wrapper });

    await screen.findByText("Hero");
    await userEvent.click(
      screen.getByRole("checkbox", { name: "Sélectionner toute la page" }),
    );

    expect(screen.getByText("Termes : 2 · Sélection : 2")).toBeInTheDocument();
    expect(
      screen.getByRole("checkbox", { name: "Sélectionner 勇者" }),
    ).toBeChecked();
    expect(
      screen.getByRole("checkbox", { name: "Sélectionner 魔王" }),
    ).toBeChecked();

    await userEvent.click(
      screen.getByRole("button", { name: "Supprimer la sélection (2)" }),
    );
    expect(
      screen.getByRole("alertdialog", {
        name: "Supprimer les termes sélectionnés ?",
      }),
    ).toHaveTextContent("2 termes");
  });

  it("opens translation for one row and permanently deletes it after confirmation", async () => {
    const deleteCalls: unknown[] = [];
    mockIPC((cmd, args) => {
      if (cmd === "get_terminology_stats") return STATS;
      if (cmd === "list_terminology") {
        return { items: [ENTRY], total: 1, page: 0, pageSize: 100 };
      }
      if (cmd === "delete_terminology_entries") {
        deleteCalls.push(args);
        return 1;
      }
      return null;
    });
    render(<TerminologyWorkspace />, { wrapper });

    await screen.findByText("Hero");
    await userEvent.click(
      screen.getByRole("button", { name: "Traduire 勇者" }),
    );
    expect(
      screen.getByRole("dialog", { name: "Traduire les termes" }),
    ).toHaveTextContent("Sélection manuelle (1)");
    await userEvent.click(screen.getByRole("button", { name: "Annuler" }));

    await userEvent.click(
      screen.getByRole("button", { name: "Supprimer 勇者" }),
    );
    expect(
      screen.getByRole("alertdialog", { name: "Supprimer ce terme ?" }),
    ).toHaveTextContent("勇者");
    await userEvent.click(
      screen.getByRole("button", { name: "Supprimer définitivement" }),
    );

    await waitFor(() =>
      expect(deleteCalls).toContainEqual({ entryIds: ["e1"] }),
    );
  });

  it("applies POS and target language as server query inputs", async () => {
    const calls: unknown[] = [];
    mockIPC((cmd, args) => {
      if (cmd === "list_terminology") {
        calls.push(args);
        return { items: [], total: 0, page: 0, pageSize: 100 };
      }
      if (cmd === "get_terminology_stats") return STATS;
      return null;
    });
    render(<TerminologyWorkspace />, { wrapper });
    await screen.findByText("Aucun terme dans cette vue");
    await userEvent.selectOptions(screen.getByLabelText("Nature"), "verb");
    await userEvent.selectOptions(screen.getByLabelText("Cible"), "fr");
    await waitFor(() =>
      expect(
        calls.some((call) => {
          const text = JSON.stringify(call);
          return (
            text.includes('"partOfSpeech":"verb"') &&
            text.includes('"targetLanguage":"fr"')
          );
        }),
      ).toBe(true),
    );
  });

  it("copies a project translation globally only after confirmation", async () => {
    const globalizeCalls: unknown[] = [];
    mockIPC((cmd, args) => {
      if (cmd === "get_terminology_stats") return STATS;
      if (cmd === "list_terminology") {
        return { items: [ENTRY], total: 1, page: 0, pageSize: 100 };
      }
      if (cmd === "globalize_terminology_translations") {
        globalizeCalls.push(args);
        return { copied: 1, alreadyGlobal: 0, conflicts: 0, skipped: 0 };
      }
      return null;
    });
    render(<TerminologyWorkspace />, { wrapper });
    await screen.findByText("Hero");
    await userEvent.click(
      screen.getByRole("button", { name: "Rendre 勇者 global" }),
    );
    expect(
      screen.getByRole("alertdialog", {
        name: "Copier vers la bibliothèque globale ?",
      }),
    ).toHaveTextContent("jamais remplacée");
    await userEvent.click(
      screen.getByRole("button", { name: "Copier globalement" }),
    );
    await waitFor(() =>
      expect(globalizeCalls).toContainEqual({
        input: { entryIds: ["e1"], targetLanguage: "en", projectId: "p1" },
      }),
    );
  });

  it("reports IPC errors without losing the workspace", async () => {
    mockIPC((cmd) => {
      if (cmd === "list_terminology") throw new Error("db locked");
      if (cmd === "get_terminology_stats") return STATS;
      return null;
    });
    render(<TerminologyWorkspace />, { wrapper });
    expect(
      await screen.findByText("Impossible de charger la terminologie"),
    ).toBeInTheDocument();
    expect(screen.getByRole("alert")).toHaveTextContent("db locked");
  });

  it("disables Japanese morphology scan for a non-Japanese source", async () => {
    useProjectStore.setState({
      projects: [{ ...PROJECT, sourceLang: "en", targetLang: "fr" }],
    });
    mockIPC((cmd) =>
      cmd === "list_terminology"
        ? { items: [], total: 0, page: 0, pageSize: 100 }
        : STATS,
    );
    render(<TerminologyWorkspace />, { wrapper });
    expect(
      await screen.findByRole("button", { name: "Analyser le vocabulaire" }),
    ).toBeDisabled();
  });
});
