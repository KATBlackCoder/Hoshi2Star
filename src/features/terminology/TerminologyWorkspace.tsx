import { useDeferredValue, useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertCircle,
  BookOpenText,
  ChevronLeft,
  ChevronRight,
  Globe2,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";
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
import { useActiveProject } from "@/stores/project";
import { useSettingsStore } from "@/stores/settings";
import { useLlmStore } from "@/stores/llm";
import type { TerminologyEntry, TerminologyScanProgress } from "@/lib/types";
import { terminologyApi } from "./api";
import { terminologyKeys } from "./queryKeys";
import { ScanProgress } from "./ScanProgress";
import { TermEditorDialog, type TermEditorValue } from "./TermEditorDialog";
import { TermTranslateDialog } from "./TermTranslateDialog";
import { TerminologyTable } from "./TerminologyTable";
import { TerminologyToolbar } from "./TerminologyToolbar";
import { useTerminologyUiStore } from "./terminologyUiStore";
import { useTranslation } from "react-i18next";
import { WorkspaceState } from "@/components/common/WorkspaceState";

const PAGE_SIZE = 100;

export function TerminologyWorkspace() {
  const { t } = useTranslation();
  const activeProject = useActiveProject();
  const defaults = useSettingsStore((state) => state.settings);
  const providerConfig = useLlmStore((state) => state.providerConfig);
  const ui = useTerminologyUiStore();
  const queryClient = useQueryClient();
  const [targetOverrides, setTargetOverrides] = useState<
    Record<string, string>
  >({});
  const scopeKey = activeProject?.id ?? "global";
  const targetLanguage =
    targetOverrides[scopeKey] ??
    activeProject?.targetLang ??
    defaults.defaultTargetLang;
  const sourceLanguage =
    activeProject?.sourceLang ?? defaults.defaultSourceLang;
  const effectiveScope = activeProject ? ui.scope : "global";
  const projectId =
    effectiveScope === "project" ? (activeProject?.id ?? null) : null;
  const deferredSearch = useDeferredValue(ui.search);
  const [editorEntry, setEditorEntry] = useState<
    TerminologyEntry | null | undefined
  >(undefined);
  const [archiveEntry, setArchiveEntry] = useState<TerminologyEntry | null>(
    null,
  );
  const [scanProgress, setScanProgress] =
    useState<TerminologyScanProgress | null>(null);
  const [translateEntry, setTranslateEntry] = useState<
    TerminologyEntry | null | undefined
  >(undefined);
  const [deleteRequest, setDeleteRequest] = useState<{
    ids: string[];
    term?: string;
    bulk?: boolean;
  } | null>(null);
  const [globalizeRequest, setGlobalizeRequest] = useState<{
    mode: "row" | "selected" | "filtered";
    ids: string[];
    term?: string;
  } | null>(null);

  const queryInput = {
    sourceLanguage,
    targetLanguage,
    projectId,
    search: deferredSearch.trim() || null,
    partOfSpeech: ui.partOfSpeech === "all" ? null : ui.partOfSpeech,
    semanticType: ui.semanticType.trim() || null,
    status: ui.status === "all" ? null : ui.status,
    page: ui.page,
    pageSize: PAGE_SIZE,
  };
  const list = useQuery({
    queryKey: terminologyKeys.list(queryInput),
    queryFn: () => terminologyApi.list(queryInput),
  });
  const stats = useQuery({
    queryKey: terminologyKeys.stats(sourceLanguage, targetLanguage, projectId),
    queryFn: () =>
      terminologyApi.stats(sourceLanguage, targetLanguage, projectId),
  });

  useEffect(() => {
    const cleanup = Promise.all([
      terminologyApi.onScanProgress((progress) => {
        if (progress.projectId === activeProject?.id) setScanProgress(progress);
      }),
      terminologyApi.onScanDone((done) => {
        if (done.projectId !== activeProject?.id) return;
        setScanProgress(null);
        void queryClient.invalidateQueries({ queryKey: terminologyKeys.all });
        if (done.status === "failed")
          toast.error(done.error ?? t("terminology.scanFailed"));
        else
          toast.success(
            t(
              done.status === "cancelled"
                ? "terminology.scanCancelled"
                : "terminology.scanDone",
            ),
          );
      }),
    ]);
    return () => {
      void cleanup.then((unlisten) => unlisten.forEach((fn) => fn()));
    };
  }, [activeProject?.id, queryClient, t]);

  const invalidate = () =>
    queryClient.invalidateQueries({ queryKey: terminologyKeys.all });
  const save = useMutation({
    mutationFn: async (value: TermEditorValue) => {
      if (!editorEntry)
        return terminologyApi.create({
          sourceLanguage,
          canonicalText: value.canonicalText,
          reading: value.reading,
          partOfSpeech: value.partOfSpeech,
          semanticType: value.semanticType,
          senseKey: "",
        });
      const updated = await terminologyApi.update({
        id: editorEntry.id,
        canonicalText: value.canonicalText,
        reading: value.reading,
        partOfSpeech: value.partOfSpeech,
        semanticType: value.semanticType,
        senseKey: editorEntry.senseKey,
        status: editorEntry.status,
      });
      if (!value.targetText.trim()) return updated;
      return terminologyApi.upsertTranslation({
        entryId: editorEntry.id,
        targetLanguage,
        projectId,
        targetText: value.targetText,
        enforcement: value.enforcement,
        confidence: 1,
        providerId: null,
        model: null,
        acceptedVariants: editorEntry.translation?.acceptedVariants ?? [],
      });
    },
    onSuccess: async () => {
      setEditorEntry(undefined);
      await invalidate();
      toast.success(t("terminology.saved"));
    },
    onError: (error) => toast.error(String(error)),
  });
  const archive = useMutation({
    mutationFn: (entryId: string) => terminologyApi.archive(entryId),
    onSuccess: async () => {
      setArchiveEntry(null);
      await invalidate();
      toast.success(t("terminology.archivedDone"));
    },
    onError: (error) => toast.error(String(error)),
  });
  const deleteEntries = useMutation({
    mutationFn: (entryIds: string[]) => terminologyApi.deleteEntries(entryIds),
    onSuccess: async (deleted, entryIds) => {
      setDeleteRequest(null);
      ui.removeSelected(entryIds);
      const visibleEntries = list.data?.items ?? [];
      if (
        ui.page > 0 &&
        visibleEntries.length > 0 &&
        visibleEntries.every((entry) => entryIds.includes(entry.id))
      ) {
        ui.setPage(ui.page - 1);
      }
      await invalidate();
      toast.success(t("terminology.deletedDone", { count: deleted }));
    },
    onError: (error) => toast.error(String(error)),
  });
  const globalize = useMutation({
    mutationFn: async (request: NonNullable<typeof globalizeRequest>) => {
      if (!activeProject) throw new Error(t("terminology.chooseProject"));
      if (request.mode === "filtered") {
        return terminologyApi.globalizeFiltered({
          ...queryInput,
          projectId: activeProject.id,
          page: 0,
        });
      }
      return terminologyApi.globalize({
        entryIds: request.ids,
        targetLanguage,
        projectId: activeProject.id,
      });
    },
    onSuccess: async (summary) => {
      setGlobalizeRequest(null);
      ui.clearSelection();
      await invalidate();
      toast.success(
        t("terminology.globalizeDone", {
          copied: summary.copied,
          same: summary.alreadyGlobal,
          conflicts: summary.conflicts,
          skipped: summary.skipped,
        }),
      );
    },
    onError: (error) => toast.error(String(error)),
  });

  async function startScan() {
    if (!activeProject) return;
    try {
      const started = await terminologyApi.startScan(activeProject.id);
      setScanProgress({
        scanId: started.scanId,
        projectId: activeProject.id,
        processed: 0,
        total: 0,
        discovered: 0,
      });
    } catch (error) {
      toast.error(String(error));
    }
  }
  const canScan = Boolean(
    activeProject && sourceLanguage.toLowerCase().startsWith("ja"),
  );
  const totalPages = Math.max(
    1,
    Math.ceil((list.data?.total ?? 0) / PAGE_SIZE),
  );

  return (
    <main
      className="observatory-grid flex h-full min-h-0 min-w-0 flex-col gap-3 overflow-hidden p-3 sm:p-4"
      aria-busy={list.isLoading}
    >
      <header className="flex min-w-0 flex-wrap items-end justify-between gap-3">
        <div className="min-w-0 flex-1">
          <h1 className="flex items-center gap-2 text-balance text-xl font-semibold">
            <BookOpenText className="size-5 text-primary" />{" "}
            {t("terminology.title")}
          </h1>
          <p className="mt-1 text-pretty text-sm text-muted-foreground">
            {t(
              activeProject
                ? "terminology.subtitleProject"
                : "terminology.subtitleGlobal",
              {
                project: activeProject?.name,
                source: sourceLanguage.toUpperCase(),
                target: targetLanguage.toUpperCase(),
              },
            )}
          </p>
        </div>
        {stats.data && (
          <dl className="flex max-w-full gap-3 overflow-x-auto rounded-xl bg-card/82 px-3 py-2 text-xs shadow-[var(--shadow-surface)] [scrollbar-width:none]">
            <Stat
              label={t("terminology.terms")}
              value={stats.data.totalEntries}
            />
            <Stat
              label={t("terminology.untranslated")}
              value={stats.data.untranslatedEntries}
            />
            <Stat
              label={t("terminology.project")}
              value={stats.data.projectTranslations}
            />
            <Stat
              label={t("terminology.global")}
              value={stats.data.globalTranslations}
            />
          </dl>
        )}
      </header>
      <TerminologyToolbar
        search={ui.search}
        onSearch={ui.setSearch}
        partOfSpeech={ui.partOfSpeech}
        onPartOfSpeech={ui.setPartOfSpeech}
        semanticType={ui.semanticType}
        onSemanticType={ui.setSemanticType}
        status={ui.status}
        onStatus={ui.setStatus}
        targetLanguage={targetLanguage}
        onTargetLanguage={(value) => {
          setTargetOverrides((current) => ({ ...current, [scopeKey]: value }));
          ui.setPage(0);
        }}
        projectAvailable={Boolean(activeProject)}
        scope={effectiveScope}
        onScope={ui.setScope}
        canScan={canScan}
        scanDisabledReason={
          !activeProject
            ? t("terminology.chooseProject")
            : !sourceLanguage.startsWith("ja")
              ? t("terminology.japaneseOnly")
              : undefined
        }
        scanning={Boolean(scanProgress)}
        onScan={() => void startScan()}
        onCreate={() => setEditorEntry(null)}
        onTranslate={() => setTranslateEntry(null)}
      />
      {scanProgress && (
        <ScanProgress
          progress={scanProgress}
          onCancel={() => void terminologyApi.cancelScan(scanProgress.scanId)}
        />
      )}
      {list.isError ? (
        <WorkspaceState
          icon={<AlertCircle />}
          title={t("terminology.loadError")}
          description={String(list.error)}
          tone="error"
          className="flex-1"
        />
      ) : list.isLoading ? (
        <WorkspaceState title={t("terminology.loading")} className="flex-1" />
      ) : list.data?.items.length ? (
        <TerminologyTable
          entries={list.data.items}
          selectedIds={ui.selectedIds}
          onToggleSelected={ui.toggleSelected}
          onTogglePage={ui.setPageSelection}
          onTranslate={setTranslateEntry}
          onEdit={setEditorEntry}
          onGlobalize={(entry) =>
            setGlobalizeRequest({
              mode: "row",
              ids: [entry.id],
              term: entry.canonicalText,
            })
          }
          onArchive={setArchiveEntry}
          onDelete={(entry) =>
            setDeleteRequest({
              ids: [entry.id],
              term: entry.canonicalText,
            })
          }
        />
      ) : (
        <WorkspaceState
          icon={<BookOpenText />}
          title={t("terminology.empty")}
          description={t(
            activeProject
              ? "terminology.emptyProject"
              : "terminology.emptyGlobal",
          )}
          className="flex-1"
        />
      )}
      <footer className="flex min-w-0 flex-wrap items-center justify-between gap-2 text-xs text-muted-foreground">
        <div className="flex min-w-0 flex-wrap items-center gap-2">
          <span className="tabular-nums">
            {t("terminology.countSelected", {
              count: list.data?.total ?? 0,
              selected: ui.selectedIds.length,
            })}
          </span>
          {ui.selectedIds.length > 0 && (
            <>
              {activeProject && effectiveScope === "project" && (
                <Button
                  size="sm"
                  variant="outline"
                  disabled={globalize.isPending}
                  onClick={() =>
                    setGlobalizeRequest({
                      mode: "selected",
                      ids: [...ui.selectedIds],
                    })
                  }
                >
                  <Globe2 />
                  {t("terminology.globalizeSelected", {
                    count: ui.selectedIds.length,
                  })}
                </Button>
              )}
              <Button
                size="sm"
                variant="destructive"
                disabled={deleteEntries.isPending}
                onClick={() =>
                  setDeleteRequest({ ids: [...ui.selectedIds], bulk: true })
                }
              >
                <Trash2 />
                {t("terminology.deleteSelected", {
                  count: ui.selectedIds.length,
                })}
              </Button>
            </>
          )}
          {activeProject &&
            effectiveScope === "project" &&
            (stats.data?.projectTranslations ?? 0) > 0 && (
              <Button
                size="sm"
                variant="outline"
                disabled={globalize.isPending}
                onClick={() =>
                  setGlobalizeRequest({ mode: "filtered", ids: [] })
                }
              >
                <Globe2 /> {t("terminology.globalizeFiltered")}
              </Button>
            )}
        </div>
        <div className="flex items-center gap-2">
          <Button
            size="icon-sm"
            variant="outline"
            aria-label={t("terminology.previousPage")}
            disabled={ui.page === 0}
            onClick={() => ui.setPage(ui.page - 1)}
          >
            <ChevronLeft />
          </Button>
          <span className="tabular-nums">
            {t("terminology.page", { page: ui.page + 1, pages: totalPages })}
          </span>
          <Button
            size="icon-sm"
            variant="outline"
            aria-label={t("terminology.nextPage")}
            disabled={ui.page + 1 >= totalPages}
            onClick={() => ui.setPage(ui.page + 1)}
          >
            <ChevronRight />
          </Button>
        </div>
      </footer>
      {editorEntry !== undefined && (
        <TermEditorDialog
          key={editorEntry?.id ?? "new"}
          open
          entry={editorEntry}
          targetLanguage={targetLanguage}
          onOpenChange={(open) => {
            if (!open) setEditorEntry(undefined);
          }}
          onSave={(value) => save.mutate(value)}
          saving={save.isPending}
        />
      )}
      {translateEntry !== undefined && (
        <TermTranslateDialog
          open
          entries={translateEntry ? [translateEntry] : (list.data?.items ?? [])}
          selectedIds={translateEntry ? [translateEntry.id] : ui.selectedIds}
          targetLanguage={targetLanguage}
          projectId={activeProject?.id ?? null}
          defaultScope={effectiveScope}
          providerConfig={providerConfig}
          onOpenChange={(open) => {
            if (!open) setTranslateEntry(undefined);
          }}
          onDone={() => {
            ui.clearSelection();
            void invalidate();
          }}
        />
      )}
      <AlertDialog
        open={Boolean(globalizeRequest)}
        onOpenChange={(open) => {
          if (!open && !globalize.isPending) setGlobalizeRequest(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t("terminology.globalizeTitle")}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t("terminology.globalizeDescription", {
                scope:
                  globalizeRequest?.mode === "filtered"
                    ? t("terminology.currentFilters")
                    : globalizeRequest?.term ??
                      t("terminology.selectedCount", {
                        count: globalizeRequest?.ids.length ?? 0,
                      }),
              })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={globalize.isPending}>
              {t("terminology.cancel")}
            </AlertDialogCancel>
            <AlertDialogAction
              disabled={globalize.isPending}
              onClick={() => globalizeRequest && globalize.mutate(globalizeRequest)}
            >
              {t("terminology.globalizeConfirm")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
      <AlertDialog
        open={Boolean(archiveEntry)}
        onOpenChange={(open) => {
          if (!open) setArchiveEntry(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("terminology.archiveTitle")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("terminology.archiveDescription", {
                term: archiveEntry?.canonicalText,
              })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("terminology.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              onClick={() => archiveEntry && archive.mutate(archiveEntry.id)}
            >
              {t("terminology.archived")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
      <AlertDialog
        open={Boolean(deleteRequest)}
        onOpenChange={(open) => {
          if (!open && !deleteEntries.isPending) setDeleteRequest(null);
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              {t(
                deleteRequest?.bulk
                  ? "terminology.deleteManyTitle"
                  : "terminology.deleteTitle",
              )}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {deleteRequest?.bulk
                ? t("terminology.deleteManyDescription", {
                    count: deleteRequest?.ids.length ?? 0,
                  })
                : t("terminology.deleteDescription", {
                    term: deleteRequest?.term ?? "",
                  })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={deleteEntries.isPending}>
              {t("terminology.cancel")}
            </AlertDialogCancel>
            <AlertDialogAction
              variant="destructive"
              disabled={deleteEntries.isPending}
              onClick={() =>
                deleteRequest && deleteEntries.mutate(deleteRequest.ids)
              }
            >
              {t("terminology.deleteConfirm")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </main>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return (
    <div>
      <dt className="text-muted-foreground">{label}</dt>
      <dd className="tabular-nums text-sm font-semibold text-foreground">
        {value}
      </dd>
    </div>
  );
}
