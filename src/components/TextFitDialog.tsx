import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { toast } from "sonner";
import { AlertTriangle, Info, Loader2 } from "lucide-react";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Checkbox } from "@/components/ui/checkbox";
import { Input } from "@/components/ui/input";
import { outcomeLabel } from "@/lib/textFit";
import type {
  FitCounts,
  FitOverflow,
  LineBreaks,
  TextFitSettings,
} from "@/lib/types";
import { useEditorStore } from "@/stores/editor";
import { rescanTextFit, saveTextFit, useTextFitStore } from "@/stores/textFit";
import { useUiStore } from "@/stores/ui";

interface TextFitDialogProps {
  projectId: string | null;
}

const ROWS_SHOWN = 40;
const FONT_MIN = 8;
const FONT_MAX = 96;

function validSize(value: number | null): boolean {
  return (
    value === null ||
    (Number.isInteger(value) && value >= FONT_MIN && value <= FONT_MAX)
  );
}

/** Wolf RPG overrides: `null`/absent = measured (or unknown font size). */
function validWolf(settings: TextFitSettings): boolean {
  const { wolfWidthEm: width, wolfMaxLines: lines } = settings;
  const base = settings.wolfBaseFontSize;
  return (
    (width == null || (width >= 4 && width <= 200)) &&
    (lines == null || (Number.isInteger(lines) && lines >= 1 && lines <= 20)) &&
    (base == null || (Number.isInteger(base) && base >= 4 && base <= 96))
  );
}

/** Empty input → `null` (measured value), a number otherwise. */
function optionalNumber(value: string): number | null {
  return value === "" ? null : Number(value);
}

/**
 * Text fitting settings (RPG Maker MZ; Wolf RPG messages): what is laid out
 * at export, how, and what that does to this game — with live counts for the
 * draft settings.
 */
export function TextFitDialog({ projectId }: TextFitDialogProps) {
  const { t } = useTranslation();
  const { open, loading, scanning, view, scan, error, setOpen } =
    useTextFitStore();
  const [draft, setDraft] = useState<TextFitSettings | null>(null);
  const [saving, setSaving] = useState(false);
  const scannedFor = useRef<string | null>(null);
  const setActiveFile = useEditorStore((s) => s.setActiveFile);
  const setActiveSegment = useEditorStore((s) => s.setActiveSegment);
  const setInspectorTab = useUiStore((s) => s.setInspectorTab);
  const inspectorOpen = useUiStore((s) => s.inspectorOpen);
  const toggleInspector = useUiStore((s) => s.toggleInspector);

  // The dialog stays mounted: re-seed the draft every time it opens.
  useEffect(() => {
    if (!open || !view) return;
    setDraft(view.settings);
    scannedFor.current = JSON.stringify(view.settings);
  }, [open, view]);

  // Live counts for the draft (debounced).
  const draftKey = draft ? JSON.stringify(draft) : null;
  useEffect(() => {
    if (!open || !projectId || !draft || !draftKey || !view?.supported) return;
    if (draftKey === scannedFor.current) return;
    if (
      !validSize(draft.minFontSize) ||
      !validSize(draft.dialogueFontSize) ||
      !validWolf(draft)
    )
      return;
    const handle = window.setTimeout(() => {
      scannedFor.current = draftKey;
      void rescanTextFit(projectId, draft);
    }, 400);
    return () => window.clearTimeout(handle);
  }, [draftKey, draft, open, projectId, view?.supported]);

  const update = (patch: Partial<TextFitSettings>) =>
    setDraft((current) => (current ? { ...current, ...patch } : current));

  const valid =
    draft !== null &&
    validSize(draft.minFontSize) &&
    validSize(draft.dialogueFontSize) &&
    validWolf(draft);

  const save = async () => {
    if (!projectId || !draft || !valid) return;
    setSaving(true);
    try {
      await saveTextFit(projectId, draft);
      toast.success(t("textFit.saved"));
      setOpen(false);
    } catch (err) {
      toast.error(String(err));
    } finally {
      setSaving(false);
    }
  };

  const counts = (key: string, value: FitCounts) => t(key, { ...value });
  const geometry = view?.geometry ?? null;
  const wolf = view?.wolf ?? null;
  /** Measured Wolf line width, rounded for display. */
  const measuredWidth = wolf?.measured
    ? Number(wolf.measured.widthEm.toFixed(1))
    : null;
  /** Wolf notes follow the draft (the game's font size may be filled in). */
  const notes = wolf
    ? [
        ...(draft?.wolfBaseFontSize == null
          ? [t("textFit.wolf.noBaseFontSize")]
          : []),
        t("textFit.wolf.choicesNote"),
      ]
    : (view?.notes ?? []);

  const openRow = (fileId: string, segmentId: string | null) => {
    setActiveFile(fileId);
    if (segmentId) setActiveSegment(segmentId);
    // The in-game preview lays out MZ texts only.
    if (!wolf) {
      setInspectorTab("preview");
      if (!inspectorOpen) toggleInspector();
    }
    setOpen(false);
  };

  return (
    <Dialog open={open} onOpenChange={setOpen}>
      <DialogContent className="max-h-[90vh] w-[min(94vw,46rem)] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>{t("textFit.title")}</DialogTitle>
          <DialogDescription>{t("textFit.description")}</DialogDescription>
        </DialogHeader>

        {loading ? (
          <p className="flex items-center gap-2 py-6 text-sm text-muted-foreground">
            <Loader2 className="h-4 w-4 animate-spin" />
            {t("textFit.loading")}
          </p>
        ) : !view ? (
          error && <p className="py-4 text-sm text-destructive">{error}</p>
        ) : !view.supported ? (
          <p className="py-4 text-sm text-muted-foreground">{view.reason}</p>
        ) : (
          draft && (
            <div className="space-y-5 text-sm">
              <section className="space-y-3">
                <label className="flex items-start gap-3">
                  <Checkbox
                    className="mt-0.5"
                    checked={draft.enabledDialogue}
                    onCheckedChange={(value) =>
                      update({ enabledDialogue: value === true })
                    }
                  />
                  <span>
                    <span className="font-medium">
                      {t(
                        wolf
                          ? "textFit.wolf.enableDialogue"
                          : "textFit.enableDialogue",
                      )}
                    </span>
                    <span className="block text-xs text-muted-foreground">
                      {t(
                        wolf
                          ? "textFit.wolf.enableDialogueHint"
                          : "textFit.enableDialogueHint",
                      )}
                    </span>
                  </span>
                </label>
                {!wolf && (
                  <label className="flex items-start gap-3">
                    <Checkbox
                      className="mt-0.5"
                      checked={draft.enabledDescriptions}
                      onCheckedChange={(value) =>
                        update({ enabledDescriptions: value === true })
                      }
                    />
                    <span>
                      <span className="font-medium">
                        {t("textFit.enableDescriptions")}
                      </span>
                      <span className="block text-xs text-muted-foreground">
                        {t("textFit.enableDescriptionsHint")}
                      </span>
                    </span>
                  </label>
                )}
              </section>

              <section className="grid gap-3 sm:grid-cols-2">
                <label className="space-y-1">
                  <span className="text-xs font-medium">
                    {t("textFit.lineBreaks")}
                  </span>
                  <select
                    className="h-9 w-full rounded-lg border bg-background px-2 text-foreground dark:[color-scheme:dark]"
                    value={draft.lineBreaks}
                    onChange={(e) =>
                      update({ lineBreaks: e.target.value as LineBreaks })
                    }
                  >
                    <option value="reflow">
                      {t("textFit.lineBreaksReflow")}
                    </option>
                    <option value="keep">{t("textFit.lineBreaksKeep")}</option>
                  </select>
                </label>
                <label className="space-y-1">
                  <span className="text-xs font-medium">
                    {t("textFit.overflow")}
                  </span>
                  <select
                    className="h-9 w-full rounded-lg border bg-background px-2 text-foreground dark:[color-scheme:dark]"
                    value={draft.overflow}
                    onChange={(e) =>
                      update({ overflow: e.target.value as FitOverflow })
                    }
                  >
                    {!wolf && (
                      <option value="new_page">
                        {t("textFit.overflowNewPage")}
                      </option>
                    )}
                    <option value="shrink">
                      {t(
                        wolf
                          ? "textFit.wolf.overflowShrink"
                          : "textFit.overflowShrink",
                      )}
                    </option>
                    <option value="leave">{t("textFit.overflowLeave")}</option>
                  </select>
                </label>
                <label className="space-y-1">
                  <span className="text-xs font-medium">
                    {t("textFit.minFontSize")}
                  </span>
                  <Input
                    type="number"
                    min={FONT_MIN}
                    max={FONT_MAX}
                    value={draft.minFontSize}
                    aria-invalid={!validSize(draft.minFontSize)}
                    onChange={(e) =>
                      update({ minFontSize: Number(e.target.value) })
                    }
                  />
                </label>
                {wolf ? (
                  <>
                    <label className="space-y-1">
                      <span className="text-xs font-medium">
                        {t("textFit.wolf.baseFontSize")}
                      </span>
                      <Input
                        type="number"
                        min={4}
                        max={96}
                        value={draft.wolfBaseFontSize ?? ""}
                        aria-invalid={!validWolf(draft)}
                        onChange={(e) =>
                          update({
                            wolfBaseFontSize: optionalNumber(e.target.value),
                          })
                        }
                      />
                      <span className="block text-xs text-muted-foreground">
                        {t("textFit.wolf.baseFontSizeHint")}
                      </span>
                    </label>
                    <label className="space-y-1">
                      <span className="text-xs font-medium">
                        {t("textFit.wolf.widthEm")}
                      </span>
                      <Input
                        type="number"
                        min={4}
                        max={200}
                        step={0.5}
                        placeholder={
                          measuredWidth === null ? "" : String(measuredWidth)
                        }
                        value={draft.wolfWidthEm ?? ""}
                        onChange={(e) =>
                          update({
                            wolfWidthEm: optionalNumber(e.target.value),
                          })
                        }
                      />
                    </label>
                    <label className="space-y-1">
                      <span className="text-xs font-medium">
                        {t("textFit.wolf.maxLines")}
                      </span>
                      <Input
                        type="number"
                        min={1}
                        max={20}
                        placeholder={
                          wolf.measured ? String(wolf.measured.maxLines) : ""
                        }
                        value={draft.wolfMaxLines ?? ""}
                        onChange={(e) =>
                          update({
                            wolfMaxLines: optionalNumber(e.target.value),
                          })
                        }
                      />
                      <span className="block text-xs text-muted-foreground">
                        {t("textFit.wolf.measuredHint")}
                      </span>
                    </label>
                  </>
                ) : (
                  <label className="space-y-1">
                    <span className="text-xs font-medium">
                      {t("textFit.dialogueFontSize")}
                    </span>
                    <Input
                      type="number"
                      min={FONT_MIN}
                      max={FONT_MAX}
                      placeholder={String(geometry?.fontSize ?? "")}
                      value={draft.dialogueFontSize ?? ""}
                      aria-invalid={!validSize(draft.dialogueFontSize)}
                      onChange={(e) =>
                        update({
                          dialogueFontSize:
                            e.target.value === ""
                              ? null
                              : Number(e.target.value),
                        })
                      }
                    />
                    <span className="block text-xs text-muted-foreground">
                      {t("textFit.dialogueFontSizeHint", {
                        size: geometry?.fontSize ?? 26,
                      })}
                    </span>
                  </label>
                )}
              </section>

              {wolf && (
                <section className="space-y-1 rounded-lg border p-3 text-xs">
                  <p className="font-medium">{t("textFit.detected")}</p>
                  <p>
                    {wolf.measured
                      ? t("textFit.wolf.window", {
                          width: measuredWidth,
                          lines: wolf.measured.maxLines,
                        })
                      : t("textFit.wolf.notMeasured")}
                  </p>
                  <p>
                    {view.font?.loaded
                      ? t("textFit.wolf.fontMeasured", {
                          file: view.font.file,
                        })
                      : t("textFit.fontEstimated", {
                          error: view.font?.error ?? "",
                        })}
                  </p>
                </section>
              )}

              {geometry && (
                <section className="space-y-1 rounded-lg border p-3 text-xs">
                  <p className="font-medium">{t("textFit.detected")}</p>
                  <p>
                    {t("textFit.box", {
                      width: geometry.message.innerWidth - 8,
                      face: geometry.message.innerWidth - 168,
                      lines: 4,
                    })}
                  </p>
                  <p>{t("textFit.help", { width: geometry.help.textWidth })}</p>
                  <p>
                    {view.font?.loaded
                      ? t("textFit.fontMeasured", {
                          file: view.font.file,
                          size: geometry.fontSize,
                        })
                      : t("textFit.fontEstimated", {
                          error: view.font?.error ?? "",
                        })}
                  </p>
                  <p className="flex items-start gap-1.5 text-muted-foreground">
                    <Info className="mt-0.5 h-3.5 w-3.5 shrink-0" />
                    {t("textFit.shrinkNote")}
                  </p>
                </section>
              )}

              {(view.warnings.length > 0 ||
                notes.length > 0 ||
                (scan?.missingChars.length ?? 0) > 0) && (
                <section className="space-y-2 text-xs">
                  {view.warnings.length > 0 && (
                    <div className="rounded-lg border border-amber-500/40 p-3">
                      <p className="flex items-center gap-1.5 font-medium">
                        <AlertTriangle className="h-3.5 w-3.5 text-amber-500" />
                        {t("textFit.warnings")}
                      </p>
                      <ul className="mt-1 list-disc space-y-0.5 pl-5">
                        {view.warnings.map((warning) => (
                          <li key={warning.plugin}>
                            {warning.plugin} —{" "}
                            {t("textFit.warningHooks", {
                              hooks: warning.hooks.join(", "),
                            })}
                          </li>
                        ))}
                      </ul>
                    </div>
                  )}
                  {notes.length > 0 && (
                    <div className="rounded-lg border p-3">
                      <p className="font-medium">
                        {t(wolf ? "textFit.wolf.notes" : "textFit.notes")}
                      </p>
                      <ul className="mt-1 list-disc space-y-0.5 pl-5">
                        {notes.map((note) => (
                          <li key={note}>{note}</li>
                        ))}
                      </ul>
                    </div>
                  )}
                  {scan && scan.missingChars.length > 0 && (
                    <p className="rounded-lg border p-3">
                      {t("textFit.missing", {
                        chars: scan.missingChars
                          .map((m) => `${m.character} (${m.count})`)
                          .join(", "),
                      })}
                    </p>
                  )}
                </section>
              )}

              <section className="space-y-2">
                <p className="flex items-center gap-2 text-xs font-medium">
                  {t("textFit.summary")}
                  {scanning && (
                    <Loader2 className="h-3.5 w-3.5 animate-spin text-muted-foreground" />
                  )}
                </p>
                {scan && (
                  <>
                    <p className="text-xs">
                      {counts(
                        wolf
                          ? "textFit.wolf.dialogueCounts"
                          : "textFit.dialogueCounts",
                        scan.dialogue,
                      )}
                    </p>
                    {!wolf && (
                      <p className="text-xs">
                        {counts("textFit.descriptionCounts", scan.descriptions)}
                      </p>
                    )}
                    {scan.rows.length > 0 && (
                      <div className="rounded-lg border">
                        <p className="border-b px-3 py-1.5 text-xs font-medium">
                          {t("textFit.review")}
                        </p>
                        <ul className="max-h-56 divide-y overflow-y-auto">
                          {scan.rows.slice(0, ROWS_SHOWN).map((row) => (
                            <li
                              key={`${row.fileName}${row.pointer}`}
                              className="flex items-center gap-2 px-3 py-1.5 text-xs"
                            >
                              <span className="min-w-0 flex-1">
                                <span className="block truncate">
                                  {row.sample}
                                </span>
                                <span className="block text-muted-foreground">
                                  {row.fileName} ·{" "}
                                  {outcomeLabel(t, row.outcome)}
                                </span>
                              </span>
                              <Button
                                size="sm"
                                variant="outline"
                                className="h-7 text-xs"
                                onClick={() =>
                                  openRow(row.fileId, row.segmentId)
                                }
                              >
                                {t("textFit.open")}
                              </Button>
                            </li>
                          ))}
                        </ul>
                      </div>
                    )}
                  </>
                )}
              </section>
            </div>
          )
        )}

        <DialogFooter>
          <Button variant="outline" onClick={() => setOpen(false)}>
            {t("textFit.cancel")}
          </Button>
          <Button
            onClick={() => void save()}
            disabled={!view?.supported || !valid || saving}
          >
            {saving && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
            {t("textFit.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
