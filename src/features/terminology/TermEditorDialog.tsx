import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import type {
  PartOfSpeech,
  TerminologyEnforcement,
  TerminologyEntry,
  TerminologyReviewStatus,
} from "@/lib/types";
import { useTranslation } from "react-i18next";

export interface TermEditorValue {
  canonicalText: string;
  reading: string | null;
  partOfSpeech: PartOfSpeech;
  semanticType: string;
  targetText: string;
  reviewStatus: TerminologyReviewStatus;
  enforcement: TerminologyEnforcement;
}

export function TermEditorDialog({
  open,
  entry,
  targetLanguage,
  onOpenChange,
  onSave,
  saving,
}: {
  open: boolean;
  entry: TerminologyEntry | null;
  targetLanguage: string;
  onOpenChange: (open: boolean) => void;
  onSave: (value: TermEditorValue) => void;
  saving: boolean;
}) {
  const { t } = useTranslation();
  const [canonicalText, setCanonicalText] = useState(
    entry?.canonicalText ?? "",
  );
  const [reading, setReading] = useState(entry?.reading ?? "");
  const [partOfSpeech, setPartOfSpeech] = useState<PartOfSpeech>(
    entry?.partOfSpeech ?? "noun",
  );
  const [semanticType, setSemanticType] = useState(
    entry?.semanticType ?? "general",
  );
  const [targetText, setTargetText] = useState(
    entry?.translation?.targetText ?? "",
  );
  const [reviewStatus, setReviewStatus] = useState<TerminologyReviewStatus>(
    entry?.translation?.reviewStatus ?? "approved",
  );
  const [enforcement, setEnforcement] = useState<TerminologyEnforcement>(
    entry?.translation?.enforcement ?? "preferred",
  );
  const valid =
    canonicalText.trim() &&
    semanticType.trim() &&
    (!entry || targetText.trim());
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent aria-busy={saving}>
        <DialogHeader>
          <DialogTitle>
            {t(entry ? "terminology.editorEdit" : "terminology.editorCreate")}
          </DialogTitle>
          <DialogDescription>
            {t(
              entry
                ? "terminology.editorEditDescription"
                : "terminology.editorCreateDescription",
              { target: targetLanguage.toUpperCase() },
            )}
          </DialogDescription>
        </DialogHeader>
        <div className="grid gap-3 sm:grid-cols-2">
          <Field label={t("terminology.sourceTerm")}>
            <Input
              autoFocus
              value={canonicalText}
              onChange={(e) => setCanonicalText(e.target.value)}
            />
          </Field>
          <Field label={t("terminology.reading")}>
            <Input
              value={reading}
              onChange={(e) => setReading(e.target.value)}
            />
          </Field>
          <Field label={t("terminology.partOfSpeech")}>
            <select
              className="h-10 w-full rounded-lg border bg-background px-2"
              value={partOfSpeech}
              onChange={(e) => setPartOfSpeech(e.target.value as PartOfSpeech)}
            >
              {[
                "noun",
                "proper_noun",
                "verb",
                "adjective",
                "adverb",
                "expression",
                "unknown",
              ].map((v) => (
                <option key={v}>{v}</option>
              ))}
            </select>
          </Field>
          <Field label={t("terminology.semanticType")}>
            <Input
              value={semanticType}
              onChange={(e) => setSemanticType(e.target.value)}
            />
          </Field>
          {entry && (
            <>
              <Field
                label={t("terminology.translation", {
                  target: targetLanguage.toUpperCase(),
                })}
                wide
              >
                <Input
                  value={targetText}
                  onChange={(e) => setTargetText(e.target.value)}
                />
              </Field>
              <Field label={t("terminology.review")}>
                <select
                  className="h-10 w-full rounded-lg border bg-background px-2"
                  value={reviewStatus}
                  onChange={(e) =>
                    setReviewStatus(e.target.value as TerminologyReviewStatus)
                  }
                >
                  {["proposed", "approved", "locked"].map((v) => (
                    <option key={v}>{v}</option>
                  ))}
                </select>
              </Field>
              <Field label={t("terminology.enforcement")}>
                <select
                  className="h-10 w-full rounded-lg border bg-background px-2"
                  value={enforcement}
                  onChange={(e) =>
                    setEnforcement(e.target.value as TerminologyEnforcement)
                  }
                >
                  {["contextual", "preferred", "required"].map((v) => (
                    <option key={v}>{v}</option>
                  ))}
                </select>
              </Field>
            </>
          )}
        </div>
        <DialogFooter>
          <Button variant="outline" onClick={() => onOpenChange(false)}>
            {t("terminology.cancel")}
          </Button>
          <Button
            disabled={!valid || saving}
            onClick={() =>
              onSave({
                canonicalText,
                reading: reading.trim() || null,
                partOfSpeech,
                semanticType,
                targetText,
                reviewStatus,
                enforcement,
              })
            }
          >
            {t(saving ? "terminology.saving" : "terminology.save")}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
function Field({
  label,
  children,
  wide,
}: {
  label: string;
  children: React.ReactNode;
  wide?: boolean;
}) {
  return (
    <label
      className={`text-xs text-muted-foreground ${wide ? "sm:col-span-2" : ""}`}
    >
      {label}
      <span className="mt-1 block text-foreground">{children}</span>
    </label>
  );
}
