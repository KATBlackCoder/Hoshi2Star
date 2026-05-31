import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { FileTree } from "@/components/editor/FileTree";
import { ProjectList } from "@/components/editor/ProjectList";
import { SegmentGrid } from "@/components/editor/SegmentGrid";
import { TMPanel } from "@/components/editor/TMPanel";
import { QAPanel } from "@/components/editor/QAPanel";
import { GlossaryPanel } from "@/components/editor/GlossaryPanel";
import { SettingsModal } from "@/components/settings/SettingsModal";
import { openProject, useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import {
  useLlmStore,
  useIsTranslating,
  useTranslationProgress,
  useTranslationStartTime,
} from "@/stores/llm";
import { useSettingsStore } from "@/stores/settings";
import { Toaster } from "@/components/ui/sonner";
import { Button } from "@/components/ui/button";
import {
  Clock,
  Download,
  FolderOpen,
  Loader2,
  Play,
  Settings,
} from "lucide-react";
import { toast } from "sonner";

// ---------------------------------------------------------------------------
// Placeholder highlight helper
// ---------------------------------------------------------------------------

const PH_RE = /\\[+\-]\w+\[\d+\]|\\[VNPCI]\[\d+\]|\\[G\\$.|!><^{}]|\[%\d+\]/g;

export function HighlightedSource({ text }: { text: string }) {
  const parts: React.ReactNode[] = [];
  let last = 0;
  let match: RegExpExecArray | null;
  PH_RE.lastIndex = 0;
  while ((match = PH_RE.exec(text)) !== null) {
    if (match.index > last) {
      parts.push(text.slice(last, match.index));
    }
    parts.push(
      <mark
        key={match.index}
        className="rounded bg-blue-500/20 px-0.5 text-blue-400 font-mono"
      >
        {match[0]}
      </mark>,
    );
    last = match.index + match[0].length;
  }
  if (last < text.length) {
    parts.push(text.slice(last));
  }
  return <>{parts}</>;
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
// Toolbar
// ---------------------------------------------------------------------------

function Toolbar({
  onOpenSettings,
  onTranslate,
}: {
  onOpenSettings: () => void;
  onTranslate: () => void;
}) {
  const { t } = useTranslation();
  const [isOpening, setIsOpening] = useState(false);
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const activeProject = useProjectStore((s) =>
    s.projects.find((p) => p.id === s.activeProjectId),
  );
  const isTranslating = useIsTranslating();
  const progress = useTranslationProgress();

  async function handleExport() {
    if (!activeProjectId) return;
    try {
      await invoke("export_project", { projectId: activeProjectId });
      toast.success(t("toasts.exportSuccess"));
    } catch (err) {
      toast.error(t("toasts.exportError", { error: String(err) }));
    }
  }

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
          disabled={isTranslating}
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
          onClick={() => void handleExport()}
          disabled={isTranslating}
        >
          <Download className="h-3.5 w-3.5" />
          {t("toolbar.export")}
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

      {/* Progress bar + timer */}
      {isTranslating && progress >= 0 && (
        <div className="flex items-center gap-2 mr-2">
          <TranslationTimer />
          <div className="h-1.5 w-32 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-primary transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      )}

      {/* Settings button — pushed to the right */}
      <Button
        size="sm"
        variant="ghost"
        className="h-7 w-7 p-0 ml-auto"
        onClick={onOpenSettings}
        title={t("settings.title")}
      >
        <Settings className="h-4 w-4" />
      </Button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

export default function App() {
  const [showSettings, setShowSettings] = useState(false);
  const { startTranslation, providerConfig } = useLlmStore();
  const { loadSettings } = useSettingsStore();
  const { t } = useTranslation();
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const activeFileId = useEditorStore((s) => s.activeFileId);
  const activeSegmentSourceText = useEditorStore(
    (s) => s.activeSegmentSourceText,
  );
  const activeSegmentTargetText = useEditorStore(
    (s) => s.activeSegmentTargetText,
  );

  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  function handleTranslate() {
    if (!providerConfig.model.trim()) {
      toast.warning(t("segmentGrid.noModelConfigured"));
      setShowSettings(true);
      return;
    }
    void startTranslation([], activeFileId ?? undefined);
  }

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <Toolbar
        onOpenSettings={() => setShowSettings(true)}
        onTranslate={handleTranslate}
      />

      <ResizablePanelGroup
        orientation="horizontal"
        className="flex-1 overflow-hidden"
      >
        {/* Left: FileTree */}
        <ResizablePanel
          defaultSize="20%"
          minSize="15%"
          maxSize="35%"
          collapsible={false}
        >
          <div className="flex h-full flex-col overflow-hidden border-r">
            <FileTreeHeader />
            <div className="flex-1 overflow-hidden">
              <FileTree />
            </div>
          </div>
        </ResizablePanel>

        <ResizableHandle withHandle={true} />

        {/* Centre: ProjectList (no active project) or SegmentGrid */}
        <ResizablePanel defaultSize="55%" minSize="40%" collapsible={false}>
          <div className="flex h-full flex-col overflow-hidden">
            {activeProjectId ? (
              <SegmentGrid highlightPlaceholders />
            ) : (
              <ProjectList />
            )}
          </div>
        </ResizablePanel>

        <ResizableHandle withHandle={true} />

        {/* Right: TM + QA side panels */}
        <ResizablePanel
          defaultSize="25%"
          minSize="20%"
          maxSize="40%"
          collapsible={false}
        >
          <div className="flex h-full flex-col overflow-hidden border-l">
            <ResizablePanelGroup orientation="vertical">
              <ResizablePanel defaultSize={40} minSize={25}>
                <TMPanel />
              </ResizablePanel>
              <ResizableHandle />
              <ResizablePanel defaultSize={30} minSize={20}>
                <QAPanel
                  sourceText={activeSegmentSourceText}
                  targetText={activeSegmentTargetText}
                />
              </ResizablePanel>
              <ResizableHandle />
              <ResizablePanel defaultSize={30} minSize={20}>
                <GlossaryPanel projectId={activeProjectId} langPair="ja-en" />
              </ResizablePanel>
            </ResizablePanelGroup>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>

      <SettingsModal
        open={showSettings}
        onClose={() => setShowSettings(false)}
      />

      <Toaster />
    </div>
  );
}

// ---------------------------------------------------------------------------
// FileTree panel header (extracted so it can use useTranslation)
// ---------------------------------------------------------------------------

function FileTreeHeader() {
  const { t } = useTranslation();
  return (
    <div className="shrink-0 border-b px-3 py-2 text-xs font-medium text-muted-foreground select-none">
      {t("fileTree.title")}
    </div>
  );
}
