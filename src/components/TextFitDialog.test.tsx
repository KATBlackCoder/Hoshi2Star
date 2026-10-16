import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { mockIPC } from "@tauri-apps/api/mocks";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn(), warning: vi.fn() },
}));

import { TextFitDialog } from "@/components/TextFitDialog";
import i18n from "@/lib/i18n";
import type { FitScan, TextFitSettings, TextFitView } from "@/lib/types";
import { useEditorStore } from "@/stores/editor";
import { openTextFitDialog, useTextFitStore } from "@/stores/textFit";
import { useUiStore } from "@/stores/ui";

const SETTINGS: TextFitSettings = {
  enabledDialogue: false,
  enabledDescriptions: false,
  lineBreaks: "reflow",
  overflow: "new_page",
  minFontSize: 20,
  dialogueFontSize: null,
};

const VIEW: TextFitView = {
  supported: true,
  reason: null,
  settings: SETTINGS,
  geometry: {
    fontSize: 26,
    fontFile: "mplus-1m-regular.woff",
    boxWidth: 808,
    lineHeight: 36,
    message: { innerWidth: 784, innerHeight: 152 },
    help: { textX: 8, textWidth: 768, innerHeight: 72 },
  },
  font: { file: "mplus-1m-regular.woff", loaded: true, error: null },
  warnings: [{ plugin: "WidePlugin", hooks: ["newLineX"] }],
  notes: ["Item gain popup (TinyGetInfoWndMZ): shows line 1."],
};

const SCAN: FitScan = {
  dialogue: {
    total: 10,
    fits: 6,
    rewrapped: 3,
    newPage: 1,
    shrunk: 0,
    tooLong: 0,
    skipped: 0,
  },
  descriptions: {
    total: 2,
    fits: 1,
    rewrapped: 1,
    newPage: 0,
    shrunk: 0,
    tooLong: 0,
    skipped: 0,
  },
  rows: [
    {
      fileId: "f1",
      fileName: "Map001.json",
      pointer: "/events/1/pages/0/list/3",
      description: false,
      segmentId: "s9",
      outcome: { kind: "newPage", pages: 2 },
      sample: "A long talkative message",
    },
  ],
  missingChars: [{ character: "♪", count: 2 }],
};

function mockBackend(calls: Array<{ cmd: string; args: unknown }> = []) {
  mockIPC((cmd, args) => {
    calls.push({ cmd, args });
    switch (cmd) {
      case "get_text_fit":
        return VIEW;
      case "scan_text_fit":
        return SCAN;
      case "set_text_fit":
        return (args as { settings: TextFitSettings }).settings;
      default:
        return null;
    }
  });
  return calls;
}

const initialFit = useTextFitStore.getState();
const initialEditor = useEditorStore.getState();

describe("TextFitDialog", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useTextFitStore.setState(initialFit, true);
    useEditorStore.setState(initialEditor, true);
  });

  it("shows the detected box, plugin warnings and the counts", async () => {
    mockBackend();
    await act(() => openTextFitDialog("p1"));
    render(<TextFitDialog projectId="p1" />);

    expect(
      screen.getByText(
        i18n.t("textFit.box", { width: 776, face: 616, lines: 4 }),
      ),
    ).toBeInTheDocument();
    expect(screen.getByText(/WidePlugin/)).toBeInTheDocument();
    expect(screen.getByText(VIEW.notes[0])).toBeInTheDocument();
    expect(
      screen.getByText(i18n.t("textFit.dialogueCounts", { ...SCAN.dialogue })),
    ).toBeInTheDocument();
    expect(screen.getByText("A long talkative message")).toBeInTheDocument();
    expect(screen.getByText(/♪ \(2\)/)).toBeInTheDocument();
  });

  it("saves the edited settings", async () => {
    const calls = mockBackend();
    await act(() => openTextFitDialog("p1"));
    render(<TextFitDialog projectId="p1" />);

    await userEvent.click(
      screen.getByRole("checkbox", {
        name: (name) => name.startsWith(i18n.t("textFit.enableDialogue")),
      }),
    );
    await userEvent.click(
      screen.getByRole("button", { name: i18n.t("textFit.save") }),
    );

    const saved = calls.find((call) => call.cmd === "set_text_fit");
    expect(saved?.args).toEqual({
      projectId: "p1",
      settings: { ...SETTINGS, enabledDialogue: true },
    });
    expect(useTextFitStore.getState().open).toBe(false);
  });

  it("opens a text to review in the editor preview", async () => {
    mockBackend();
    await act(() => openTextFitDialog("p1"));
    render(<TextFitDialog projectId="p1" />);

    await userEvent.click(
      screen.getByRole("button", { name: i18n.t("textFit.open") }),
    );

    expect(useEditorStore.getState().activeFileId).toBe("f1");
    expect(useEditorStore.getState().activeSegmentId).toBe("s9");
    expect(useUiStore.getState().inspectorTab).toBe("preview");
    expect(useTextFitStore.getState().open).toBe(false);
  });

  it("explains when the game is not supported", async () => {
    mockIPC((cmd) =>
      cmd === "get_text_fit"
        ? {
            ...VIEW,
            supported: false,
            reason: "Not an MZ game.",
            geometry: null,
            font: null,
          }
        : null,
    );
    await act(() => openTextFitDialog("p1"));
    render(<TextFitDialog projectId="p1" />);
    expect(screen.getByText("Not an MZ game.")).toBeInTheDocument();
    expect(
      screen.getByRole("button", { name: i18n.t("textFit.save") }),
    ).toBeDisabled();
  });
});
