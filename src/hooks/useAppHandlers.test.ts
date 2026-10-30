import { beforeEach, describe, expect, it, vi } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";
import { mockIPC } from "@tauri-apps/api/mocks";

vi.mock("sonner", () => ({
  toast: { success: vi.fn(), error: vi.fn() },
}));

import { toast } from "sonner";
import { integrityIssuesOf, useAppHandlers } from "@/hooks/useAppHandlers";
import { useProjectStore } from "@/stores/project";
import i18n from "@/lib/i18n";
import type {
  DebugExport,
  ExportPreflight,
  FitScan,
  ProjectStats,
  TextFitView,
} from "@/lib/types";

const initialProject = useProjectStore.getState();

function statsWith(untranslated: number): ProjectStats {
  return {
    fileCount: 1,
    totalSegments: 10,
    untranslatedCount: untranslated,
    translatedCount: 10 - untranslated,
    needsReviewCount: 0,
    reviewedCount: 0,
    resumableCount: 0,
  };
}

const PREFLIGHT: ExportPreflight = {
  locale: {
    current: "ja_JP",
    target: "en_US",
    consequences: ["Sets System.json `locale` from ja_JP to en_US."],
    uses: [{ location: "js/rmmz_objects.js:7", code: "$gameSystem.isCJK()" }],
  },
};

const DEBUG_RESULT: DebugExport = {
  zipPath: "C:/game/hoshi2star-debug.zip",
  exported: 6,
  untranslated: 3,
  skippedCritical: 1,
};

const INTEGRITY_ERROR =
  "Export blocked: the integrity check found 2 problem(s); no file was written.\n" +
  "data/System.json /gameTitle: unplanned change\n" +
  "js/plugins.js line 4: unplanned change";

interface MockOptions {
  untranslated: number;
  statsError?: string;
  preflight?: ExportPreflight | Error;
  exportResult?: string | Error;
  debugResult?: DebugExport | Error;
  fit?: { view: TextFitView; scan: FitScan };
}

/** Mock the backend; returns the arguments of every `export_project` and
 * `export_debug_project` call, with the command name. */
function mockBackend({
  untranslated,
  statsError,
  preflight,
  exportResult,
  debugResult,
  fit,
}: MockOptions) {
  const exportCalls: unknown[] = [];
  mockIPC((cmd, args) => {
    switch (cmd) {
      case "get_project_stats":
        if (statsError) throw statsError;
        return statsWith(untranslated);
      case "get_text_fit":
        if (!fit) throw new Error("text fitting not mocked");
        return fit.view;
      case "scan_text_fit":
        return fit?.scan;
      case "preflight_export":
        if (preflight instanceof Error) throw preflight;
        return preflight ?? { locale: null };
      case "export_project":
        exportCalls.push(args);
        if (exportResult instanceof Error) throw exportResult.message;
        return exportResult ?? "C:/game/hoshi2star.zip";
      case "export_debug_project":
        exportCalls.push({ cmd, ...(args as object) });
        if (debugResult instanceof Error) throw debugResult.message;
        return debugResult ?? DEBUG_RESULT;
      case "plugin:event|listen":
        return 1;
      case "plugin:event|unlisten":
        return undefined;
      default:
        throw new Error(`unexpected command: ${cmd}`);
    }
  });
  return exportCalls;
}

beforeEach(() => {
  vi.clearAllMocks();
  useProjectStore.setState(initialProject, true);
  useProjectStore.setState({ activeProjectId: "p1" });
});

describe("useAppHandlers — export gate", () => {
  it("blocks the export dialog while untranslated segments remain", async () => {
    mockBackend({ untranslated: 3 });
    const { result } = renderHook(() => useAppHandlers());

    await act(async () => {
      await result.current.handleExportAll();
    });

    await waitFor(() => expect(result.current.exportDialog).toBe("blocked"));
    expect(result.current.exportStats?.untranslatedCount).toBe(3);
  });

  it("opens the confirm dialog when everything is translated", async () => {
    mockBackend({ untranslated: 0 });
    const { result } = renderHook(() => useAppHandlers());

    await act(async () => {
      await result.current.handleExportAll();
    });

    await waitFor(() => expect(result.current.exportDialog).toBe("confirm"));
  });
});

describe("useAppHandlers — language switch", () => {
  it("offers the switch off by default and sends it only when checked", async () => {
    const exportCalls = mockBackend({ untranslated: 0, preflight: PREFLIGHT });
    const { result } = renderHook(() => useAppHandlers());

    await act(async () => {
      await result.current.handleExportAll();
    });
    expect(result.current.exportPreflight).toEqual(PREFLIGHT);
    expect(result.current.switchLocale).toBe(false);

    act(() => result.current.handleExportFontApply(22, false));
    await waitFor(() => expect(exportCalls).toHaveLength(1));
    expect(exportCalls[0]).toMatchObject({ setLocale: false });

    act(() => result.current.setSwitchLocale(true));
    act(() => result.current.handleExportFontApply(22, false));
    await waitFor(() => expect(exportCalls).toHaveLength(2));
    expect(exportCalls[1]).toMatchObject({ setLocale: true });
  });

  it("exports without the option when the preflight fails", async () => {
    const exportCalls = mockBackend({
      untranslated: 0,
      preflight: new Error("no snapshot"),
    });
    const { result } = renderHook(() => useAppHandlers());

    await act(async () => {
      await result.current.handleExportAll();
    });
    expect(result.current.exportDialog).toBe("confirm");
    expect(result.current.exportPreflight).toBeNull();

    act(() => result.current.handleExportFontApply(22, false));
    await waitFor(() => expect(exportCalls).toHaveLength(1));
    expect(exportCalls[0]).toMatchObject({ setLocale: null });
  });
});

describe("useAppHandlers — integrity refusals", () => {
  it("lists the problems in a dialog instead of a toast", async () => {
    mockBackend({ untranslated: 0, exportResult: new Error(INTEGRITY_ERROR) });
    const { result } = renderHook(() => useAppHandlers());

    act(() => result.current.handleExportFontApply(22, false));
    await waitFor(() =>
      expect(result.current.integrityIssues).toEqual([
        "data/System.json /gameTitle: unplanned change",
        "js/plugins.js line 4: unplanned change",
      ]),
    );
    expect(toast.error).not.toHaveBeenCalled();
  });

  it("keeps other export errors as toasts", () => {
    expect(integrityIssuesOf("Export bloqué : 2 segment(s)")).toBeNull();
  });
});

describe("useAppHandlers — text fitting summary", () => {
  const counts = {
    total: 4,
    fits: 1,
    rewrapped: 2,
    newPage: 1,
    shrunk: 0,
    tooLong: 0,
    skipped: 0,
  };
  const scan: FitScan = {
    dialogue: counts,
    descriptions: counts,
    rows: [],
    missingChars: [],
  };
  const view = (enabledDialogue: boolean): TextFitView => ({
    supported: true,
    reason: null,
    settings: {
      enabledDialogue,
      enabledDescriptions: false,
      lineBreaks: "reflow",
      overflow: "new_page",
      minFontSize: 20,
      dialogueFontSize: null,
    },
    geometry: null,
    font: null,
    warnings: [],
    notes: [],
  });

  it("loads what fitting will do when it is enabled", async () => {
    mockBackend({ untranslated: 0, fit: { view: view(true), scan } });
    const { result } = renderHook(() => useAppHandlers());
    await act(async () => {
      await result.current.handleExportAll();
    });
    expect(result.current.exportFit).toEqual(scan);
  });

  it("shows nothing when fitting is off", async () => {
    mockBackend({ untranslated: 0, fit: { view: view(false), scan } });
    const { result } = renderHook(() => useAppHandlers());
    await act(async () => {
      await result.current.handleExportAll();
    });
    expect(result.current.exportFit).toBeNull();
  });
});

describe("useAppHandlers — debug export (developer tool)", () => {
  it("never goes through the normal export gate", async () => {
    const exportCalls = mockBackend({ untranslated: 3 });
    const { result } = renderHook(() => useAppHandlers());

    act(() => result.current.setShowDebugExport(true));
    expect(result.current.showDebugExport).toBe(true);
    await act(async () => {
      await result.current.handleDebugExportConfirm();
    });

    expect(result.current.showDebugExport).toBe(false);
    expect(result.current.exportDialog).toBeNull();
    expect(exportCalls).toHaveLength(1);
    expect(exportCalls[0]).toMatchObject({
      cmd: "export_debug_project",
      projectId: "p1",
    });
    expect(toast.success).toHaveBeenCalledWith(
      expect.any(String),
      expect.objectContaining({
        description: expect.stringContaining("C:/game/hoshi2star-debug.zip"),
      }),
    );
  });

  it("lists integrity problems in the dialog, as the normal export does", async () => {
    mockBackend({ untranslated: 3, debugResult: new Error(INTEGRITY_ERROR) });
    const { result } = renderHook(() => useAppHandlers());

    await act(async () => {
      await result.current.handleDebugExportConfirm();
    });

    expect(result.current.integrityIssues).toEqual([
      "data/System.json /gameTitle: unplanned change",
      "js/plugins.js line 4: unplanned change",
    ]);
    expect(toast.success).not.toHaveBeenCalled();
    expect(toast.error).not.toHaveBeenCalled();
  });

  it("reports other refusals as a toast", async () => {
    mockBackend({
      untranslated: 10,
      debugResult: new Error("Debug export: no translation"),
    });
    const { result } = renderHook(() => useAppHandlers());

    await act(async () => {
      await result.current.handleDebugExportConfirm();
    });

    expect(toast.error).toHaveBeenCalledTimes(1);
    expect(result.current.isExporting).toBe(false);
  });
});

describe("useAppHandlers — Translate all (audit F2)", () => {
  it("reports a failure to prepare the run, not an export error", async () => {
    mockBackend({ untranslated: 0, statsError: "db locked" });
    const { result } = renderHook(() => useAppHandlers());

    await act(async () => {
      await result.current.handleTranslateAll();
    });

    expect(toast.error).toHaveBeenCalledWith(
      i18n.t("toasts.translateAllError", { error: "db locked" }),
    );
    expect(result.current.showTranslateAll).toBe(false);
  });
});
