import { renderWithQuery } from "@/test/renderWithQuery";
import { mockIPC } from "@tauri-apps/api/mocks";
import { screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  toast: { error: vi.fn(), success: vi.fn() },
}));

vi.mock("sonner", () => ({ toast: mocks.toast }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async () => () => {}),
}));

import { TerminologyWorkspace } from "./TerminologyWorkspace";
import type { Project, TerminologyEntry, TerminologyStats } from "@/lib/types";
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
const STATS: TerminologyStats = {
  totalEntries: 1,
  untranslatedEntries: 0,
  projectTranslations: 1,
  globalTranslations: 0,
};

beforeEach(() => {
  mocks.toast.error.mockClear();
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

// Plan DD, L6-1 (F1): an `Error` is shown by its message, without the
// "Error: " that `String(error)` put in front of it.
describe("TerminologyWorkspace error toast", () => {
  it("shows a failed global copy without an 'Error:' prefix", async () => {
    mockIPC((cmd) => {
      if (cmd === "get_terminology_stats") return STATS;
      if (cmd === "list_terminology") {
        return { items: [ENTRY], total: 1, page: 0, pageSize: 100 };
      }
      if (cmd === "globalize_terminology_translations") {
        throw new Error("global copy failed");
      }
      return null;
    });
    renderWithQuery(<TerminologyWorkspace />);
    await screen.findByText("Hero");
    await userEvent.click(
      screen.getByRole("button", { name: "Rendre 勇者 global" }),
    );
    await userEvent.click(
      screen.getByRole("button", { name: "Copier globalement" }),
    );
    await waitFor(() => expect(mocks.toast.error).toHaveBeenCalled());
    const shown = String(mocks.toast.error.mock.calls[0][0]);
    expect(shown).toBe("global copy failed");
    expect(shown.startsWith("Error:")).toBe(false);
  });
});
