import { useMemo, useRef, useState } from "react";
import { Loader2, Sparkles } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type { ProviderConfig, TerminologyEntry } from "@/lib/types";
import { terminologyApi } from "./api";
import { TerminologySelect } from "./TerminologySelect";

type SelectionMode = "selected" | "visible" | "untranslated";

export function TermTranslateDialog({
  open,
  entries,
  selectedIds,
  targetLanguage,
  projectId,
  defaultScope,
  providerConfig,
  onOpenChange,
  onDone,
}: {
  open: boolean;
  entries: TerminologyEntry[];
  selectedIds: string[];
  targetLanguage: string;
  projectId: string | null;
  defaultScope?: "project" | "global";
  providerConfig: ProviderConfig;
  onOpenChange: (open: boolean) => void;
  onDone: () => void;
}) {
  const { t } = useTranslation();
  const [selectionMode, setSelectionMode] = useState<SelectionMode>(
    selectedIds.length ? "selected" : "untranslated",
  );
  const [scope, setScope] = useState<"project" | "global" | "both">(
    defaultScope ?? (projectId ? "project" : "global"),
  );
  const [running, setRunning] = useState(false);
  const [progress, setProgress] = useState({ processed: 0, total: 0 });
  const activeJobId = useRef<string | null>(null);
  const ids = useMemo(() => {
    if (selectionMode === "selected") return selectedIds;
    if (selectionMode === "untranslated")
      return entries
        .filter((entry) => !entry.translation)
        .map((entry) => entry.id);
    return entries.map((entry) => entry.id);
  }, [entries, selectedIds, selectionMode]);
  const estimatedTokens = Math.ceil(
    entries
      .filter((entry) => ids.includes(entry.id))
      .reduce((total, entry) => total + entry.canonicalText.length + 80, 0) / 4,
  );

  async function start() {
    if (!ids.length) return;
    setRunning(true);
    let unlisten: Array<() => void> = [];
    unlisten = await Promise.all([
      terminologyApi.onTranslateProgress((event) => {
        if (activeJobId.current && event.jobId !== activeJobId.current) return;
        setProgress({ processed: event.processed, total: event.total });
      }),
      terminologyApi.onTranslateDone((event) => {
        if (activeJobId.current && event.jobId !== activeJobId.current) return;
        unlisten.forEach((fn) => fn());
        activeJobId.current = null;
        if (event.error) toast.error(event.error);
        else
          toast.success(
            t("terminology.translateDialog.done", {
              count: event.summary?.translated ?? 0,
            }),
          );
        setRunning(false);
        onDone();
        onOpenChange(false);
      }),
    ]);
    try {
      const started = await terminologyApi.translate({
        entryIds: ids,
        targetLanguage,
        projectId,
        scope,
        providerConfig,
      });
      activeJobId.current = started.jobId;
    } catch (error) {
      unlisten.forEach((fn) => fn());
      setRunning(false);
      toast.error(String(error));
    }
  }

  return (
    <Dialog open={open} onOpenChange={running ? undefined : onOpenChange}>
      <DialogContent aria-busy={running}>
        <DialogHeader>
          <DialogTitle>{t("terminology.translateTerms")}</DialogTitle>
          <DialogDescription>
            {t("terminology.translateDialog.description", {
              target: targetLanguage.toUpperCase(),
            })}
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-3 sm:grid-cols-2">
          <label className="text-xs text-muted-foreground">
            {t("terminology.translateDialog.selection")}
            <span className="mt-1 block">
              <TerminologySelect
                ariaLabel={t("terminology.translateDialog.selection")}
                value={selectionMode}
                onValueChange={(value) =>
                  setSelectionMode(value as SelectionMode)
                }
                disabled={running}
                options={[
                  {
                    value: "selected",
                    label: t("terminology.translateDialog.selectionSelected", {
                      count: selectedIds.length,
                    }),
                  },
                  {
                    value: "visible",
                    label: t("terminology.translateDialog.selectionVisible", {
                      count: entries.length,
                    }),
                  },
                  {
                    value: "untranslated",
                    label: t(
                      "terminology.translateDialog.selectionUntranslated",
                    ),
                  },
                ]}
              />
            </span>
          </label>
          <label className="text-xs text-muted-foreground">
            {t("terminology.scope")}
            <span className="mt-1 block">
              <TerminologySelect
                ariaLabel={t("terminology.scope")}
                value={scope}
                onValueChange={(value) =>
                  setScope(value as "project" | "global" | "both")
                }
                disabled={running || !projectId}
                options={[
                  {
                    value: "project",
                    label: t("terminology.translateDialog.scopeProject"),
                  },
                  {
                    value: "global",
                    label: t("terminology.translateDialog.scopeGlobal"),
                  },
                  {
                    value: "both",
                    label: t("terminology.translateDialog.scopeBoth"),
                  },
                ]}
              />
            </span>
          </label>
        </div>
        <div className="rounded-xl bg-muted/60 p-3 text-sm">
          <div className="flex justify-between gap-3">
            <span>{t("terminology.translateDialog.termsSent")}</span>
            <strong className="tabular-nums">{ids.length}</strong>
          </div>
          <div className="mt-1 flex justify-between gap-3">
            <span>{t("terminology.translateDialog.promptEstimate")}</span>
            <strong className="tabular-nums">
              {t("terminology.translateDialog.tokens", {
                count: estimatedTokens,
              })}
            </strong>
          </div>
          <div className="mt-1 flex justify-between gap-3">
            <span>{t("terminology.translateDialog.model")}</span>
            <strong className="truncate">{providerConfig.model}</strong>
          </div>
        </div>
        {running && (
          <p className="flex items-center gap-2 text-sm" aria-live="polite">
            <Loader2 className="size-4 animate-spin" />
            <span className="tabular-nums">
              {progress.processed}/{progress.total || ids.length}
            </span>{" "}
            {t("terminology.translateDialog.termsProcessed")}
          </p>
        )}
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={running}
          >
            {t("terminology.cancel")}
          </Button>
          <Button
            onClick={() => void start()}
            disabled={running || !ids.length}
          >
            <Sparkles />{" "}
            {t(
              running
                ? "terminology.translateDialog.translating"
                : "terminology.translateDialog.translate",
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
