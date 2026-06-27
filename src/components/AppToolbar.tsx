import { useState, useEffect } from "react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { invoke } from "@tauri-apps/api/core";
import {
  Bug,
  Clock,
  Download,
  FolderOpen,
  Info,
  Languages,
  Loader2,
  Play,
  Settings,
  Snowflake,
} from "lucide-react";
import {
  openProject,
  useProjectStore,
  useIsExtractingGlossary,
  useActiveProjectStats,
} from "@/stores/project";
import {
  useIsTranslating,
  useTranslationProgress,
  useTranslationStartTime,
  useIsCooling,
  useCooldownRemaining,
} from "@/stores/llm";

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
        className="absolute left-0 top-1/2 h-0.5 -translate-y-1/2 rounded-full bg-gradient-to-r from-primary to-star shadow-[0_0_8px_var(--star)] transition-all duration-300"
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
        className="absolute top-1/2 -translate-x-1/2 -translate-y-1/2 animate-pulse text-[13px] text-star [text-shadow:0_0_10px_var(--star)] transition-all duration-300"
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
  onOpenSettings: () => void;
  onOpenAbout: () => void;
  onTranslate: () => void;
  onTranslateAll: () => void;
  onExportAll: () => void;
  isExporting: boolean;
}

export function AppToolbar({
  onOpenSettings,
  onOpenAbout,
  onTranslate,
  onTranslateAll,
  onExportAll,
  isExporting,
}: AppToolbarProps) {
  const { t } = useTranslation();
  const [isOpening, setIsOpening] = useState(false);
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const activeProject = useProjectStore((s) =>
    s.projects.find((p) => p.id === s.activeProjectId),
  );
  const isTranslating = useIsTranslating();
  const isExtractingGlossary = useIsExtractingGlossary();
  const progress = useTranslationProgress();
  const activeProjectStats = useActiveProjectStats();

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
      const msg = String(err);
      const key = msg.includes("could not identify game engine")
        ? "projectList.engineNotFound"
        : "projectList.openError";
      toast.error(t(key));
    } finally {
      setIsOpening(false);
    }
  }

  return (
    <div className="flex h-10 shrink-0 items-center gap-3 border-b px-3">
      <span className="flex items-baseline gap-1.5 text-sm font-semibold tracking-tight select-none">
        <span className="text-star drop-shadow-[0_0_6px_var(--star)]">★</span>
        Hoshi2Star
        <span className="text-[9px] font-normal tracking-widest text-muted-foreground/60">
          星 → ★
        </span>
      </span>

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
          disabled={isTranslating || isExtractingGlossary}
        >
          {isExtractingGlossary || isTranslating ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <Play className="h-3.5 w-3.5" />
          )}
          {isExtractingGlossary
            ? t("glossaryPrompt.translationBlocked")
            : isTranslating
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
          disabled={isTranslating || isExtractingGlossary}
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
          disabled={isTranslating || isExporting}
        >
          {isExporting ? (
            <Loader2 className="h-3.5 w-3.5 animate-spin" />
          ) : (
            <Download className="h-3.5 w-3.5" />
          )}
          {t("toolbar.exportAll")}
        </Button>
      )}

      {activeProjectId && activeProject && (
        <span className="ml-1 flex min-w-0 items-center gap-1.5 rounded-full border bg-card/60 px-2.5 py-0.5 text-xs text-muted-foreground">
          <span className="truncate">{activeProject.name}</span>
          <span className="shrink-0 rounded-full border border-primary/30 bg-primary/10 px-1.5 font-mono text-[9px] uppercase tracking-wider text-primary">
            {activeProject.engine}
          </span>
          {activeProjectStats && activeProjectStats.totalSegments > 0 && (
            <span className="shrink-0 font-mono text-[10px] tabular-nums text-star/80">
              {Math.round(
                (activeProjectStats.translatedCount /
                  activeProjectStats.totalSegments) *
                  100,
              )}
              %
            </span>
          )}
        </span>
      )}

      {/* Progress bar + timer + cooldown */}
      {isTranslating && progress >= 0 && (
        <div className="ml-auto flex items-center gap-2 mr-2">
          <TranslationTimer />
          <CooldownBadge />
          <ConstellationProgress progress={progress} />
        </div>
      )}

      {/* About + Settings buttons — pushed to the right */}
      <Button
        size="sm"
        variant="ghost"
        className="h-7 w-7 p-0 ml-auto"
        onClick={onOpenAbout}
        title={t("about.title")}
      >
        <Info className="h-4 w-4" />
      </Button>
      {activeProject && (
        <Button
          size="sm"
          variant="ghost"
          className="h-7 w-7 p-0 text-muted-foreground hover:text-amber-400"
          title="Debug — dump extracted segments to JSON"
          onClick={() => {
            void invoke<string>("debug_dump_segments", {
              gamePath: activeProject.gamePath,
            })
              .then((path) => {
                console.info("[h2s] debug dump written →", path);
                alert(`Debug JSON écrit :\n${path}`);
              })
              .catch((err) => {
                console.error("[h2s] debug dump failed:", err);
                alert(`Erreur : ${err}`);
              });
          }}
        >
          <Bug className="h-4 w-4" />
        </Button>
      )}
      <Button
        size="sm"
        variant="ghost"
        className="h-7 w-7 p-0"
        onClick={onOpenSettings}
        title={t("settings.title")}
      >
        <Settings className="h-4 w-4" />
      </Button>
    </div>
  );
}
