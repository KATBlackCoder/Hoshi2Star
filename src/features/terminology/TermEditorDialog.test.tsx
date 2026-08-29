import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { TermEditorDialog } from "./TermEditorDialog";
import type { TerminologyEntry } from "@/lib/types";

const entry: TerminologyEntry = {
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
  occurrenceCount: 1,
  hasProjectTranslation: false,
  hasGlobalTranslation: true,
  contexts: [],
  translation: {
    id: "t1",
    targetLanguage: "en",
    projectId: null,
    targetText: "Hero",
    enforcement: "preferred",
    confidence: 0.8,
    providerId: null,
    model: null,
    acceptedVariants: [],
  },
};

it("edits a usable term and its enforcement only after explicit save", async () => {
  const onSave = vi.fn();
  render(
    <TermEditorDialog
      open
      entry={entry}
      targetLanguage="en"
      onOpenChange={vi.fn()}
      onSave={onSave}
      saving={false}
    />,
  );
  const target = screen.getByLabelText("Traduction EN");
  await userEvent.clear(target);
  await userEvent.type(target, "Champion");
  await userEvent.selectOptions(
    screen.getByLabelText("Application"),
    "required",
  );
  expect(onSave).not.toHaveBeenCalled();
  await userEvent.click(screen.getByRole("button", { name: "Enregistrer" }));
  expect(onSave).toHaveBeenCalledWith(
    expect.objectContaining({
      targetText: "Champion",
      enforcement: "required",
    }),
  );
});
