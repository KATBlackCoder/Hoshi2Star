import { useMemo, useRef, useState } from "react";
import { Loader2, Sparkles } from "lucide-react";
import { toast } from "sonner";
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

type SelectionMode = "selected" | "visible" | "untranslated";

export function TermTranslateDialog({
  open,
  entries,
  selectedIds,
  targetLanguage,
  projectId,
  providerConfig,
  onOpenChange,
  onDone,
}: {
  open: boolean;
  entries: TerminologyEntry[];
  selectedIds: string[];
  targetLanguage: string;
  projectId: string | null;
  providerConfig: ProviderConfig;
  onOpenChange: (open: boolean) => void;
  onDone: () => void;
}) {
  const [selectionMode, setSelectionMode] = useState<SelectionMode>(
    selectedIds.length ? "selected" : "untranslated",
  );
  const [scope, setScope] = useState<"project" | "global">(
    projectId ? "project" : "global",
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
          toast.success(`${event.summary?.proposed ?? 0} propositions créées`);
        setRunning(false);
        onDone();
        onOpenChange(false);
      }),
    ]);
    try {
      const started = await terminologyApi.translate({
        entryIds: ids,
        targetLanguage,
        projectId: scope === "project" ? projectId : null,
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
          <DialogTitle>Traduire les termes proposés</DialogTitle>
          <DialogDescription>
            Le modèle suggère des termes {targetLanguage.toUpperCase()}. Ils
            resteront « proposés » jusqu’à votre validation explicite.
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-3 sm:grid-cols-2">
          <label className="text-xs text-muted-foreground">
            Sélection
            <select
              className="mt-1 h-10 w-full rounded-lg border bg-background px-2 text-sm"
              value={selectionMode}
              onChange={(event) =>
                setSelectionMode(event.target.value as SelectionMode)
              }
              disabled={running}
            >
              <option value="selected">
                Sélection manuelle ({selectedIds.length})
              </option>
              <option value="visible">Page visible ({entries.length})</option>
              <option value="untranslated">Uniquement sans traduction</option>
            </select>
          </label>
          <label className="text-xs text-muted-foreground">
            Portée
            <select
              className="mt-1 h-10 w-full rounded-lg border bg-background px-2 text-sm"
              value={scope}
              onChange={(event) =>
                setScope(event.target.value as "project" | "global")
              }
              disabled={running || !projectId}
            >
              <option value="project">Projet actif</option>
              <option value="global">Bibliothèque globale</option>
            </select>
          </label>
        </div>
        <div className="rounded-xl bg-muted/60 p-3 text-sm">
          <div className="flex justify-between gap-3">
            <span>Termes envoyés</span>
            <strong className="tabular-nums">{ids.length}</strong>
          </div>
          <div className="mt-1 flex justify-between gap-3">
            <span>Estimation du prompt</span>
            <strong className="tabular-nums">≈ {estimatedTokens} tokens</strong>
          </div>
          <div className="mt-1 flex justify-between gap-3">
            <span>Modèle</span>
            <strong className="truncate">{providerConfig.model}</strong>
          </div>
        </div>
        {running && (
          <p className="flex items-center gap-2 text-sm" aria-live="polite">
            <Loader2 className="size-4 animate-spin" />
            <span className="tabular-nums">
              {progress.processed}/{progress.total || ids.length}
            </span>{" "}
            termes traités
          </p>
        )}
        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => onOpenChange(false)}
            disabled={running}
          >
            Annuler
          </Button>
          <Button
            onClick={() => void start()}
            disabled={running || !ids.length}
          >
            <Sparkles /> {running ? "Traduction…" : "Créer les propositions"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
