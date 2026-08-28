import { lazy, Suspense, useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { GlobalSearchResults } from "@/components/editor/GlobalSearchResults";
import { ProjectList } from "@/components/editor/ProjectList";
import { SegmentGrid } from "@/components/editor/SegmentGrid";
import { AppToolbar } from "@/components/AppToolbar";
import { AppDialogs } from "@/components/AppDialogs";
import { UpdateDialog } from "@/components/UpdateDialog";
import { useProjectStore, useActiveLangPair } from "@/stores/project";
import { useEditorStore } from "@/stores/editor";
import { useSearchActive } from "@/stores/search";
import { useSettingsStore } from "@/stores/settings";
import { useUpdaterStore } from "@/stores/updater";
import { useAppHandlers } from "@/hooks/useAppHandlers";
import { Toaster } from "@/components/ui/sonner";
import { AppShell } from "@/components/shell/AppShell";
import { InspectorRail } from "@/components/shell/InspectorRail";
import { useAppMode, useUiStore } from "@/stores/ui";
import { FileTreePanel } from "@/components/shell/FileTreePanel";
import type { PanelImperativeHandle } from "react-resizable-panels";
import { PilotComparisonWorkspace } from "@/components/pilot/PilotComparisonWorkspace";
import { usePilotStore } from "@/stores/pilot";
import { PatchWorkflow } from "@/components/patch/PatchWorkflow";
import { WorkspaceState } from "@/components/common/WorkspaceState";
import { Images, Play } from "lucide-react";

const TerminologyWorkspace = lazy(() =>
  import("@/features/terminology/TerminologyWorkspace").then((module) => ({
    default: module.TerminologyWorkspace,
  })),
);

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

export default function App() {
  const handlers = useAppHandlers();
  const { loadSettings } = useSettingsStore();
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const activeLangPair = useActiveLangPair();
  const activeSegmentSourceText = useEditorStore(
    (s) => s.activeSegmentSourceText,
  );
  const activeSegmentTargetText = useEditorStore(
    (s) => s.activeSegmentTargetText,
  );
  const checkForUpdate = useUpdaterStore((s) => s.checkForUpdate);
  const searchActive = useSearchActive();
  const mode = useAppMode();
  const fileTreeOpen = useUiStore((state) => state.fileTreeOpen);
  const setFileTreeOpen = useUiStore((state) => state.setFileTreeOpen);
  const toggleFileTree = useUiStore((state) => state.toggleFileTree);
  const fileTreePanelRef = useRef<PanelImperativeHandle>(null);
  const pilotOpen = usePilotStore((state) => state.isOpen);
  const openPilot = usePilotStore((state) => state.openWorkspace);

  useEffect(() => {
    void loadSettings();
  }, [loadSettings]);

  // Non-blocking update check once at startup (silent if offline / unsupported).
  useEffect(() => {
    void checkForUpdate();
  }, [checkForUpdate]);

  useEffect(() => {
    if (fileTreeOpen) fileTreePanelRef.current?.expand();
    else fileTreePanelRef.current?.collapse();
  }, [fileTreeOpen]);

  return (
    <>
      <AppShell onOpenSettings={() => handlers.setShowSettings(true)}>
        {mode === "library" ? (
          <ProjectList />
        ) : mode === "terminology" ? (
          <Suspense
            fallback={
              <div className="grid h-full place-items-center text-sm text-muted-foreground">
                Chargement de la terminologie…
              </div>
            }
          >
            <TerminologyWorkspace />
          </Suspense>
        ) : mode === "patch" ? (
          <div className="flex h-full flex-col overflow-hidden">
            <PatchWorkflow
              onExport={() => void handlers.handleExportAll()}
              exporting={handlers.isExporting}
            />
            <AppToolbar
              onOpenAbout={() => handlers.setShowAbout(true)}
              onTranslate={handlers.handleTranslate}
              onTranslateAll={() => void handlers.handleTranslateAll()}
              onExportAll={() => void handlers.handleExportAll()}
              onOpenPilot={openPilot}
              isExporting={handlers.isExporting}
            />

            {pilotOpen ? (
              <PilotComparisonWorkspace />
            ) : (
              <div className="relative flex min-h-0 flex-1 overflow-hidden">
                <ResizablePanelGroup
                  orientation="horizontal"
                  className="min-w-0 flex-1 overflow-hidden"
                >
                  {/* Left: game files */}
                  <ResizablePanel
                    id="file-tree"
                    defaultSize="24%"
                    minSize="15%"
                    maxSize="35%"
                    collapsedSize="48px"
                    collapsible
                    panelRef={fileTreePanelRef}
                    onResize={(size) => {
                      const expanded = size.inPixels > 56;
                      if (expanded !== fileTreeOpen) setFileTreeOpen(expanded);
                    }}
                  >
                    <FileTreePanel
                      expanded={fileTreeOpen}
                      activeProjectId={activeProjectId}
                      onToggle={toggleFileTree}
                    />
                  </ResizablePanel>

                  <ResizableHandle withHandle />

                  {/* Centre: project picker or translation grid */}
                  <ResizablePanel
                    defaultSize="76%"
                    minSize="45%"
                    collapsible={false}
                  >
                    <div className="flex h-full flex-col overflow-hidden">
                      {activeProjectId ? (
                        searchActive ? (
                          <GlobalSearchResults />
                        ) : (
                          <SegmentGrid highlightPlaceholders />
                        )
                      ) : (
                        <ProjectList />
                      )}
                    </div>
                  </ResizablePanel>
                </ResizablePanelGroup>

                <InspectorRail
                  projectId={activeProjectId}
                  langPair={activeLangPair}
                  sourceText={activeSegmentSourceText}
                  targetText={activeSegmentTargetText}
                />
              </div>
            )}
          </div>
        ) : (
          <ModePlaceholder mode={mode} />
        )}
      </AppShell>
      <AppDialogs handlers={handlers} />
      <UpdateDialog />
      <Toaster />
    </>
  );
}

function ModePlaceholder({ mode }: { mode: "player" | "images" }) {
  const { t } = useTranslation();
  return (
    <div className="observatory-grid grid h-full place-items-center p-4 sm:p-8">
      <WorkspaceState
        className="w-full max-w-lg"
        icon={mode === "player" ? <Play className="size-4" /> : <Images className="size-4" />}
        eyebrow="Phase 2"
        title={t("modes.comingSoonTitle", { mode: t(`modes.${mode}`) })}
        description={t("modes.comingSoonDescription")}
      />
    </div>
  );
}
