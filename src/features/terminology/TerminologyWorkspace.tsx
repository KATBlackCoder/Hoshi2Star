import { useDeferredValue, useEffect, useState } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import {
  AlertCircle,
  BookOpenText,
  ChevronLeft,
  ChevronRight,
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
import type { TerminologyEntry, TerminologyScanProgress } from "@/lib/types";
import { terminologyApi } from "./api";
import { terminologyKeys } from "./queryKeys";
import { ScanProgress } from "./ScanProgress";
import { TermEditorDialog, type TermEditorValue } from "./TermEditorDialog";
import { TerminologyTable } from "./TerminologyTable";
import { TerminologyToolbar } from "./TerminologyToolbar";
import { useTerminologyUiStore } from "./terminologyUiStore";
import { useTranslation } from "react-i18next";

const PAGE_SIZE = 100;

export function TerminologyWorkspace() {
  const { t } = useTranslation();
  const activeProject = useActiveProject();
  const defaults = useSettingsStore((state) => state.settings);
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
      await terminologyApi.update({
        id: editorEntry.id,
        canonicalText: value.canonicalText,
        reading: value.reading,
        partOfSpeech: value.partOfSpeech,
        semanticType: value.semanticType,
        senseKey: editorEntry.senseKey,
        status: editorEntry.status,
      });
      return terminologyApi.upsertTranslation({
        entryId: editorEntry.id,
        targetLanguage,
        projectId,
        targetText: value.targetText,
        reviewStatus: value.reviewStatus,
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
      className="flex h-full min-h-0 flex-col gap-3 overflow-hidden p-4"
      aria-busy={list.isLoading}
    >
      <header className="flex flex-wrap items-end justify-between gap-4">
        <div>
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
          <dl className="flex gap-4 rounded-xl bg-muted/60 px-3 py-2 text-xs">
            <Stat
              label={t("terminology.terms")}
              value={stats.data.totalEntries}
            />
            <Stat
              label={t("terminology.untranslated")}
              value={stats.data.untranslatedEntries}
            />
            <Stat
              label={t("terminology.proposed")}
              value={stats.data.proposedTranslations}
            />
            <Stat
              label={t("terminology.validated")}
              value={
                stats.data.approvedTranslations + stats.data.lockedTranslations
              }
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
      />
      {scanProgress && (
        <ScanProgress
          progress={scanProgress}
          onCancel={() => void terminologyApi.cancelScan(scanProgress.scanId)}
        />
      )}
      {list.isError ? (
        <StateMessage
          icon={<AlertCircle />}
          title={t("terminology.loadError")}
          detail={String(list.error)}
        />
      ) : list.isLoading ? (
        <StateMessage title={t("terminology.loading")} />
      ) : list.data?.items.length ? (
        <TerminologyTable
          entries={list.data.items}
          selectedIds={ui.selectedIds}
          onToggleSelected={ui.toggleSelected}
          onEdit={setEditorEntry}
          onArchive={setArchiveEntry}
        />
      ) : (
        <StateMessage
          icon={<BookOpenText />}
          title={t("terminology.empty")}
          detail={t(
            activeProject
              ? "terminology.emptyProject"
              : "terminology.emptyGlobal",
          )}
        />
      )}
      <footer className="flex items-center justify-between text-xs text-muted-foreground">
        <span className="tabular-nums">
          {t("terminology.countSelected", {
            count: list.data?.total ?? 0,
            selected: ui.selectedIds.length,
          })}
        </span>
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
function StateMessage({
  icon,
  title,
  detail,
}: {
  icon?: React.ReactNode;
  title: string;
  detail?: string;
}) {
  return (
    <div className="grid min-h-48 flex-1 place-items-center rounded-2xl bg-card/65 p-8 text-center shadow-[var(--shadow-surface)]">
      <div>
        {icon && (
          <span className="mx-auto mb-3 grid size-10 place-items-center rounded-xl bg-muted text-muted-foreground">
            {icon}
          </span>
        )}
        <p className="font-medium">{title}</p>
        {detail && (
          <p
            className="mt-1 max-w-xl text-sm text-muted-foreground"
            role={title.startsWith("Impossible") ? "alert" : undefined}
          >
            {detail}
          </p>
        )}
      </div>
    </div>
  );
}
