import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { FileTree } from "@/components/editor/FileTree";
import { SegmentGrid } from "@/components/editor/SegmentGrid";
import { TMPanel } from "@/components/editor/TMPanel";
import { QAPanel } from "@/components/editor/QAPanel";
import { openProject, useProjectStore } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import {
  useLlmStore,
  useIsTranslating,
  useTranslationProgress,
  useTranslationStartTime,
} from "@/stores/llm";
import { Toaster } from "@/components/ui/sonner";
import { Button } from "@/components/ui/button";
import { Clock, Download, FolderOpen, Loader2, Play, X } from "lucide-react";
import { toast } from "sonner";

// ---------------------------------------------------------------------------
// Placeholder highlight helper
// ---------------------------------------------------------------------------

const PH_RE = /\\[VNPCI]\[\d+\]|\\[G\\$.|!><^{}]|\[%\d+\]/g;

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
// LLM config modal
// ---------------------------------------------------------------------------

interface LlmConfigModalProps {
  onClose: () => void;
  onStart: (segmentIds: string[], fileId?: string) => void;
}

function LlmConfigModal({ onClose, onStart }: LlmConfigModalProps) {
  const { t } = useTranslation();
  const { providerConfig, setProviderConfig } = useLlmStore();
  const activeFileId = useEditorStore((s) => s.activeFileId);

  const [models, setModels] = useState<string[]>([]);
  const [modelsLoading, setModelsLoading] = useState(false);
  const [modelsError, setModelsError] = useState<string | null>(null);
  const [urlDraft, setUrlDraft] = useState(providerConfig.url);

  async function fetchModels(url: string) {
    setModelsLoading(true);
    setModelsError(null);
    try {
      const list = await invoke<string[]>("get_ollama_models", { url });
      setModels(list);
      if (list.length > 0) {
        const keep = list.includes(providerConfig.model)
          ? providerConfig.model
          : list[0];
        setProviderConfig({ model: keep });
      }
    } catch {
      setModelsError(t("llmModal.modelError"));
      setModels([]);
    } finally {
      setModelsLoading(false);
    }
  }

  useEffect(() => {
    void fetchModels(providerConfig.url);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  function handleUrlBlur() {
    const trimmed = urlDraft.trim();
    setProviderConfig({ url: trimmed });
    void fetchModels(trimmed);
  }

  function handleStart() {
    onStart([], activeFileId ?? undefined);
    onClose();
  }

  const canStart =
    !!activeFileId && (models.length > 0 || providerConfig.model.trim() !== "");

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-96 rounded-lg border bg-background p-4 shadow-xl">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-semibold">{t("llmModal.title")}</h2>
          <button type="button" onClick={onClose}>
            <X className="h-4 w-4 text-muted-foreground hover:text-foreground" />
          </button>
        </div>

        <div className="space-y-3">
          <div>
            <label className="mb-1 block text-xs text-muted-foreground">
              {t("llmModal.urlLabel")}
            </label>
            <input
              type="text"
              className="w-full rounded border bg-muted/30 px-2 py-1.5 text-xs outline-none focus:border-primary"
              value={urlDraft}
              onChange={(e) => setUrlDraft(e.target.value)}
              onBlur={handleUrlBlur}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs text-muted-foreground">
              {t("llmModal.modelLabel")}
            </label>
            {models.length > 0 ? (
              <Select
                value={providerConfig.model}
                onValueChange={(v) => setProviderConfig({ model: v })}
              >
                <SelectTrigger className="h-8 w-full text-xs">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  {models.map((m) => (
                    <SelectItem key={m} value={m} className="text-xs">
                      {m}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            ) : (
              <>
                <Select disabled>
                  <SelectTrigger className="h-8 w-full text-xs">
                    <SelectValue
                      placeholder={
                        modelsLoading
                          ? t("llmModal.modelLoading")
                          : t("llmModal.modelNone")
                      }
                    />
                  </SelectTrigger>
                  <SelectContent />
                </Select>
                {modelsError && (
                  <input
                    type="text"
                    className="mt-1.5 w-full rounded border bg-muted/30 px-2 py-1.5 text-xs outline-none focus:border-primary"
                    placeholder={t("llmModal.modelManual")}
                    value={providerConfig.model}
                    onChange={(e) =>
                      setProviderConfig({ model: e.target.value })
                    }
                  />
                )}
              </>
            )}
            {modelsError && (
              <p className="mt-1 text-[11px] text-destructive">{modelsError}</p>
            )}
          </div>
        </div>

        <div className="mt-4 flex justify-end gap-2">
          <Button
            size="sm"
            variant="outline"
            className="h-7 text-xs"
            onClick={onClose}
          >
            {t("llmModal.cancel")}
          </Button>
          <Button
            size="sm"
            className="h-7 text-xs"
            onClick={handleStart}
            disabled={!canStart}
          >
            {modelsLoading ? (
              <Loader2 className="mr-1 h-3 w-3 animate-spin" />
            ) : (
              <Play className="mr-1 h-3 w-3" />
            )}
            {t("llmModal.start")}
          </Button>
        </div>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Toolbar
// ---------------------------------------------------------------------------

function Toolbar({ onOpenLlmConfig }: { onOpenLlmConfig: () => void }) {
  const { t, i18n } = useTranslation();
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

  function handleToggleLang() {
    void i18n.changeLanguage(i18n.language === "fr" ? "en" : "fr");
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
          onClick={onOpenLlmConfig}
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

      <Button
        size="sm"
        variant="ghost"
        className="h-7 text-xs"
        onClick={handleToggleLang}
      >
        {i18n.language === "fr" ? "🇬🇧 EN" : "🇫🇷 FR"}
      </Button>

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
        <div className="ml-auto flex items-center gap-2 mr-2">
          <TranslationTimer />
          <div className="h-1.5 w-32 overflow-hidden rounded-full bg-muted">
            <div
              className="h-full rounded-full bg-primary transition-all duration-300"
              style={{ width: `${progress}%` }}
            />
          </div>
        </div>
      )}
    </div>
  );
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

export default function App() {
  const [showLlmConfig, setShowLlmConfig] = useState(false);
  const { startTranslation } = useLlmStore();
  const activeSegmentSourceText = useEditorStore(
    (s) => s.activeSegmentSourceText,
  );
  const activeSegmentTargetText = useEditorStore(
    (s) => s.activeSegmentTargetText,
  );

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <Toolbar onOpenLlmConfig={() => setShowLlmConfig(true)} />

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

        {/* Centre: SegmentGrid */}
        <ResizablePanel defaultSize="55%" minSize="40%" collapsible={false}>
          <div className="flex h-full flex-col overflow-hidden">
            <SegmentGrid highlightPlaceholders />
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
              <ResizablePanel defaultSize={55} minSize={30}>
                <TMPanel />
              </ResizablePanel>
              <ResizableHandle />
              <ResizablePanel defaultSize={45} minSize={25}>
                <QAPanel
                  sourceText={activeSegmentSourceText}
                  targetText={activeSegmentTargetText}
                />
              </ResizablePanel>
            </ResizablePanelGroup>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>

      {showLlmConfig && (
        <LlmConfigModal
          onClose={() => setShowLlmConfig(false)}
          onStart={(ids, fileId) => void startTranslation(ids, fileId)}
        />
      )}

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
