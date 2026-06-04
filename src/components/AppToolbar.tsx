import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "@/components/ui/button";
import {
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
// AppToolbar
// ---------------------------------------------------------------------------

interface AppToolbarProps {
  onOpenSettings: () => void;
  onOpenAbout: () => void;
  onTranslate: () => void;
  onTranslateAll: () => void;
  onExportAll: () => void;
}

export function AppToolbar({
  onOpenSettings,
  onOpenAbout,
  onTranslate,
  onTranslateAll,
  onExportAll,
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
      console.error("open_project failed:", err);
    } finally {
      setIsOpening(false);
    }
  }

  return (
    <div className="flex h-10 shrink-0 items-center gap-3 border-b px-3">
      <span className="text-sm font-semibold tracking-tight select-none">
        Hoshi2Star ★
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
          variant="outline"
          className="h-7 gap-1.5 text-xs"
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
          disabled={isTranslating}
        >
          <Download className="h-3.5 w-3.5" />
          {t("toolbar.exportAll")}
        </Button>
      )}

      {activeProjectId && activeProject && (
        <span className="ml-1 truncate text-xs text-muted-foreground">
          {activeProject.name}
          <span className="ml-1.5 rounded bg-muted px-1 py-0.5 font-mono text-[10px]">
            {activeProject.engine}
          </span>
        </span>
      )}

      {/* Progress bar + timer + cooldown */}
      {isTranslating && progress >= 0 && (
        <div className="flex items-center gap-2 mr-2">
          <TranslationTimer />
          <CooldownBadge />
          <div className="h-1.5 w-32 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-primary transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
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
