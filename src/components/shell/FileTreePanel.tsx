import { PanelLeftClose, PanelLeftOpen } from "lucide-react";
import { useTranslation } from "react-i18next";
import { FileTree } from "@/components/editor/FileTree";
import { ProjectSearchBar } from "@/components/editor/ProjectSearchBar";
import { Button } from "@/components/ui/button";

interface FileTreePanelProps {
  expanded: boolean;
  activeProjectId: string | null;
  onToggle: () => void;
}

export function FileTreePanel({
  expanded,
  activeProjectId,
  onToggle,
}: FileTreePanelProps) {
  const { t } = useTranslation();

  if (!expanded) {
    return (
      <aside
        aria-label={t("fileTree.title")}
        className="flex h-full items-start justify-center border-r bg-card/35 py-2"
      >
        <Button
          type="button"
          variant="ghost"
          size="icon"
          className="h-10 w-10 text-muted-foreground"
          aria-label={t("fileTree.expand")}
          onClick={onToggle}
        >
          <PanelLeftOpen className="h-4 w-4" />
        </Button>
      </aside>
    );
  }

  return (
    <aside
      aria-label={t("fileTree.title")}
      className="flex h-full flex-col overflow-hidden border-r"
    >
      <div className="flex min-h-10 shrink-0 items-center justify-between border-b pl-3 pr-1">
        <span className="text-[10px] font-semibold uppercase tracking-[0.12em] text-muted-foreground/80 select-none">
          {t("fileTree.title")}
        </span>
        <Button
          type="button"
          variant="ghost"
          size="icon-sm"
          className="text-muted-foreground"
          aria-label={t("fileTree.collapse")}
          onClick={onToggle}
        >
          <PanelLeftClose className="h-3.5 w-3.5" />
        </Button>
      </div>
      {activeProjectId && <ProjectSearchBar key={activeProjectId} />}
      <div className="min-h-0 flex-1 overflow-hidden">
        <FileTree />
      </div>
    </aside>
  );
}
