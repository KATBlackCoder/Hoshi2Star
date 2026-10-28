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
} from "@/lib/types";
import { useTranslation } from "react-i18next";
import { TerminologySelect } from "./TerminologySelect";
import {
  includeCurrentOption,
  PART_OF_SPEECH_VALUES,
  SEMANTIC_TYPE_VALUES,
} from "./terminologyOptions";

export interface TermEditorValue {
  canonicalText: string;
  reading: string | null;
  partOfSpeech: PartOfSpeech;
  semanticType: string;
  targetText: string;
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
  const [enforcement, setEnforcement] = useState<TerminologyEnforcement>(
    entry?.translation?.enforcement ?? "preferred",
  );
  const semanticTypeOptions = includeCurrentOption(
    SEMANTIC_TYPE_VALUES,
    semanticType,
  );
  const valid = canonicalText.trim() && semanticType.trim();
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
            <TerminologySelect
              ariaLabel={t("terminology.partOfSpeech")}
              value={partOfSpeech}
              onValueChange={(value) => setPartOfSpeech(value as PartOfSpeech)}
              options={PART_OF_SPEECH_VALUES.map((value) => ({
                value,
                label: t(`terminology.partOfSpeechTypes.${value}`),
              }))}
            />
          </Field>
          <Field label={t("terminology.semanticType")}>
            <TerminologySelect
              ariaLabel={t("terminology.semanticType")}
              value={semanticType}
              onValueChange={setSemanticType}
              options={semanticTypeOptions.map((value) => ({
                value,
                label:
                  value in SEMANTIC_TYPE_LABELS
                    ? t(`terminology.semanticTypes.${value}`)
                    : value,
              }))}
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
              <Field label={t("terminology.enforcement")}>
                <TerminologySelect
                  ariaLabel={t("terminology.enforcement")}
                  value={enforcement}
                  onValueChange={(value) =>
                    setEnforcement(value as TerminologyEnforcement)
                  }
                  options={["contextual", "preferred", "required"].map(
                    (value) => ({
                      value,
                      label: t(`terminology.enforcementStatus.${value}`),
                    }),
                  )}
                />
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

const SEMANTIC_TYPE_LABELS = Object.fromEntries(
  SEMANTIC_TYPE_VALUES.map((value) => [value, true]),
);

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
