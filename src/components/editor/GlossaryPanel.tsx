import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import {
  BookMarked,
  Plus,
  Pencil,
  Trash2,
  Check,
  X,
  Loader2,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { useProviderConfig } from "@/stores/llm";
import { useProjectStore } from "@/stores/project";
import type { GlossaryTerm } from "@/lib/types";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

const DOMAINS = [
  { value: "general", label: "General" },
  { value: "characters", label: "Characters" },
  { value: "combat", label: "Combat" },
  { value: "items", label: "Items" },
  { value: "story", label: "Story" },
  { value: "ui", label: "UI" },
  { value: "system", label: "System" },
] as const;

// ---------------------------------------------------------------------------
// Props
// ---------------------------------------------------------------------------

interface GlossaryPanelProps {
  projectId: string | null;
  langPair: string;
}

// ---------------------------------------------------------------------------
// Inline-edit row
// ---------------------------------------------------------------------------

interface EditRowProps {
  term: GlossaryTerm;
  onSave: (id: string, source: string, target: string, domain: string) => void;
  onCancel: () => void;
}

function EditRow({ term, onSave, onCancel }: EditRowProps) {
  const [source, setSource] = useState(term.sourceText);
  const [target, setTarget] = useState(term.targetText);
  // Normalize: LLM may return singular ("character") or empty string — fall back to "general"
  const [domain, setDomain] = useState(
    DOMAINS.some((d) => d.value === term.domain) ? term.domain : "general",
  );

  return (
    <div className="flex items-center gap-1 py-1">
      <Input
        className="h-6 flex-1 text-xs px-1"
        value={source}
        onChange={(e) => setSource(e.target.value)}
      />
      <Input
        className="h-6 flex-1 text-xs px-1"
        value={target}
        onChange={(e) => setTarget(e.target.value)}
      />
      <Select value={domain} onValueChange={setDomain}>
        <SelectTrigger className="h-6 w-24 text-xs px-1">
          <SelectValue placeholder="Domain" />
        </SelectTrigger>
        <SelectContent>
          {DOMAINS.map((d) => (
            <SelectItem key={d.value} value={d.value} className="text-xs">
              {d.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <button
        type="button"
        className="shrink-0 p-0.5 text-green-500 hover:text-green-400"
        onClick={() => onSave(term.id, source, target, domain)}
        aria-label="Save"
      >
        <Check className="h-3.5 w-3.5" />
      </button>
      <button
        type="button"
        className="shrink-0 p-0.5 text-muted-foreground hover:text-foreground"
        onClick={onCancel}
        aria-label="Cancel"
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Add form
// ---------------------------------------------------------------------------

interface AddFormProps {
  langPair: string;
  projectId: string | null;
  onAdd: (term: GlossaryTerm) => void;
  onCancel: () => void;
}

function AddForm({ langPair, projectId, onAdd, onCancel }: AddFormProps) {
  const { t } = useTranslation();
  const [source, setSource] = useState("");
  const [target, setTarget] = useState("");
  const [domain, setDomain] = useState("general");

  async function handleSubmit() {
    if (!source.trim() || !target.trim()) return;
    try {
      const term = await invoke<GlossaryTerm>("add_glossary_term", {
        sourceText: source.trim(),
        targetText: target.trim(),
        langPair,
        domain: domain.trim(),
        projectId,
      });
      onAdd(term);
    } catch (err) {
      toast.error(String(err));
    }
  }

  return (
    <div className="flex items-center gap-1 py-1 border-b border-border/40 pb-2 mb-1">
      <Input
        className="h-6 flex-1 text-xs px-1"
        placeholder={t("glossaryPanel.source")}
        value={source}
        onChange={(e) => setSource(e.target.value)}
        autoFocus
      />
      <Input
        className="h-6 flex-1 text-xs px-1"
        placeholder={t("glossaryPanel.target")}
        value={target}
        onChange={(e) => setTarget(e.target.value)}
      />
      <Select value={domain} onValueChange={setDomain}>
        <SelectTrigger className="h-6 w-24 text-xs px-1">
          <SelectValue placeholder={t("glossaryPanel.domain")} />
        </SelectTrigger>
        <SelectContent>
          {DOMAINS.map((d) => (
            <SelectItem key={d.value} value={d.value} className="text-xs">
              {d.label}
            </SelectItem>
          ))}
        </SelectContent>
      </Select>
      <button
        type="button"
        className="shrink-0 p-0.5 text-green-500 hover:text-green-400"
        onClick={handleSubmit}
        aria-label="Add"
      >
        <Check className="h-3.5 w-3.5" />
      </button>
      <button
        type="button"
        className="shrink-0 p-0.5 text-muted-foreground hover:text-foreground"
        onClick={onCancel}
        aria-label="Cancel"
      >
        <X className="h-3.5 w-3.5" />
      </button>
    </div>
  );
}

// ---------------------------------------------------------------------------
// GlossaryPanel
// ---------------------------------------------------------------------------

export function GlossaryPanel({ projectId, langPair }: GlossaryPanelProps) {
  const { t } = useTranslation();
  const providerConfig = useProviderConfig();
  const [terms, setTerms] = useState<GlossaryTerm[]>([]);
  const [isExtracting, setIsExtracting] = useState(false);
  const [isExtractingSpeakers, setIsExtractingSpeakers] = useState(false);
  const [showAddForm, setShowAddForm] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const activeProject = useProjectStore((s) =>
    s.projects.find((p) => p.id === s.activeProjectId),
  );
  const isWolf = activeProject?.engine === "wolf";

  const loadTerms = useCallback(async () => {
    if (!projectId) {
      setTerms([]);
      return;
    }
    try {
      const list = await invoke<GlossaryTerm[]>("get_glossary", {
        projectId,
        langPair,
      });
      setTerms(list);
    } catch (err) {
      console.error("get_glossary error:", err);
    }
  }, [projectId, langPair]);

  useEffect(() => {
    loadTerms();
  }, [loadTerms]);

  // Listen for extraction-done events
  useEffect(() => {
    const unlisten = listen<{
      projectId: string;
      terms: GlossaryTerm[];
      error: string | null;
    }>("h2s://glossary/extraction-done", (event) => {
      if (event.payload.projectId !== projectId) return;
      setIsExtracting(false);

      if (event.payload.error) {
        toast.error(
          `${t("glossaryPanel.extractError")}: ${event.payload.error}`,
          { duration: 8000 },
        );
        return;
      }

      const newTerms = event.payload.terms;
      setTerms((prev) => {
        const existingIds = new Set(prev.map((t) => t.id));
        return [...prev, ...newTerms.filter((t) => !existingIds.has(t.id))];
      });

      if (newTerms.length === 0) {
        toast.info(t("glossaryPanel.extractNoTerms"), { duration: 5000 });
      } else {
        toast.success(
          `${newTerms.length} ${t("glossaryPanel.title").toLowerCase()} ${t("glossaryPanel.extractDone")}`,
        );
      }
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, [projectId, t]);

  async function handleExtract() {
    if (!projectId || isExtracting) return;
    setIsExtracting(true);
    try {
      await invoke("extract_glossary_terms", {
        projectId,
        langPair,
        providerConfig,
      });
    } catch (err) {
      setIsExtracting(false);
      toast.error(String(err));
    }
  }

  async function handleExtractSpeakers() {
    if (!projectId || isExtractingSpeakers) return;
    setIsExtractingSpeakers(true);
    try {
      const newTerms = await invoke<GlossaryTerm[]>("extract_wolf_speakers", {
        projectId,
        langPair,
      });
      if (newTerms.length === 0) {
        toast.info(t("glossaryPanel.extractSpeakersNone"));
      } else {
        setTerms((prev) => {
          const existingIds = new Set(prev.map((t) => t.id));
          return [...prev, ...newTerms.filter((t) => !existingIds.has(t.id))];
        });
        toast.success(
          `${newTerms.length} ${t("glossaryPanel.extractSpeakersDone")}`,
        );
      }
    } catch (err) {
      toast.error(String(err));
    } finally {
      setIsExtractingSpeakers(false);
    }
  }

  async function handleSaveEdit(
    id: string,
    source: string,
    target: string,
    domain: string,
  ) {
    try {
      const updated = await invoke<GlossaryTerm>("update_glossary_term", {
        id,
        sourceText: source,
        targetText: target,
        domain,
      });
      setTerms((prev) => prev.map((t) => (t.id === id ? updated : t)));
      setEditingId(null);
    } catch (err) {
      toast.error(String(err));
    }
  }

  async function handleDelete(id: string) {
    try {
      await invoke("delete_glossary_term", { id });
      setTerms((prev) => prev.filter((t) => t.id !== id));
    } catch (err) {
      toast.error(String(err));
    }
  }

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {/* Header */}
      <div className="shrink-0 border-b px-3 py-2 text-[10px] font-semibold uppercase tracking-[0.12em] text-muted-foreground/80 select-none flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <BookMarked className="h-3 w-3" />
          <span>{t("glossaryPanel.title")}</span>
          {terms.length > 0 && (
            <span className="text-[10px] text-muted-foreground/60">
              ({terms.length})
            </span>
          )}
        </div>
        <div className="flex items-center gap-1">
          <Button
            variant="ghost"
            size="sm"
            className="h-5 px-1.5 text-[10px]"
            onClick={() => setShowAddForm((v) => !v)}
            disabled={!projectId}
            title={t("glossaryPanel.addTerm")}
          >
            <Plus className="h-3 w-3" />
          </Button>
          <Button
            variant="ghost"
            size="sm"
            className="h-5 px-1.5 text-[10px]"
            onClick={handleExtract}
            disabled={!projectId || isExtracting}
            title={t("glossaryPanel.extract")}
          >
            {isExtracting ? (
              <>
                <Loader2 className="h-3 w-3 animate-spin mr-1" />
                {t("glossaryPanel.extracting")}
              </>
            ) : (
              t("glossaryPanel.extract")
            )}
          </Button>
          {isWolf && (
            <Button
              variant="ghost"
              size="sm"
              className="h-5 px-1.5 text-[10px]"
              onClick={() => void handleExtractSpeakers()}
              disabled={!projectId || isExtractingSpeakers}
              title={t("glossaryPanel.extractSpeakers")}
            >
              {isExtractingSpeakers ? (
                <Loader2 className="h-3 w-3 animate-spin" />
              ) : (
                t("glossaryPanel.extractSpeakers")
              )}
            </Button>
          )}
        </div>
      </div>

      {/* Body */}
      <div className="flex-1 overflow-y-auto p-2">
        {showAddForm && (
          <AddForm
            langPair={langPair}
            projectId={projectId}
            onAdd={(term) => {
              setTerms((prev) => [...prev, term]);
              setShowAddForm(false);
            }}
            onCancel={() => setShowAddForm(false)}
          />
        )}

        {terms.length === 0 && !showAddForm && (
          <p className="py-4 text-center text-xs text-muted-foreground leading-relaxed">
            {t("glossaryPanel.noTerms")}
          </p>
        )}

        <div className="space-y-0.5">
          {terms.map((term) =>
            editingId === term.id ? (
              <EditRow
                key={term.id}
                term={term}
                onSave={handleSaveEdit}
                onCancel={() => setEditingId(null)}
              />
            ) : (
              <div
                key={term.id}
                className="flex items-center gap-1.5 rounded px-1 py-0.5 text-xs hover:bg-muted/30 group"
              >
                {/* Source */}
                <span className="flex-1 truncate font-medium">
                  {term.sourceText}
                </span>
                {/* Arrow */}
                <span className="text-muted-foreground/50">→</span>
                {/* Target */}
                <span className="flex-1 truncate">{term.targetText}</span>
                {/* Badges */}
                <div className="flex shrink-0 items-center gap-0.5">
                  {term.autoGenerated && (
                    <Badge
                      variant="secondary"
                      className="h-3.5 px-1 text-[9px]"
                    >
                      {t("glossaryPanel.auto")}
                    </Badge>
                  )}
                  <Badge
                    variant={term.projectId ? "default" : "outline"}
                    className="h-3.5 px-1 text-[9px]"
                  >
                    {term.projectId
                      ? t("glossaryPanel.project")
                      : t("glossaryPanel.global")}
                  </Badge>
                </div>
                {/* Actions */}
                <div className="flex shrink-0 items-center gap-0.5 opacity-0 group-hover:opacity-100 transition-opacity">
                  <button
                    type="button"
                    className="p-0.5 text-muted-foreground hover:text-foreground"
                    onClick={() => setEditingId(term.id)}
                    aria-label="Edit"
                  >
                    <Pencil className="h-3 w-3" />
                  </button>
                  <AlertDialog>
                    <AlertDialogTrigger asChild>
                      <button
                        type="button"
                        className="p-0.5 text-muted-foreground hover:text-destructive"
                        aria-label="Delete"
                      >
                        <Trash2 className="h-3 w-3" />
                      </button>
                    </AlertDialogTrigger>
                    <AlertDialogContent>
                      <AlertDialogHeader>
                        <AlertDialogTitle>
                          {t("glossaryPanel.deleteConfirm")}
                        </AlertDialogTitle>
                        <AlertDialogDescription>
                          {term.sourceText} → {term.targetText}
                        </AlertDialogDescription>
                      </AlertDialogHeader>
                      <AlertDialogFooter>
                        <AlertDialogCancel>Cancel</AlertDialogCancel>
                        <AlertDialogAction
                          onClick={() => handleDelete(term.id)}
                        >
                          Delete
                        </AlertDialogAction>
                      </AlertDialogFooter>
                    </AlertDialogContent>
                  </AlertDialog>
                </div>
              </div>
            ),
          )}
        </div>
      </div>
    </div>
  );
}
