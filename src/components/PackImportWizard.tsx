import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "sonner";
import { AlertTriangle, Loader2 } from "lucide-react";
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
import { Checkbox } from "@/components/ui/checkbox";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { refreshProjectData } from "@/stores/project";
import type {
  ImportBlocker,
  ImportPolicy,
  ImportPreview,
  ImportReport,
} from "@/lib/types";

interface PackImportWizardProps {
  projectId: string;
  packPath: string;
  langPair?: string;
  /** Dry-run result — already fetched by the caller before opening. */
  preview: ImportPreview;
  onClose: () => void;
}

function blockerMessage(
  blocker: ImportBlocker,
  t: (key: string, opts?: Record<string, unknown>) => string,
): string {
  switch (blocker.type) {
    case "notAPack":
      return t("pack.blockerNotAPack");
    case "unsupportedVersion":
      return t("pack.blockerUnsupportedVersion", { version: blocker.version });
    case "engineMismatch":
      return t("pack.blockerEngineMismatch", {
        packEngine: blocker.packEngine,
        projectEngine: blocker.projectEngine,
      });
    case "langPairMismatch":
      return t("pack.blockerLangPairMismatch", {
        packLangPair: blocker.packLangPair,
        projectLangPair: blocker.projectLangPair,
      });
    case "invalidPack":
      return t("pack.blockerInvalidPack", { detail: blocker.detail });
  }
}

/** One "label: count" line of the dry-run / final report. */
function CountLine({ label, value }: { label: string; value: number }) {
  return (
    <div className="flex items-baseline justify-between gap-3">
      <span className="text-muted-foreground">{label}</span>
      <span className="font-mono tabular-nums">{value}</span>
    </div>
  );
}

/**
 * Import wizard for a `.h2s` exchange pack. The caller runs the mandatory
 * dry-run (`preview_h2s_import`) first; this dialog shows the classification,
 * lets the user pick the collision policy (default = safest), then applies
 * in one backend transaction (with automatic backup) and shows the report.
 */
export function PackImportWizard({
  projectId,
  packPath,
  preview,
  langPair = "ja-en",
  onClose,
}: PackImportWizardProps) {
  const { t } = useTranslation();
  const [policy, setPolicy] = useState<ImportPolicy>("fill");
  const [applySourceChanged, setApplySourceChanged] = useState(false);
  const [importGlossary, setImportGlossary] = useState(true);
  const [importTm, setImportTm] = useState(false);
  const [confirmOverwriteAll, setConfirmOverwriteAll] = useState(false);
  const [isApplying, setIsApplying] = useState(false);
  const [report, setReport] = useState<ImportReport | null>(null);

  const blocked = preview.blocker !== null;
  const canApply =
    !isApplying && (policy !== "overwrite_all" || confirmOverwriteAll);

  async function handleApply() {
    setIsApplying(true);
    try {
      const result = await invoke<ImportReport>("apply_h2s_import", {
        projectId,
        packPath,
        policy,
        applySourceChanged,
        importGlossary,
        importTm,
        langPair,
      });
      setReport(result);
      void refreshProjectData(projectId);
    } catch (e) {
      toast.error(t("pack.importError", { error: String(e) }));
    } finally {
      setIsApplying(false);
    }
  }

  // ---- Step 3: final report -----------------------------------------------
  if (report) {
    return (
      <AlertDialog open>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("pack.importDoneTitle")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("pack.importDoneDesc")}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <div className="grid gap-1 py-1 text-xs">
            <CountLine label={t("pack.reportApplied")} value={report.applied} />
            {report.appliedSourceChanged > 0 && (
              <CountLine
                label={t("pack.reportAppliedSourceChanged")}
                value={report.appliedSourceChanged}
              />
            )}
            <CountLine
              label={t("pack.reportIdentical")}
              value={report.identical}
            />
            <CountLine
              label={t("pack.reportSkippedConflicts")}
              value={report.skippedConflicts}
            />
            <CountLine
              label={t("pack.reportSkippedSourceChanged")}
              value={report.skippedSourceChanged}
            />
            <CountLine label={t("pack.reportOrphans")} value={report.orphans} />
            {(report.glossaryAdded > 0 || report.glossaryConflicts > 0) && (
              <CountLine
                label={t("pack.reportGlossary")}
                value={report.glossaryAdded}
              />
            )}
            {report.glossaryConflicts > 0 && (
              <CountLine
                label={t("pack.reportGlossaryConflicts")}
                value={report.glossaryConflicts}
              />
            )}
            {report.tmAdded > 0 && (
              <CountLine label={t("pack.reportTm")} value={report.tmAdded} />
            )}
            <p className="mt-2 break-all text-[10px] text-muted-foreground">
              {t("pack.reportBackup", { path: report.backupPath })}
            </p>
          </div>
          <AlertDialogFooter>
            <AlertDialogAction onClick={onClose}>
              {t("pack.close")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    );
  }

  // ---- Blocked: pack refused ----------------------------------------------
  if (blocked) {
    return (
      <AlertDialog open>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("pack.importBlockedTitle")}</AlertDialogTitle>
            <AlertDialogDescription>
              {blockerMessage(preview.blocker as ImportBlocker, t)}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogAction onClick={onClose}>
              {t("pack.close")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    );
  }

  // ---- Step 2: dry-run report + policy choice ------------------------------
  return (
    <AlertDialog open>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("pack.importTitle")}</AlertDialogTitle>
          <AlertDialogDescription>
            {t("pack.importDesc", {
              title: preview.packGameTitle,
              version: preview.packAppVersion,
              date: preview.packCreatedAt,
            })}
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="grid gap-1 py-1 text-xs">
          {preview.titleMismatch && (
            <p className="mb-1 flex items-start gap-1.5 text-amber-400">
              <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0" />
              {t("pack.titleMismatch", { title: preview.packGameTitle })}
            </p>
          )}
          <CountLine
            label={t("pack.previewApplicable")}
            value={preview.applicable}
          />
          <CountLine
            label={t("pack.previewIdentical")}
            value={preview.identical}
          />
          <CountLine
            label={t("pack.previewConflicts")}
            value={preview.conflicts}
          />
          <CountLine
            label={t("pack.previewSourceChanged")}
            value={preview.sourceChanged}
          />
          <CountLine label={t("pack.previewOrphans")} value={preview.orphans} />

          <div className="mt-3 grid gap-2">
            <label className="text-xs text-muted-foreground">
              {t("pack.policyLabel")}
            </label>
            <Select
              value={policy}
              onValueChange={(v) => {
                setPolicy(v as ImportPolicy);
                setConfirmOverwriteAll(false);
              }}
            >
              <SelectTrigger className="h-7 text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="fill">{t("pack.policyFill")}</SelectItem>
                <SelectItem value="overwrite_except_reviewed">
                  {t("pack.policyOverwriteExceptReviewed")}
                </SelectItem>
                <SelectItem value="overwrite_all">
                  {t("pack.policyOverwriteAll")}
                </SelectItem>
              </SelectContent>
            </Select>

            {policy === "overwrite_all" && (
              <label className="flex items-start gap-2 rounded-md border border-destructive/40 bg-destructive/10 p-2 text-xs">
                <Checkbox
                  checked={confirmOverwriteAll}
                  onCheckedChange={(v) => setConfirmOverwriteAll(v === true)}
                />
                {t("pack.overwriteAllConfirm")}
              </label>
            )}

            {preview.sourceChanged > 0 && (
              <label className="flex items-center gap-2 text-xs">
                <Checkbox
                  checked={applySourceChanged}
                  onCheckedChange={(v) => setApplySourceChanged(v === true)}
                />
                {t("pack.applySourceChanged", {
                  count: preview.sourceChanged,
                })}
              </label>
            )}
            {preview.glossaryCount > 0 && (
              <label className="flex items-center gap-2 text-xs">
                <Checkbox
                  checked={importGlossary}
                  onCheckedChange={(v) => setImportGlossary(v === true)}
                />
                {t("pack.importGlossary", { count: preview.glossaryCount })}
              </label>
            )}
            {preview.tmCount > 0 && (
              <label className="flex items-center gap-2 text-xs">
                <Checkbox
                  checked={importTm}
                  onCheckedChange={(v) => setImportTm(v === true)}
                />
                {t("pack.importTm", { count: preview.tmCount })}
              </label>
            )}
          </div>
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel disabled={isApplying} onClick={onClose}>
            {t("pack.cancel")}
          </AlertDialogCancel>
          <AlertDialogAction
            disabled={!canApply}
            onClick={(e) => {
              // Keep the dialog open: it becomes the final report on success.
              e.preventDefault();
              void handleApply();
            }}
          >
            {isApplying && <Loader2 className="h-3.5 w-3.5 animate-spin" />}
            {t("pack.importConfirm")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
