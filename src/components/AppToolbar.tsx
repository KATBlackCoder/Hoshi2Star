import { useState, useEffect } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { openProjectErrorKey } from "@/lib/projectError";
import { invoke } from "@tauri-apps/api/core";
import {
  Clock,
  Download,
  FolderOpen,
  FlaskConical,
  Info,
  Languages,
  Loader2,
  PackageOpen,
  Play,
  Share2,
  Snowflake,
} from "lucide-react";
import { openProject, useProjectStore } from "@/stores/project";
import {
  useIsTranslating,
  useTranslationProgress,
  useTranslationStartTime,
  useIsCooling,
  useCooldownRemaining,
} from "@/stores/llm";
import { UpdateBadge } from "@/components/UpdateBadge";
import { PackExportDialog } from "@/components/PackExportDialog";
import { PackImportWizard } from "@/components/PackImportWizard";
import type { ImportPreview } from "@/lib/types";
import { usePilotStore } from "@/stores/pilot";
import { useSettingsStore } from "@/stores/settings";

// ---------------------------------------------------------------------------
// Translation timer
// ---------------------------------------------------------------------------

function TranslationTimer() {
  const startTime = useTranslationStartTime();
  const isTranslating = useIsTranslating();
  const [elapsed, setElapsed] = useState(0);

  useEffect(() => {
    if (!startTime) {
      setElapsed(0);
      return;
    }
    const interval = setInterval(() => {
      setElapsed(Math.floor((Date.now() - startTime) / 1000));
    }, 1000);
    return () => clearInterval(interval);
  }, [startTime]);

  if (!startTime) return null;

  const mm = String(Math.floor(elapsed / 60)).padStart(2, "0");
  const ss = String(elapsed % 60).padStart(2, "0");

  return (
    <div
      className={`flex items-center gap-1 font-mono text-xs tabular-nums ${
        isTranslating ? "text-muted-foreground" : "text-green-400"
      }`}
    >
      <Clock className="h-3 w-3 shrink-0" />
      {mm}:{ss}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Cooldown badge
// ---------------------------------------------------------------------------

function CooldownBadge() {
  const isCooling = useIsCooling();
  const remaining = useCooldownRemaining();
  const { t } = useTranslation();

  if (!isCooling) return null;

  const mm = String(Math.floor(remaining / 60)).padStart(2, "0");
  const ss = String(remaining % 60).padStart(2, "0");

  return (
    <div className="flex items-center gap-1 font-mono text-xs tabular-nums text-blue-400">
      <Snowflake className="h-3 w-3 shrink-0 animate-pulse" />
      {t("toolbar.translateAllCooling", { remaining: `${mm}:${ss}` })}
    </div>
  );
}

// ---------------------------------------------------------------------------
// Constellation progress
// ---------------------------------------------------------------------------

const CONSTELLATION_NODES = [8, 26, 45, 78, 94];

function ConstellationProgress({ progress }: { progress: number }) {
  return (
    <div className="relative h-[22px] w-[170px]">
      <div className="absolute left-0 right-0 top-1/2 h-0.5 -translate-y-1/2 rounded-full bg-primary/15" />
      <div
        className="absolute left-0 top-1/2 h-0.5 -translate-y-1/2 rounded-full bg-gradient-to-r from-primary to-star shadow-[0_0_8px_var(--star)] transition-[width] duration-300"
        style={{ width: `${progress}%` }}
      />
      {CONSTELLATION_NODES.map((pos) => (
        <div
          key={pos}
          className={cn(
            "absolute top-1/2 h-[5px] w-[5px] -translate-x-1/2 -translate-y-1/2 rotate-45 rounded-[1px]",
            pos <= progress
              ? "bg-star shadow-[0_0_6px_var(--star)]"
              : "bg-muted-foreground/30",
          )}
          style={{ left: `${pos}%` }}
        />
      ))}
      <div
        className="absolute top-1/2 -translate-x-1/2 -translate-y-1/2 animate-pulse text-[13px] text-star [text-shadow:0_0_10px_var(--star)] transition-[left] duration-300"
        style={{ left: `${progress}%` }}
      >
        ★
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// AppToolbar
// ---------------------------------------------------------------------------

interface AppToolbarProps {
  onOpenAbout: () => void;
  onTranslate: () => void;
  onTranslateAll: () => void;
  onExportAll: () => void;
  onOpenPilot: () => void;
  isExporting: boolean;
}

export function AppToolbar({
  onOpenAbout,
  onTranslate,
  onTranslateAll,
  onExportAll,
  onOpenPilot,
  isExporting,
}: AppToolbarProps) {
  const { t } = useTranslation();
  const [isOpening, setIsOpening] = useState(false);
  const [showPackExport, setShowPackExport] = useState(false);
  const [isPreviewingPack, setIsPreviewingPack] = useState(false);
  const [packImport, setPackImport] = useState<{
    packPath: string;
    preview: ImportPreview;
  } | null>(null);
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const activeProject = useProjectStore((s) =>
    s.projects.find((p) => p.id === s.activeProjectId),
  );
  const isTranslating = useIsTranslating();
  const progress = useTranslationProgress();
  const isPilotOpen = usePilotStore((state) => state.isOpen);
  const isPilotRunning = usePilotStore((state) => state.isRunning);
  const developerTools = useSettingsStore(
    (state) => state.settings.developerTools,
  );

  async function handleOpenGame() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t("toolbar.openGame"),
    });
    if (!selected) return;

    setIsOpening(true);
    try {
      await openProject(selected as string);
    } catch (err) {
      toast.error(t(openProjectErrorKey(err)));
    } finally {
      setIsOpening(false);
    }
  }

  // Import a .h2s pack: pick the file, run the mandatory dry-run, then hand
  // the preview to the wizard (which owns policy choice + apply + report).
  async function handleImportPack() {
    if (!activeProjectId) return;
    const selected = await open({
      multiple: false,
      title: t("pack.importPick"),
      filters: [{ name: "Hoshi2Star pack", extensions: ["h2s"] }],
    });
    if (!selected) return;
    setIsPreviewingPack(true);
    try {
      const preview = await invoke<ImportPreview>("preview_h2s_import", {
        projectId: activeProjectId,
        packPath: selected as string,
        langPair: activeProject
          ? `${activeProject.sourceLang}-${activeProject.targetLang}`
          : "ja-fr",
      });
      setPackImport({ packPath: selected as string, preview });
    } catch (err) {
      toast.error(t("pack.importError", { error: String(err) }));
    } finally {
      setIsPreviewingPack(false);
    }
  }

  return (
    <div className="flex h-10 shrink-0 items-center gap-3 border-b px-3">
      <Button
        size="sm"
        variant="outline"
        className="h-7 gap-1.5 text-xs"
        onClick={() => void handleOpenGame()}
        disabled={isOpening}
      >
        {isOpening ? (
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
        ) : (
          <FolderOpen className="h-3.5 w-3.5" />
        )}
        {t("toolbar.openGame")}
      </Button>

      {activeProjectId && (
        <Button
          size="sm"
          className="h-7 gap-1.5 text-xs shadow-[0_0_12px_oklch(0.65_0.18_285/35%)]"
          onClick={onTranslate}
          disabled={isTranslating || isPilotRunning}
        >
          {isTranslating ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <Play className="h-3.5 w-3.5" />
          )}
          {isTranslating
            ? `${t("toolbar.translating")} ${progress > 0 ? `${progress}%` : ""}`
            : t("toolbar.translate")}
        </Button>
      )}

      {activeProjectId && (
        <Button
          size="sm"
          variant="outline"
          className="h-7 gap-1.5 text-xs"
          onClick={onTranslateAll}
          disabled={isTranslating || isPilotRunning}
        >
          {isTranslating ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <Languages className="h-3.5 w-3.5" />
          )}
          {t("toolbar.translateAll")}
        </Button>
      )}

      {activeProjectId && (
        <Button
          size="sm"
          variant="outline"
          className="h-7 gap-1.5 text-xs"
          onClick={onExportAll}
          disabled={isTranslating || isExporting || isPilotRunning}
        >
          {isExporting ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <Download className="h-3.5 w-3.5" />
          )}
          {t("toolbar.exportAll")}
        </Button>
      )}

      {developerTools && activeProject?.engine === "mv_mz" && (
        <Button
          size="sm"
          variant={isPilotOpen ? "secondary" : "outline"}
          className="h-7 gap-1.5 text-xs"
          onClick={onOpenPilot}
          disabled={isTranslating || isPilotRunning}
        >
          {isPilotRunning ? (
            <Loader2 className="h-3.5 w-3.5 motion-safe:animate-spin" />
          ) : (
            <FlaskConical className="h-3.5 w-3.5" />
          )}
          {t("pilot.toolbar")}
        </Button>
      )}

      {activeProjectId && (
        <>
          <Button
            size="sm"
            variant="outline"
            className="hit-area-40 relative h-7 w-7 p-0"
            title={t("pack.shareButton")}
            onClick={() => setShowPackExport(true)}
            disabled={isTranslating || isPilotRunning}
          >
            <Share2 className="h-3.5 w-3.5" />
          </Button>
          <Button
            size="sm"
            variant="outline"
            className="hit-area-40 relative h-7 w-7 p-0"
            title={t("pack.importButton")}
            onClick={() => void handleImportPack()}
            disabled={isTranslating || isPreviewingPack || isPilotRunning}
          >
            {isPreviewingPack ? (
              <Loader2 className="h-3.5 w-3.5 animate-spin" />
            ) : (
              <PackageOpen className="h-3.5 w-3.5" />
            )}
          </Button>
        </>
      )}

      {/* Progress bar + timer + cooldown */}
      {isTranslating && progress >= 0 && (
        <div className="ml-auto flex items-center gap-2 mr-2">
          <TranslationTimer />
          <CooldownBadge />
          <ConstellationProgress progress={progress} />
        </div>
      )}

      {/* Update badge + About + Settings buttons — pushed to the right */}
      <div className="ml-auto" />
      <UpdateBadge />
      <Button
        size="sm"
        variant="ghost"
        className="hit-area-40 relative h-7 w-7 p-0"
        onClick={onOpenAbout}
        title={t("about.title")}
      >
        <Info className="h-4 w-4" />
      </Button>
      {activeProjectId && activeProject && (
        <PackExportDialog
          open={showPackExport}
          projectId={activeProjectId}
          projectName={activeProject.name}
          langPair={`${activeProject.sourceLang}-${activeProject.targetLang}`}
          onClose={() => setShowPackExport(false)}
        />
      )}
      {activeProjectId && packImport && (
        <PackImportWizard
          projectId={activeProjectId}
          packPath={packImport.packPath}
          preview={packImport.preview}
          langPair={
            activeProject
              ? `${activeProject.sourceLang}-${activeProject.targetLang}`
              : "ja-fr"
          }
          onClose={() => setPackImport(null)}
        />
      )}
    </div>
  );
}
