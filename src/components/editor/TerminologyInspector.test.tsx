import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { mockIPC } from "@tauri-apps/api/mocks";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it } from "vitest";
import { TerminologyInspector } from "@/components/editor/TerminologyInspector";
import { useTerminologyUiStore } from "@/features/terminology/terminologyUiStore";
import { useEditorStore } from "@/stores/editor";
import { useUiStore } from "@/stores/ui";

beforeEach(() => {
  useEditorStore.setState({ activeSegmentId: "s1" });
  useUiStore.setState({ mode: "patch" });
  useTerminologyUiStore.setState({ search: "", scope: "project" });
});

describe("TerminologyInspector", () => {
  it("shows only the active segment rules and opens the filtered library", async () => {
    mockIPC((command, args) => {
      if (command !== "get_segment_terminology") {
        throw new Error(`unexpected command: ${command}`);
      }
      expect(args).toMatchObject({ segmentId: "s1", targetLanguage: "en" });
      return [
        {
          entryId: "e1",
          source: "勇者",
          target: "Hero",
          semanticType: "character",
          partOfSpeech: "proper_noun",
          reviewStatus: "locked",
          enforcement: "required",
          acceptedTargets: ["The Hero"],
        },
      ];
    });
    const client = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    render(
      <QueryClientProvider client={client}>
        <TerminologyInspector projectId="p1" langPair="ja-en" />
      </QueryClientProvider>,
    );

    expect(await screen.findByText("勇者")).toBeInTheDocument();
    expect(screen.getByText("Hero")).toBeInTheDocument();
    await userEvent.click(
      screen.getByRole("button", {
        name: /Ouvrir le terme 勇者 dans la bibliothèque/,
      }),
    );
    expect(useUiStore.getState().mode).toBe("terminology");
    expect(useTerminologyUiStore.getState().search).toBe("勇者");
  });
});
