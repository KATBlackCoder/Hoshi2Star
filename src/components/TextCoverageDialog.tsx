import { useEffect, useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  AlertTriangle,
  ChevronDown,
  ChevronRight,
  Info,
  Loader2,
  Link2,
  ShieldCheck,
  ShieldX,
} from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
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
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import type {
  CategoryChange,
  CategoryState,
  CoverageCategory,
  CoverageNotCovered,
  CoverageReference,
} from "@/lib/types";
import { useShallow } from "zustand/react/shallow";
import {
  applyCoverageChanges,
  useCoverageStore,
} from "@/stores/coverage";
import { refreshProjectData } from "@/stores/project";
import { openResyncDialog } from "@/stores/resync";
import { useIsTranslating } from "@/stores/llm";

interface TextCoverageDialogProps {
  projectId: string | null;
}

const STATE_ORDER: CategoryState[] = ["verified", "linked", "blocked"];
const REFERENCES_SHOWN = 8;

/**
 * Review optional player-visible text (plugin parameters, System lists, map
 * names…) before it becomes translatable. Every category states where it is
 * visible, what enabling it changes, and — when it cannot be translated
 * safely — exactly which game data would break.
 */
export function TextCoverageDialog({ projectId }: TextCoverageDialogProps) {
  const { t } = useTranslation();
  const { open, loading, view, error, setOpen } = useCoverageStore(
    useShallow((s) => ({
      open: s.open,
      loading: s.loading,
      view: s.view,
      error: s.error,
      setOpen: s.setOpen,
    })),
  );
  const isTranslating = useIsTranslating();
  const [draft, setDraft] = useState<Record<string, boolean>>({});
  const [expanded, setExpanded] = useState<Record<string, boolean>>({});
  const [confirmLinked, setConfirmLinked] = useState(false);

  // Re-seed the draft whenever the dialog opens or a new scan arrives: the
  // dialog stays mounted, so initial state alone would go stale.
  useEffect(() => {
    if (!open || !view) return;
    const next: Record<string, boolean> = {};
    for (const category of view.categories) {
      next[category.key] = category.decided
        ? category.enabled
        : category.state === "verified" && category.defaultEnabled;
    }
    setDraft(next);
    setExpanded({});
  }, [open, view]);

  const changes = useMemo<CategoryChange[]>(() => {
    if (!view) return [];
    return view.categories
      .filter((category) => category.state !== "blocked" || category.enabled)
      .filter(
        (category) =>
          !category.decided ||
          (draft[category.key] ?? false) !== category.enabled,
      )
      .map((category) => ({
        categoryKey: category.key,
        enabled: draft[category.key] ?? false,
      }));
  }, [view, draft]);

  const linkedToEnable = useMemo(
    () =>
      (view?.categories ?? []).filter(
        (category) =>
          category.state === "linked" &&
          (draft[category.key] ?? false) &&
          !category.enabled,
      ),
    [view, draft],
  );

  async function apply(acceptLinked: boolean) {
    if (!projectId) return;
    setConfirmLinked(false);
    const payload = changes.map((change) => ({
      ...change,
      acceptConsequences:
        acceptLinked &&
        linkedToEnable.some((category) => category.key === change.categoryKey),
    }));
    try {
      await applyCoverageChanges(projectId, payload);
      await refreshProjectData(projectId);
      toast.success(t("coverage.saved"));
      setOpen(false);
    } catch (err) {
      toast.error(t("coverage.saveError", { error: String(err) }));
    }
  }

  function handleSave() {
    if (linkedToEnable.length > 0) {
      setConfirmLinked(true);
      return;
    }
    void apply(false);
  }

  const grouped = useMemo(() => {
    const groups = new Map<CategoryState, CoverageCategory[]>();
    for (const category of view?.categories ?? []) {
      const list = groups.get(category.state) ?? [];
      list.push(category);
      groups.set(category.state, list);
    }
    return STATE_ORDER.map((state) => [state, groups.get(state) ?? []] as const)
      .filter(([, list]) => list.length > 0);
  }, [view]);

  return (
    <>
      <Dialog open={open} onOpenChange={setOpen}>
        <DialogContent className="w-[min(94vw,46rem)]">
          <DialogHeader>
            <DialogTitle>{t("coverage.title")}</DialogTitle>
            <DialogDescription>{t("coverage.description")}</DialogDescription>
          </DialogHeader>

          {loading && !view && (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <Loader2 className="h-4 w-4 animate-spin" />
              {t("coverage.scanning")}
            </div>
          )}
          {error && (
            <p role="alert" className="text-sm text-destructive">
              {error}
            </p>
          )}

          {view && !view.supported && (
            <p className="text-sm text-muted-foreground">
              {t("coverage.unsupported")}
            </p>
          )}

          {view?.supported && (
            <div className="space-y-4">
              <Notices
                reconstructed={view.reconstructed}
                driftedFiles={view.driftedFiles}
                pluginsUnreadable={view.pluginsUnreadable}
                onResync={
                  projectId
                    ? () => {
                        setOpen(false);
                        void openResyncDialog(projectId);
                      }
                    : undefined
                }
              />

              {view.categories.length === 0 && (
                <p className="text-sm text-muted-foreground">
                  {t("coverage.nothingFound")}
                </p>
              )}

              {grouped.map(([state, categories]) => (
                <section key={state} aria-labelledby={`coverage-${state}`}>
                  <h3
                    id={`coverage-${state}`}
                    className="mb-2 flex items-center gap-1.5 text-sm font-medium"
                  >
                    <StateIcon state={state} />
                    {t(`coverage.state.${state}`)}
                  </h3>
                  <p className="mb-2 text-xs text-muted-foreground">
                    {t(`coverage.stateHint.${state}`)}
                  </p>
                  <ul className="space-y-2">
                    {categories.map((category) => (
                      <CategoryRow
                        key={category.key}
                        category={category}
                        checked={draft[category.key] ?? false}
                        expanded={expanded[category.key] ?? false}
                        disabled={
                          category.state === "blocked" || isTranslating
                        }
                        onCheckedChange={(value) =>
                          setDraft((d) => ({ ...d, [category.key]: value }))
                        }
                        onToggleDetails={() =>
                          setExpanded((e) => ({
                            ...e,
                            [category.key]: !e[category.key],
                          }))
                        }
                      />
                    ))}
                  </ul>
                </section>
              ))}

              {view.notCovered.length > 0 && (
                <NotCoveredList items={view.notCovered} />
              )}

              {view.coreWarnings.length > 0 && (
                <CoreWarnings warnings={view.coreWarnings} />
              )}
            </div>
          )}

          <DialogFooter>
            {isTranslating && (
              <p className="mr-auto self-center text-xs text-muted-foreground">
                {t("coverage.busyTranslating")}
              </p>
            )}
            <Button variant="outline" onClick={() => setOpen(false)}>
              {t("coverage.close")}
            </Button>
            <Button
              onClick={handleSave}
              disabled={
                !view?.supported ||
                loading ||
                isTranslating ||
                changes.length === 0
              }
            >
              {loading && view ? (
                <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
              ) : null}
              {t("coverage.save")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={confirmLinked}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("coverage.linkedConfirmTitle")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("coverage.linkedConfirmDesc")}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <ul className="max-h-64 space-y-3 overflow-y-auto text-sm">
            {linkedToEnable.map((category) => (
              <li key={category.key}>
                <p className="font-medium">{categoryTitle(t, category)}</p>
                <ul className="ml-4 list-disc text-muted-foreground">
                  {category.consequences.map((line) => (
                    <li key={line}>{line}</li>
                  ))}
                </ul>
              </li>
            ))}
          </ul>
          <AlertDialogFooter>
            <AlertDialogCancel onClick={() => setConfirmLinked(false)}>
              {t("coverage.linkedConfirmCancel")}
            </AlertDialogCancel>
            <AlertDialogAction onClick={() => void apply(true)}>
              {t("coverage.linkedConfirmAccept")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}

type TFn = ReturnType<typeof useTranslation>["t"];

function categoryTitle(t: TFn, category: CoverageCategory): string {
  return t(`coverage.category.${category.key}.title`, {
    defaultValue: category.title,
  });
}

function StateIcon({ state }: { state: CategoryState }) {
  if (state === "verified")
    return <ShieldCheck className="h-4 w-4 text-green-500" aria-hidden />;
  if (state === "linked")
    return <Link2 className="h-4 w-4 text-amber-500" aria-hidden />;
  return <ShieldX className="h-4 w-4 text-destructive" aria-hidden />;
}

interface CategoryRowProps {
  category: CoverageCategory;
  checked: boolean;
  expanded: boolean;
  disabled: boolean;
  onCheckedChange: (value: boolean) => void;
  onToggleDetails: () => void;
}

function CategoryRow({
  category,
  checked,
  expanded,
  disabled,
  onCheckedChange,
  onToggleDetails,
}: CategoryRowProps) {
  const { t } = useTranslation();
  const checkboxId = `coverage-toggle-${category.key}`;
  const detailsId = `coverage-details-${category.key}`;
  return (
    <li className="rounded-lg border p-3">
      <div className="flex items-start gap-3">
        <Checkbox
          id={checkboxId}
          className="mt-0.5"
          checked={checked}
          disabled={disabled}
          onCheckedChange={(value) => onCheckedChange(value === true)}
          aria-describedby={detailsId}
        />
        <div className="min-w-0 flex-1">
          <div className="flex flex-wrap items-center gap-2">
            <label htmlFor={checkboxId} className="text-sm font-medium">
              {categoryTitle(t, category)}
            </label>
            <Badge variant="secondary">
              {t("coverage.textCount", { count: category.segmentCount })}
            </Badge>
            {category.enabled && (
              <Badge variant="outline">
                {t("coverage.translatedCount", {
                  done: category.translatedCount,
                  total: category.segmentCount,
                })}
              </Badge>
            )}
          </div>
          <p className="mt-0.5 text-xs text-muted-foreground">
            {t("coverage.visibleIn")}{" "}
            {t(`coverage.category.${category.key}.visibleIn`, {
              defaultValue: category.visibleIn,
            })}
          </p>
          {category.blockedReason && (
            <p className="mt-1 text-xs text-destructive">
              {category.blockedReason}
            </p>
          )}
          <button
            type="button"
            className="mt-1 inline-flex items-center gap-1 text-xs text-primary hover:underline"
            aria-expanded={expanded}
            aria-controls={detailsId}
            onClick={onToggleDetails}
          >
            {expanded ? (
              <ChevronDown className="h-3 w-3" />
            ) : (
              <ChevronRight className="h-3 w-3" />
            )}
            {expanded ? t("coverage.hideDetails") : t("coverage.showDetails")}
          </button>
          <div id={detailsId} hidden={!expanded} className="mt-2 space-y-2 text-xs">
            {category.consequences.length > 0 && (
              <div>
                <p className="font-medium">{t("coverage.consequences")}</p>
                <ul className="ml-4 list-disc text-muted-foreground">
                  {category.consequences.map((line) => (
                    <li key={line}>{line}</li>
                  ))}
                </ul>
              </div>
            )}
            {category.constraints.length > 0 && (
              <div>
                <p className="font-medium">{t("coverage.constraints")}</p>
                <ul className="ml-4 list-disc text-muted-foreground">
                  {category.constraints.map((constraint) => (
                    <li key={constraint.rule}>
                      {t(`coverage.constraint.${constraint.rule}`, {
                        defaultValue: constraint.rule,
                      })}
                    </li>
                  ))}
                </ul>
              </div>
            )}
            {category.samples.length > 0 && (
              <div>
                <p className="font-medium">{t("coverage.samples")}</p>
                <div className="mt-1 flex flex-wrap gap-1">
                  {category.samples.map((sample, index) => (
                    <Badge key={`${sample}-${index}`} variant="outline">
                      {sample}
                    </Badge>
                  ))}
                </div>
              </div>
            )}
            {category.referenceCount > 0 && (
              <ReferenceList
                references={category.references}
                total={category.referenceCount}
              />
            )}
          </div>
        </div>
      </div>
    </li>
  );
}

function ReferenceList({
  references,
  total,
}: {
  references: CoverageReference[];
  total: number;
}) {
  const { t } = useTranslation();
  const shown = references.slice(0, REFERENCES_SHOWN);
  return (
    <div>
      <p className="font-medium">
        {t("coverage.references", { count: total })}
      </p>
      <ul className="mt-1 space-y-1">
        {shown.map((reference, index) => (
          <li
            key={`${reference.location}-${reference.text}-${index}`}
            className="rounded bg-muted/50 px-2 py-1"
          >
            <span className="font-mono">{reference.text}</span>
            {" — "}
            {t(`coverage.site.${reference.site}`)}
            {" · "}
            <span className="font-mono text-muted-foreground">
              {reference.location}
            </span>
            <div className="truncate font-mono text-muted-foreground">
              {reference.excerpt}
            </div>
          </li>
        ))}
      </ul>
      {total > shown.length && (
        <p className="mt-1 text-muted-foreground">
          {t("coverage.moreReferences", { count: total - shown.length })}
        </p>
      )}
    </div>
  );
}

function Notices({
  reconstructed,
  driftedFiles,
  pluginsUnreadable,
  onResync,
}: {
  reconstructed: boolean;
  driftedFiles: { file: string; pointers: number }[];
  pluginsUnreadable: string | null;
  onResync?: () => void;
}) {
  const { t } = useTranslation();
  if (!reconstructed && driftedFiles.length === 0 && !pluginsUnreadable) {
    return null;
  }
  return (
    <div className="space-y-2">
      {reconstructed && (
        <p className="flex gap-2 rounded-md bg-muted/60 p-2 text-xs">
          <Info className="mt-0.5 h-3.5 w-3.5 shrink-0" aria-hidden />
          {t("coverage.reconstructedNotice")}
        </p>
      )}
      {driftedFiles.length > 0 && (
        <p className="flex gap-2 rounded-md bg-amber-500/10 p-2 text-xs">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-500" aria-hidden />
          <span>
            {t("coverage.driftNotice", {
              files: driftedFiles.map((entry) => entry.file).join(", "),
            })}
            {onResync && (
              <>
                {" "}
                <button
                  type="button"
                  className="font-medium underline underline-offset-2"
                  onClick={onResync}
                >
                  {t("coverage.driftResync")}
                </button>
              </>
            )}
          </span>
        </p>
      )}
      {pluginsUnreadable && (
        <p className="flex gap-2 rounded-md bg-amber-500/10 p-2 text-xs">
          <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-500" aria-hidden />
          {t("coverage.pluginsUnreadable", { error: pluginsUnreadable })}
        </p>
      )}
    </div>
  );
}

function NotCoveredList({ items }: { items: CoverageNotCovered[] }) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const byPlugin = useMemo(() => {
    const groups = new Map<string, CoverageNotCovered[]>();
    for (const item of items) {
      const list = groups.get(item.plugin) ?? [];
      list.push(item);
      groups.set(item.plugin, list);
    }
    return [...groups.entries()];
  }, [items]);
  return (
    <section className="rounded-lg border border-dashed p-3">
      <button
        type="button"
        className="inline-flex items-center gap-1 text-sm font-medium"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        {open ? (
          <ChevronDown className="h-3.5 w-3.5" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5" />
        )}
        {t("coverage.notCoveredTitle", { count: items.length })}
      </button>
      <p className="mt-1 text-xs text-muted-foreground">
        {t("coverage.notCoveredHint")}
      </p>
      {open && (
        <ul className="mt-2 space-y-2 text-xs">
          {byPlugin.map(([plugin, rows]) => (
            <li key={plugin}>
              <p className="font-mono font-medium">{plugin}</p>
              <p className="text-muted-foreground">{rows[0].reason}</p>
              <ul className="mt-1 space-y-1">
                {rows.map((row, index) => (
                  <li
                    key={`${row.field}-${index}`}
                    className="rounded bg-muted/50 px-2 py-1"
                  >
                    <span className="font-mono">{row.field}</span>
                    {" — "}
                    <span>{row.sample}</span>
                    {row.hint && (
                      <div className="text-muted-foreground">
                        {t("coverage.notCoveredHintLabel")} {row.hint}
                      </div>
                    )}
                  </li>
                ))}
              </ul>
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}

function CoreWarnings({
  warnings,
}: {
  warnings: { segmentKind: string; source: string; references: CoverageReference[] }[];
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  return (
    <section className="rounded-lg border border-dashed p-3">
      <button
        type="button"
        className="inline-flex items-center gap-1 text-sm font-medium"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        {open ? (
          <ChevronDown className="h-3.5 w-3.5" />
        ) : (
          <ChevronRight className="h-3.5 w-3.5" />
        )}
        {t("coverage.coreWarningsTitle", { count: warnings.length })}
      </button>
      <p className="mt-1 text-xs text-muted-foreground">
        {t("coverage.coreWarningsHint")}
      </p>
      {open && (
        <ul className="mt-2 space-y-2 text-xs">
          {warnings.map((warning) => (
            <li key={`${warning.segmentKind}-${warning.source}`}>
              <span className="font-mono">{warning.source}</span>
              <ReferenceList
                references={warning.references}
                total={warning.references.length}
              />
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
