import { useEffect } from "react";
import { useTranslation } from "react-i18next";
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
import { AppToolbar } from "@/components/AppToolbar";
import { AppDialogs } from "@/components/AppDialogs";
import { useProjectStore, useIsExtractingGlossary } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { useSettingsStore } from "@/stores/settings";
import { useAppHandlers } from "@/hooks/useAppHandlers";
import { Toaster } from "@/components/ui/sonner";
import { BookOpen, Loader2 } from "lucide-react";

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

export default function App() {
  const handlers = useAppHandlers();
  const { loadSettings } = useSettingsStore();
  const { t } = useTranslation();
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const activeSegmentSourceText = useEditorStore(
    (s) => s.activeSegmentSourceText,
  );
  const activeSegmentTargetText = useEditorStore(
    (s) => s.activeSegmentTargetText,
  );
  const isExtractingGlossary = useIsExtractingGlossary();

  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <AppToolbar
        onOpenSettings={() => handlers.setShowSettings(true)}
        onOpenAbout={() => handlers.setShowAbout(true)}
        onTranslate={handlers.handleTranslate}
        onTranslateAll={() => void handlers.handleTranslateAll()}
        onExportAll={() => void handlers.handleExportAll()}
      />

      {isExtractingGlossary && (
        <div className="flex h-7 shrink-0 items-center gap-2 border-b bg-muted/50 px-3 text-xs text-muted-foreground">
          <Loader2 className="h-3 w-3 animate-spin shrink-0" />
          <BookOpen className="h-3 w-3 shrink-0" />
          <span>{t("glossaryPrompt.extracting")}</span>
        </div>
      )}

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

        {/* Right: TM + QA + Glossary side panels */}
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

      <AppDialogs handlers={handlers} />
      <Toaster />
    </div>
  );
}

// ---------------------------------------------------------------------------
// FileTree panel header
// ---------------------------------------------------------------------------

function FileTreeHeader() {
  const { t } = useTranslation();
  return (
    <div className="shrink-0 border-b px-3 py-2 text-xs font-medium text-muted-foreground select-none">
      {t("fileTree.title")}
    </div>
  );
}
