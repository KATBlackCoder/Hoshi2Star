import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import { FolderOpen, Loader2, Play, Trash2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  useProjectStore,
  openProject,
  loadAllProjects,
  deleteProject,
} from "@/stores/project";
import type { Project, ProjectStats } from "@/lib/types";
import { engineLabel, relativeDate } from "@/lib/format";
import { openProjectErrorKey } from "@/lib/projectError";
import { STATUS_SUMMARY } from "@/lib/statusSummary";
import { toast } from "sonner";

export function ProjectList() {
  const { t } = useTranslation();
  const projects = useProjectStore((s) => s.projects);
  const [isOpening, setIsOpening] = useState<string | null>(null);
  const [isDeleting, setIsDeleting] = useState<string | null>(null);
  const [pendingDelete, setPendingDelete] = useState<Project | null>(null);
  const [openingNew, setOpeningNew] = useState(false);
  const [projectStats, setProjectStats] = useState<
    Record<string, ProjectStats>
  >({});

  useEffect(() => {
    void loadAllProjects();
  }, []);

  useEffect(() => {
    if (projects.length === 0) return;
    // allSettled: one failing get_project_stats must not reject the whole
    // batch and wipe the stats bars of every other card.
    void Promise.allSettled(
      projects.map(async (p) => {
        const stats = await invoke<ProjectStats>("get_project_stats", {
          projectId: p.id,
        });
        setProjectStats((prev) => ({ ...prev, [p.id]: stats }));
      }),
    );
  }, [projects]);

  async function handleResume(project: Project) {
    setIsOpening(project.id);
    try {
      await openProject(project.gamePath);
    } catch (err) {
      toast.error(t(openProjectErrorKey(err)));
    } finally {
      setIsOpening(null);
    }
  }

  function requestDelete(project: Project, e: React.MouseEvent) {
    e.stopPropagation();
    setPendingDelete(project);
  }

  async function confirmDelete() {
    if (!pendingDelete) return;
    const project = pendingDelete;
    setPendingDelete(null);
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
      toast.error(t(openProjectErrorKey(err)));
    } finally {
      setOpeningNew(false);
    }
  }

  return (
    <>
      <AlertDialog
        open={!!pendingDelete}
        onOpenChange={(open) => {
          if (!open) setPendingDelete(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("projectList.confirmDeleteTitle", {
                name: pendingDelete?.name ?? "",
              })}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t("projectList.confirmDeleteDesc")}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>
              {t("projectList.confirmDeleteCancel")}
            </AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={() => void confirmDelete()}
            >
              {t("projectList.confirmDeleteConfirm")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
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
            {projects.map((project) => {
              const stats = projectStats[project.id];
              return (
                <div
                  key={project.id}
                  className="group flex flex-col gap-1.5 rounded-lg border bg-card px-3 py-2.5 hover:bg-accent/30 transition-[background-color,transform] cursor-pointer active:scale-[0.96]"
                  onClick={() => void handleResume(project)}
                >
                  <div className="flex items-center gap-3">
                    <div className="flex-1 min-w-0">
                      <p className="text-sm font-medium truncate">
                        {project.name}
                      </p>
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
                      className="hit-area-40 relative shrink-0 flex h-6 w-6 items-center justify-center rounded opacity-0 group-hover:opacity-100 hover:bg-destructive/20 hover:text-destructive text-muted-foreground transition-[color,background-color,opacity,transform] active:scale-[0.96]"
                      disabled={isDeleting === project.id}
                      title={t("projectList.delete")}
                      onClick={(e) => requestDelete(project, e)}
                    >
                      {isDeleting === project.id ? (
                        <Loader2 className="h-3 w-3 animate-spin" />
                      ) : (
                        <Trash2 className="h-3 w-3" />
                      )}
                    </button>
                  </div>

                  {stats && stats.totalSegments > 0 && (
                    <SegmentStatsBar stats={stats} />
                  )}
                </div>
              );
            })}
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
    </>
  );
}

function SegmentStatsBar({ stats }: { stats: ProjectStats }) {
  const {
    totalSegments,
    translatedCount,
    reviewedCount,
    needsReviewCount,
    untranslatedCount,
  } = stats;
  const translatedPct = (translatedCount / totalSegments) * 100;
  const reviewedPct = (reviewedCount / totalSegments) * 100;
  const reviewPct = (needsReviewCount / totalSegments) * 100;

  return (
    <div className="flex flex-col gap-1">
      <div className="h-1 w-full overflow-hidden rounded-full bg-muted">
        <div className="flex h-full">
          <div
            className="bg-green-500/60 transition-[width]"
            style={{ width: `${translatedPct}%` }}
          />
          <div
            className="bg-blue-400/60 transition-[width]"
            style={{ width: `${reviewedPct}%` }}
          />
          <div
            className="bg-amber-400/60 transition-[width]"
            style={{ width: `${reviewPct}%` }}
          />
        </div>
      </div>
      <div className="flex gap-3 font-mono text-[10px] tabular-nums text-muted-foreground">
        <span className={STATUS_SUMMARY.translated.className}>
          {STATUS_SUMMARY.translated.glyph} {translatedCount}
        </span>
        {reviewedCount > 0 && (
          <span className={STATUS_SUMMARY.reviewed.className}>
            {STATUS_SUMMARY.reviewed.glyph} {reviewedCount}
          </span>
        )}
        {needsReviewCount > 0 && (
          <span className={STATUS_SUMMARY.needsReview.className}>
            {STATUS_SUMMARY.needsReview.glyph} {needsReviewCount}
          </span>
        )}
        <span>
          {STATUS_SUMMARY.untranslated.glyph} {untranslatedCount}
        </span>
        <span className="ml-auto">{totalSegments}</span>
      </div>
    </div>
  );
}
