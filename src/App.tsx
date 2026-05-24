import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import { FileTree } from "@/components/editor/FileTree";
import { SegmentGrid } from "@/components/editor/SegmentGrid";
import { openProject, useProjectStore } from "@/stores/project";
import { Button } from "@/components/ui/button";
import { FolderOpen, Loader2 } from "lucide-react";

function Toolbar() {
  const [isOpening, setIsOpening] = useState(false);
  const activeProjectId = useProjectStore((s) => s.activeProjectId);
  const activeProject = useProjectStore((s) =>
    s.projects.find((p) => p.id === s.activeProjectId),
  );

  async function handleOpenGame() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: "Ouvrir un jeu",
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
        Ouvrir un jeu
      </Button>

      {activeProjectId && activeProject && (
        <span className="ml-1 truncate text-xs text-muted-foreground">
          {activeProject.name}
          <span className="ml-1.5 rounded bg-muted px-1 py-0.5 font-mono text-[10px]">
            {activeProject.engine}
          </span>
        </span>
      )}
    </div>
  );
}

export default function App() {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <Toolbar />

      <ResizablePanelGroup
        orientation="horizontal"
        className="flex-1 overflow-hidden"
      >
        {/* Left: FileTree */}
        <ResizablePanel defaultSize={18} minSize={12} maxSize={30}>
          <div className="flex h-full flex-col overflow-hidden border-r">
            <div className="shrink-0 border-b px-3 py-2 text-xs font-medium text-muted-foreground select-none">
              Fichiers
            </div>
            <div className="flex-1 overflow-hidden">
              <FileTree />
            </div>
          </div>
        </ResizablePanel>

        <ResizableHandle />

        {/* Centre: SegmentGrid */}
        <ResizablePanel defaultSize={60} minSize={40}>
          <div className="flex h-full flex-col overflow-hidden">
            <SegmentGrid />
          </div>
        </ResizablePanel>

        <ResizableHandle />

        {/* Right: SidePanel (placeholder — F2) */}
        <ResizablePanel defaultSize={22} minSize={16} maxSize={35}>
          <div className="flex h-full flex-col overflow-hidden border-l">
            <div className="shrink-0 border-b px-3 py-2 text-xs font-medium text-muted-foreground select-none">
              TM / Glossaire
            </div>
            <div className="flex flex-1 items-center justify-center p-4">
              <p className="text-center text-xs text-muted-foreground leading-relaxed">
                Mémoire de traduction
                <br />
                et glossaire — F2
              </p>
            </div>
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}
