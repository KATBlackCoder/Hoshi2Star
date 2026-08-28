import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { open } from "@tauri-apps/plugin-dialog";
import { invoke } from "@tauri-apps/api/core";
import {
  BookOpenText,
  FolderOpen,
  Images,
  Languages,
  Loader2,
  Play,
  Telescope,
  Trash2,
} from "lucide-react";
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
import { useUiStore } from "@/stores/ui";

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
  const setMode = useUiStore((state) => state.setMode);

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
      <div className="observatory-grid h-full overflow-y-auto px-4 py-6 sm:px-6 lg:px-8">
        <div className="mx-auto w-full max-w-6xl">
          <header className="mb-6 flex flex-wrap items-end justify-between gap-4">
            <div className="max-w-2xl min-w-0">
              <p className="mb-2 flex items-center gap-2 text-[10px] font-semibold uppercase tracking-[0.18em] text-star">
                <Telescope className="size-3.5" aria-hidden="true" />
                {t("projectList.eyebrow")}
              </p>
              <h1 className="text-balance text-2xl font-semibold tracking-tight text-foreground sm:text-3xl">
                {t("projectList.title")}
              </h1>
              <p className="mt-2 text-pretty text-sm leading-6 text-muted-foreground">
                {t("projectList.subtitle")}
              </p>
            </div>
            <div className="flex shrink-0 gap-2">
              <LibraryMetric label={t("projectList.activeProjects")} value={projects.length} />
              <LibraryMetric label={t("projectList.supportedEngines")} value={2} />
            </div>
          </header>

        {projects.length === 0 ? (
          <div className="shrine-edge rounded-2xl bg-card/86 p-6 text-center shadow-[var(--shadow-surface)] backdrop-blur-sm sm:p-8">
            <span className="mx-auto grid size-12 place-items-center rounded-full bg-star/12 text-star shadow-[var(--shadow-surface)]">
              <Telescope className="size-5" aria-hidden="true" />
            </span>
            <h2 className="mt-4 text-lg font-semibold">{t("projectList.emptyTitle")}</h2>
            <p className="mx-auto mt-2 max-w-xl text-sm leading-6 text-muted-foreground">
              {t("projectList.emptyDescription")}
            </p>
            <Button
              className="mt-5 gap-2 shadow-[0_0_20px_color-mix(in_srgb,var(--star)_22%,transparent)]"
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
        ) : (
          <section aria-label={t("projectList.activeProjects")} className="grid w-full gap-3 xl:grid-cols-2">
            {projects.map((project) => {
              const stats = projectStats[project.id];
              return (
                <article
                  key={project.id}
                  className="group flex min-w-0 flex-col gap-4 rounded-2xl bg-card/86 p-4 shadow-[var(--shadow-surface)] backdrop-blur-sm transition-[background-color,box-shadow,transform] duration-[var(--duration-normal)] hover:bg-card hover:shadow-[var(--shadow-surface-hover)] focus-within:shadow-[var(--shadow-surface-hover)]"
                >
                  <div className="flex min-w-0 items-start gap-3">
                    <span className="mt-0.5 grid size-9 shrink-0 place-items-center rounded-xl bg-primary/10 text-primary shadow-[var(--shadow-surface)]">
                      <Languages className="size-4" aria-hidden="true" />
                    </span>
                    <div className="min-w-0 flex-1">
                      <h2 className="text-safe text-sm font-semibold leading-5">
                        {project.name}
                      </h2>
                      <p className="mt-0.5 truncate text-[11px] text-muted-foreground" title={project.gamePath}>
                        {project.gamePath}
                      </p>
                      <p className="mt-2 font-mono text-[10px] uppercase tracking-wide text-primary">
                        {project.sourceLang} → {project.targetLang}
                      </p>
                    </div>
                    <div className="flex shrink-0 flex-col items-end gap-1.5">
                      <span className="rounded-md bg-muted px-2 py-1 font-mono text-[10px] text-muted-foreground">
                        {engineLabel(project.engine)}
                      </span>
                      <span className="text-[10px] tabular-nums text-muted-foreground">
                        {relativeDate(project.updatedAt)}
                      </span>
                    </div>
                  </div>

                  {stats && stats.totalSegments > 0 && (
                    <SegmentStatsBar stats={stats} />
                  )}
                  <div className="flex flex-wrap justify-end gap-2 border-t border-border/70 pt-3">
                    <Button
                      size="sm"
                      variant="outline"
                      className="gap-1.5 text-xs"
                      disabled={isOpening === project.id}
                      onClick={() => void handleResume(project)}
                    >
                      {isOpening === project.id ? <Loader2 className="animate-spin" /> : <Play />}
                      {t("projectList.continue")}
                    </Button>
                    <Button
                      type="button"
                      size="icon-sm"
                      variant="destructive"
                      className="size-10"
                      disabled={isDeleting === project.id}
                      aria-label={`${t("projectList.delete")} — ${project.name}`}
                      title={t("projectList.delete")}
                      onClick={(event) => requestDelete(project, event)}
                    >
                      {isDeleting === project.id ? <Loader2 className="animate-spin" /> : <Trash2 />}
                    </Button>
                  </div>
                </article>
              );
            })}
          </section>
        )}

        {projects.length > 0 && (
          <Button
            size="sm"
            className="mt-6 gap-1.5 text-xs shadow-[0_0_16px_oklch(0.65_0.18_285/25%)]"
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
        )}
          <section className="mt-8 grid gap-3 md:grid-cols-3" aria-label={t("modes.navigation")}>
            <LibraryPortal
              icon={Languages}
              title={t("projectList.translationDesk")}
              description={t("projectList.translationDeskDescription")}
              onClick={() => setMode("patch")}
              label={t("projectList.openWorkspace", { name: t("modes.patch") })}
            />
            <LibraryPortal
              icon={BookOpenText}
              title={t("projectList.terminologyDesk")}
              description={t("projectList.terminologyDeskDescription")}
              onClick={() => setMode("terminology")}
              label={t("projectList.openWorkspace", { name: t("modes.terminology") })}
            />
            <LibraryPortal
              icon={Images}
              title={t("projectList.futureDesk")}
              description={t("projectList.futureDeskDescription")}
              onClick={() => setMode("player")}
              label={t("projectList.openWorkspace", { name: t("modes.player") })}
              future
            />
          </section>
        </div>
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
    <div className="flex flex-col gap-1.5">
      <div
        className="h-1.5 w-full overflow-hidden rounded-full bg-muted"
        role="progressbar"
        aria-label="Progression de la traduction"
        aria-valuemin={0}
        aria-valuemax={totalSegments}
        aria-valuenow={translatedCount + reviewedCount}
      >
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

function LibraryMetric({ label, value }: { label: string; value: number }) {
  return (
    <div className="min-w-24 rounded-xl bg-card/82 px-3 py-2 text-right shadow-[var(--shadow-surface)]">
      <p className="tabular-nums text-lg font-semibold text-star">{value}</p>
      <p className="text-[10px] text-muted-foreground">{label}</p>
    </div>
  );
}

function LibraryPortal({
  icon: Icon,
  title,
  description,
  onClick,
  label,
  future = false,
}: {
  icon: typeof Languages;
  title: string;
  description: string;
  onClick: () => void;
  label: string;
  future?: boolean;
}) {
  return (
    <button
      type="button"
      className="min-h-32 min-w-0 rounded-2xl bg-card/74 p-4 text-left shadow-[var(--shadow-surface)] transition-[background-color,box-shadow,transform] hover:bg-card hover:shadow-[var(--shadow-surface-hover)] active:scale-[0.96]"
      onClick={onClick}
      aria-label={label}
    >
      <span className="flex items-center justify-between gap-3">
        <span className="grid size-9 place-items-center rounded-xl bg-primary/10 text-primary">
          <Icon className="size-4" aria-hidden="true" />
        </span>
        {future && <span className="rounded-full bg-muted px-2 py-1 text-[9px] uppercase tracking-wide text-muted-foreground">Phase 2</span>}
      </span>
      <span className="mt-3 block text-sm font-semibold">{title}</span>
      <span className="mt-1 block text-pretty text-xs leading-5 text-muted-foreground">{description}</span>
    </button>
  );
}
