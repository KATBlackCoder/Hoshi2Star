import { render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { expect, it, vi } from "vitest";
import { TermTranslateDialog } from "./TermTranslateDialog";
import { terminologyApi } from "./api";
import type { ProviderConfig, TerminologyEntry } from "@/lib/types";

const provider: ProviderConfig = {
  providerId: "ollama",
  url: "http://localhost:11434/v1",
  model: "qwen-test",
  batchSize: 20,
  resourceProfile: "balanced",
};
const entries: TerminologyEntry[] = [
  {
    id: "new",
    sourceLanguage: "ja",
    canonicalText: "勇者",
    normalizedText: "勇者",
    reading: null,
    partOfSpeech: "noun",
    semanticType: "character",
    senseKey: "",
    status: "active",
    origin: "engine",
    confidence: 1,
    occurrenceCount: 2,
    translation: null,
    contexts: [],
  },
  {
    id: "done",
    sourceLanguage: "ja",
    canonicalText: "剣",
    normalizedText: "剣",
    reading: null,
    partOfSpeech: "noun",
    semanticType: "item",
    senseKey: "",
    status: "active",
    origin: "engine",
    confidence: 1,
    occurrenceCount: 1,
    translation: {
      id: "translation",
      targetLanguage: "en",
      projectId: null,
      targetText: "Sword",
      reviewStatus: "approved",
      enforcement: "preferred",
      confidence: 1,
      providerId: null,
      model: null,
      acceptedVariants: [],
    },
    contexts: [],
  },
];

it("estimates tokens and sends only untranslated terms as proposals", async () => {
  let doneHandler:
    | ((payload: {
        jobId: string;
        summary: { proposed: number } | null;
        error: string | null;
      }) => void)
    | undefined;
  vi.spyOn(terminologyApi, "onTranslateProgress").mockResolvedValue(() => {});
  vi.spyOn(terminologyApi, "onTranslateDone").mockImplementation(
    async (handler) => {
      doneHandler = handler;
      return () => {};
    },
  );
  const translate = vi
    .spyOn(terminologyApi, "translate")
    .mockResolvedValue({ jobId: "job" });
  const onDone = vi.fn();
  render(
    <TermTranslateDialog
      open
      entries={entries}
      selectedIds={[]}
      targetLanguage="en"
      projectId="p1"
      providerConfig={provider}
      onOpenChange={vi.fn()}
      onDone={onDone}
    />,
  );
  expect(screen.getByText(/≈ \d+ tokens/)).toBeInTheDocument();
  expect(screen.getByText("qwen-test")).toBeInTheDocument();
  await userEvent.click(
    screen.getByRole("button", { name: "Créer les propositions" }),
  );
  await waitFor(() =>
    expect(translate).toHaveBeenCalledWith(
      expect.objectContaining({ entryIds: ["new"], targetLanguage: "en" }),
    ),
  );
  doneHandler?.({ jobId: "job", summary: { proposed: 1 }, error: null });
  expect(onDone).toHaveBeenCalledOnce();
});
