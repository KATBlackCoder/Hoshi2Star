import { useState, useEffect, type ComponentProps } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { cn, errorMessage } from "@/lib/utils";
import {
  Bug,
  Clock,
  Download,
  FolderOpen,
  FlaskConical,
  Info,
  Languages,
  ListChecks,
  Loader2,
  PackageOpen,
  Play,
  Share2,
  Snowflake,
  Square,
  WrapText,
  type LucideIcon,
} from "lucide-react";
import { getProjectLangPair, useProjectStore } from "@/stores/project";
import { openGameFolder } from "@/stores/wolfUnpack";
import { openCoverageDialog } from "@/stores/coverage";
import { openTextFitDialog } from "@/stores/textFit";
import {
  useIsTranslating,
  useTranslationProgress,
  useTranslationStartTime,
  useIsCooling,
  useCooldownRemaining,
  useIsCancelling,
  useLlmStore,
} from "@/stores/llm";
import { UpdateBadge } from "@/components/UpdateBadge";
import { PackExportDialog } from "@/components/PackExportDialog";
import { H2S_FILTER } from "@/lib/constants";
import { PackImportWizard } from "@/components/PackImportWizard";
import type { ImportPreview } from "@/lib/types";
import { usePilotStore } from "@/stores/pilot";
import { useSettingsStore } from "@/stores/settings";
import { packApi } from "@/lib/api/pack";
import {
  hasDebugExport,
  hasPilot,
  hasTextCoverage,
  hasTextFit,
} from "@/lib/engine";

/**
 * A toolbar button whose icon becomes a spinner while `busy`. Other props go
 * to the `Button`.
 */
function ToolbarButton({
  icon: Icon,
  busy,
  spinClassName = "animate-spin",
  children,
  ...props
}: ComponentProps<typeof Button> & {
  icon: LucideIcon;
  busy: boolean;
  spinClassName?: string;
}) {
  return (
    <Button {...props}>
      {busy ? (
        <Loader2 className={`h-3.5 w-3.5 ${spinClassName}`} />
      ) : (
        <Icon className="h-3.5 w-3.5" />
      )}
      {children}
    </Button>
  );
}

/** `seconds` as `mm:ss`. */
function formatMinutesSeconds(seconds: number): string {
  const mm = String(Math.floor(seconds / 60)).padStart(2, "0");
  const ss = String(seconds % 60).padStart(2, "0");
  return `${mm}:${ss}`;
}

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


  return (
    <div
      className={`flex items-center gap-1 font-mono text-xs tabular-nums ${
        isTranslating ? "text-muted-foreground" : "text-green-400"
      }`}
    >
      <Clock className="h-3 w-3 shrink-0" />
      {formatMinutesSeconds(elapsed)}
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


  return (
    <div className="flex items-center gap-1 font-mono text-xs tabular-nums text-blue-400">
      <Snowflake className="h-3 w-3 shrink-0 animate-pulse" />
      {t("toolbar.translateAllCooling", { remaining: formatMinutesSeconds(remaining) })}
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
  /** Developer tool: export the translations ready so far, for a test in game. */
  onDebugExport: () => void;
  isExporting: boolean;
}

export function AppToolbar({
  onOpenAbout,
  onTranslate,
  onTranslateAll,
  onExportAll,
  onOpenPilot,
  onDebugExport,
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
  const isCancelling = useIsCancelling();
  const cancelTranslation = useLlmStore((s) => s.cancelTranslation);
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
      // A packed Wolf RPG game opens the decryption dialog instead of a toast.
      await openGameFolder(selected as string);
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
      filters: [H2S_FILTER],
    });
    if (!selected) return;
    setIsPreviewingPack(true);
    try {
      const preview = await packApi.previewImport({
        projectId: activeProjectId,
        packPath: selected as string,
        langPair: getProjectLangPair(activeProjectId),
      });
      setPackImport({ packPath: selected as string, preview });
    } catch (err) {
      toast.error(t("pack.importError", { error: errorMessage(err) }));
    } finally {
      setIsPreviewingPack(false);
    }
  }

  return (
    <div className="flex min-h-11 min-w-0 shrink-0 items-center gap-2 overflow-x-auto border-b px-2 [scrollbar-width:none] sm:px-3">
      <ToolbarButton
        icon={FolderOpen}
        busy={isOpening}
        size="sm"
        variant="outline"
        className="h-7 gap-1.5 text-xs"
        onClick={() => void handleOpenGame()}
        disabled={isOpening}
      >
        {t("toolbar.openGame")}
      </ToolbarButton>

      {activeProjectId && (
        <ToolbarButton
          icon={Play}
          busy={isTranslating}
          size="sm"
          className="h-7 gap-1.5 text-xs shadow-[0_0_12px_oklch(0.65_0.18_285/35%)]"
          onClick={onTranslate}
          disabled={isTranslating || isPilotRunning}
        >
          {isTranslating
            ? `${t("toolbar.translating")} ${progress > 0 ? `${progress}%` : ""}`
            : t("toolbar.translate")}
        </ToolbarButton>
      )}

      {activeProjectId && (
        <ToolbarButton
          icon={Languages}
          busy={isTranslating}
          size="sm"
          variant="outline"
          className="h-7 gap-1.5 text-xs"
          onClick={onTranslateAll}
          disabled={isTranslating || isPilotRunning}
        >
          {t("toolbar.translateAll")}
        </ToolbarButton>
      )}

      {activeProjectId && (
        <ToolbarButton
          icon={Download}
          busy={isExporting}
          id="toolbar-export-all"
          size="sm"
          variant="outline"
          className="h-7 gap-1.5 text-xs"
          onClick={onExportAll}
          disabled={isTranslating || isExporting || isPilotRunning}
        >
          {t("toolbar.exportAll")}
        </ToolbarButton>
      )}

      {activeProjectId && hasTextCoverage(activeProject?.engine) && (
        <Button
          size="sm"
          variant="outline"
          className="h-7 gap-1.5 text-xs"
          title={t("coverage.toolbarHint")}
          onClick={() => void openCoverageDialog(activeProjectId)}
          disabled={isTranslating || isPilotRunning}
        >
          <ListChecks className="h-3.5 w-3.5" />
          {t("coverage.toolbar")}
        </Button>
      )}

      {activeProjectId && hasTextFit(activeProject?.engine) && (
        <Button
          size="sm"
          variant="outline"
          className="h-7 gap-1.5 text-xs"
          title={t("textFit.toolbarHint")}
          onClick={() => void openTextFitDialog(activeProjectId)}
          disabled={isTranslating || isPilotRunning}
        >
          <WrapText className="h-3.5 w-3.5" />
          {t("textFit.toolbar")}
        </Button>
      )}

      {developerTools && hasPilot(activeProject?.engine) && (
        <ToolbarButton
          icon={FlaskConical}
          busy={isPilotRunning}
          spinClassName="motion-safe:animate-spin"
          size="sm"
          variant={isPilotOpen ? "secondary" : "outline"}
          className="h-7 gap-1.5 text-xs"
          onClick={onOpenPilot}
          disabled={isTranslating || isPilotRunning}
        >
          {t("pilot.toolbar")}
        </ToolbarButton>
      )}

      {developerTools &&
        activeProjectId &&
        hasDebugExport(activeProject?.engine) && (
          <Button
            size="sm"
            variant="outline"
            className="h-7 gap-1.5 text-xs"
            title={t("debugExport.toolbarHint")}
            onClick={onDebugExport}
            disabled={isTranslating || isPilotRunning || isExporting}
          >
            <Bug className="h-3.5 w-3.5" />
            {t("debugExport.toolbar")}
          </Button>
        )}

      {activeProjectId && (
        <>
          <Button
            size="sm"
            variant="outline"
            className="hit-area-40 relative h-7 w-7 p-0"
            title={t("pack.shareButton")}
            aria-label={t("pack.shareButton")}
            onClick={() => setShowPackExport(true)}
            disabled={isTranslating || isPilotRunning}
          >
            <Share2 className="h-3.5 w-3.5" />
          </Button>
          <ToolbarButton
            icon={PackageOpen}
            busy={isPreviewingPack}
            size="sm"
            variant="outline"
            className="hit-area-40 relative h-7 w-7 p-0"
            title={t("pack.importButton")}
            aria-label={t("pack.importButton")}
            onClick={() => void handleImportPack()}
            disabled={isTranslating || isPreviewingPack || isPilotRunning}
          >
          </ToolbarButton>
        </>
      )}

      {/* Progress bar + timer + cooldown */}
      {isTranslating && progress >= 0 && (
        <div className="ml-auto flex items-center gap-2 mr-2">
          <ToolbarButton
            icon={Square}
            busy={isCancelling}
            id="toolbar-stop-translation"
            size="sm"
            variant="outline"
            className="h-7 gap-1.5 text-xs"
            onClick={() => void cancelTranslation()}
            disabled={isCancelling}
            title={t("llmRun.stopHint")}
          >
            {isCancelling ? t("llmRun.stopping") : t("llmRun.stop")}
          </ToolbarButton>
          <TranslationTimer />
          <CooldownBadge />
          <ConstellationProgress progress={progress} />
        </div>
      )}

      {/* Update badge + About + Settings buttons — pushed to the right */}
      <div className="ml-auto" />
      <UpdateBadge />
      <Button
        id="about-trigger"
        size="sm"
        variant="ghost"
        className="hit-area-40 relative h-7 w-7 p-0"
        onClick={onOpenAbout}
        title={t("about.title")}
        aria-label={t("about.title")}
      >
        <Info className="h-4 w-4" />
      </Button>
      {activeProjectId && activeProject && (
        <PackExportDialog
          open={showPackExport}
          projectId={activeProjectId}
          projectName={activeProject.name}
          langPair={getProjectLangPair(activeProjectId)}
          onClose={() => setShowPackExport(false)}
        />
      )}
      {activeProjectId && packImport && (
        <PackImportWizard
          projectId={activeProjectId}
          packPath={packImport.packPath}
          preview={packImport.preview}
          onClose={() => setPackImport(null)}
        />
      )}
    </div>
  );
}
