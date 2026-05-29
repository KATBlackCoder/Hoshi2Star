import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { FolderOpen, Loader2, Play, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  useProjectStore,
  openProject,
  loadAllProjects,
  deleteProject,
} from "@/stores/project";
import type { Project } from "@/lib/types";
import { toast } from "sonner";

function engineLabel(engine: string): string {
  switch (engine) {
    case "mv_mz":
      return "MV/MZ";
    case "vx_ace":
      return "VX Ace";
    case "wolf":
      return "Wolf RPG";
    case "bakin":
      return "Bakin";
    default:
      return engine;
  }
}

function relativeDate(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const days = Math.floor(diff / 86_400_000);
  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 30) return `${days}d ago`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months}mo ago`;
  return `${Math.floor(months / 12)}y ago`;
}

export function ProjectList() {
  const { t } = useTranslation();
  const projects = useProjectStore((s) => s.projects);
  const [isOpening, setIsOpening] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState<string | null>(null);
  const [openingNew, setOpeningNew] = useState(false);

  useEffect(() => {
    void loadAllProjects();
  }, []);

  async function handleResume(project: Project) {
    setIsOpening(project.id);
    try {
      await openProject(project.gamePath);
    } catch {
      toast.error(t("projectList.openError"));
    } finally {
      setIsOpening(null);
    }
  }

  async function handleDelete(project: Project, e: React.MouseEvent) {
    e.stopPropagation();
    setIsDeleting(project.id);
    try {
      await deleteProject(project.id);
      toast.success(t("projectList.deleted", { name: project.name }));
    } catch {
      toast.error(t("projectList.deleteError"));
    } finally {
      setIsDeleting(null);
    }
  }

  async function handleOpenNew() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t("toolbar.openGame"),
    });
    if (!selected) return;
    setOpeningNew(true);
    try {
      await openProject(selected as string);
    } catch (err) {
      console.error("open_project failed:", err);
    } finally {
      setOpeningNew(false);
    }
  }

  return (
    <div className="flex h-full flex-col items-center justify-center p-6 gap-4">
      <p className="text-sm font-medium text-foreground">
        {t("projectList.title")}
      </p>

      {projects.length === 0 ? (
        <p className="text-xs text-muted-foreground">
          {t("projectList.empty")}
        </p>
      ) : (
        <div className="w-full max-w-md space-y-1.5">
          {projects.map((project) => (
            <div
              key={project.id}
              className="group flex items-center gap-3 rounded-lg border bg-card px-3 py-2.5 hover:bg-accent/30 transition-colors cursor-pointer"
              onClick={() => void handleResume(project)}
            >
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate">{project.name}</p>
                <p className="text-[11px] text-muted-foreground truncate">
                  {project.gamePath}
                </p>
              </div>

              <span className="shrink-0 rounded bg-muted px-1.5 py-0.5 font-mono text-[10px] text-muted-foreground">
                {engineLabel(project.engine)}
              </span>
              <span className="shrink-0 text-[11px] text-muted-foreground tabular-nums">
                {relativeDate(project.updatedAt)}
              </span>

              <Button
                size="sm"
                variant="outline"
                className="h-7 gap-1 text-xs shrink-0 opacity-0 group-hover:opacity-100 transition-opacity"
                disabled={isOpening === project.id}
                onClick={(e) => {
                  e.stopPropagation();
                  void handleResume(project);
                }}
              >
                {isOpening === project.id ? (
                  <Loader2 className="h-3 w-3 animate-spin" />
                ) : (
                  <Play className="h-3 w-3" />
                )}
                {t("projectList.continue")}
              </Button>

              <button
                type="button"
                className="shrink-0 flex h-6 w-6 items-center justify-center rounded opacity-0 group-hover:opacity-100 hover:bg-destructive/20 hover:text-destructive text-muted-foreground transition-all"
                disabled={isDeleting === project.id}
                title={t("projectList.delete")}
                onClick={(e) => void handleDelete(project, e)}
              >
                {isDeleting === project.id ? (
                  <Loader2 className="h-3 w-3 animate-spin" />
                ) : (
                  <Trash2 className="h-3 w-3" />
                )}
              </button>
            </div>
          ))}
        </div>
      )}

      <Button
        size="sm"
        variant="outline"
        className="h-7 gap-1.5 text-xs mt-2"
        onClick={() => void handleOpenNew()}
        disabled={openingNew}
      >
        {openingNew ? (
          <Loader2 className="h-3.5 w-3.5 animate-spin" />
        ) : (
          <FolderOpen className="h-3.5 w-3.5" />
        )}
        {t("toolbar.openGame")}
      </Button>
    </div>
  );
}
