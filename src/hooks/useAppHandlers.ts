import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { toast } from "sonner";
import { useProjectStore, usePendingGlossaryExtract } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { useLlmStore } from "@/stores/llm";
import type { FontScanResult, ProjectStats } from "@/lib/types";

// ---------------------------------------------------------------------------
// Hook
// ---------------------------------------------------------------------------

export function useAppHandlers() {
  const [showSettings, setShowSettings] = useState(false);
  const [showAbout, setShowAbout] = useState(false);
  const [exportDialog, setExportDialog] = useState<
    null | "confirm" | "blocked"
  >(null);
  const [exportStats, setExportStats] = useState<ProjectStats | null>(null);
  const [showTranslateAll, setShowTranslateAll] = useState(false);
  const [translateAllStats, setTranslateAllStats] =
    useState<ProjectStats | null>(null);
  const [showFontDialog, setShowFontDialog] = useState(false);
  const [fontScanResult, setFontScanResult] = useState<FontScanResult | null>(
    null,
  );
  const [isExporting, setIsExporting] = useState(false);

  const { t } = useTranslation();
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const activeFileId = useEditorStore((s) => s.activeFileId);
  const pendingGlossaryExtract = usePendingGlossaryExtract();
  const { startTranslation, startTranslateAll, providerConfig } = useLlmStore();
  const setPendingGlossaryExtract = useProjectStore(
    (s) => s.setPendingGlossaryExtract,
  );
  const setExtractingGlossary = useProjectStore((s) => s.setExtractingGlossary);

  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listen<{ projectId: string; terms: unknown[]; error: string | null }>(
      "h2s://glossary/extraction-done",
      (event) => {
        setExtractingGlossary(false);
        if (event.payload.error) {
          toast.error(t("glossaryPrompt.extractError"));
        } else {
          const count = event.payload.terms.length;
          toast.success(t("glossaryPrompt.extractDone", { count }));
        }
      },
    ).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, [setExtractingGlossary, t]);

  async function handleGlossaryConfirm() {
    if (!pendingGlossaryExtract) return;
    const projectId = pendingGlossaryExtract;
    setPendingGlossaryExtract(null);
    setExtractingGlossary(true);
    try {
      await invoke("extract_glossary_terms", {
        projectId,
        langPair: "ja-en",
        providerConfig,
      });
    } catch {
      setExtractingGlossary(false);
      toast.error(t("glossaryPrompt.extractError"));
    }
  }

  function handleGlossaryDecline() {
    setPendingGlossaryExtract(null);
  }

  async function handleExportAll() {
    if (!activeProjectId) return;
    try {
      const stats = await invoke<ProjectStats>("get_project_stats", {
        projectId: activeProjectId,
      });
      setExportStats(stats);
      setExportDialog(stats.untranslatedCount > 0 ? "blocked" : "confirm");
    } catch (err) {
      toast.error(t("toasts.exportError", { error: String(err) }));
    }
  }

  async function handleExportConfirm() {
    setExportDialog(null);
    if (!activeProjectId) return;
    try {
      const scan = await invoke<FontScanResult>("scan_font_status", {
        projectId: activeProjectId,
      });
      if (scan.engine === "wolf" || scan.engine === "mv_mz") {
        setFontScanResult(scan);
        setShowFontDialog(true);
        return;
      }
    } catch {
      // scan failure is non-fatal — export without font dialog
    }
    await doExport(null, false);
  }

  async function doExport(fontSize: number | null, replaceExisting: boolean) {
    if (!activeProjectId) return;
    setIsExporting(true);
    try {
      const zipPath = await invoke<string>("export_project", {
        projectId: activeProjectId,
        fontSize,
        replaceExisting,
      });
      toast.success(t("toasts.exportSuccess"), { description: zipPath });
    } catch (err) {
      toast.error(t("toasts.exportError", { error: String(err) }));
    } finally {
      setIsExporting(false);
    }
  }

  function handleExportFontApply(fontSize: number, replaceExisting: boolean) {
    setShowFontDialog(false);
    setFontScanResult(null);
    void doExport(fontSize, replaceExisting);
  }

  async function handleExportFontSkip() {
    setShowFontDialog(false);
    const scan = fontScanResult;
    setFontScanResult(null);
    setIsExporting(true);
    try {
      // Strip any prefix already in the DB (written by a previous export before the fix).
      if (activeProjectId && scan && scan.existingFontCount > 0) {
        await invoke("strip_font_prefixes", { projectId: activeProjectId });
      }
      await doExport(null, false);
    } finally {
      setIsExporting(false);
    }
  }

  async function handleTranslateAll() {
    if (!activeProjectId) return;
    if (!providerConfig.model.trim()) {
      toast.warning(t("segmentGrid.noModelConfigured"));
      setShowSettings(true);
      return;
    }
    try {
      const stats = await invoke<ProjectStats>("get_project_stats", {
        projectId: activeProjectId,
      });
      setTranslateAllStats(stats);
      setShowTranslateAll(true);
    } catch (err) {
      toast.error(t("toasts.exportError", { error: String(err) }));
    }
  }

  function handleTranslateAllStart(
    thresholdMins: number,
    cooldownMins: number,
  ) {
    setShowTranslateAll(false);
    if (!activeProjectId) return;
    void startTranslateAll(activeProjectId, thresholdMins, cooldownMins);
  }

  function handleTranslate() {
    if (!providerConfig.model.trim()) {
      toast.warning(t("segmentGrid.noModelConfigured"));
      setShowSettings(true);
      return;
    }
    void startTranslation([], activeFileId ?? undefined);
  }

  return {
    // State
    showSettings,
    showAbout,
    exportDialog,
    exportStats,
    showTranslateAll,
    translateAllStats,
    showFontDialog,
    fontScanResult,
    isExporting,
    // Setters
    setShowSettings,
    setShowAbout,
    setExportDialog,
    setShowTranslateAll,
    // Handlers
    handleGlossaryConfirm,
    handleGlossaryDecline,
    handleExportAll,
    handleExportConfirm,
    handleExportFontApply,
    handleExportFontSkip,
    handleTranslateAll,
    handleTranslateAllStart,
    handleTranslate,
  };
}
