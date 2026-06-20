import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import type { FontScanResult } from "@/lib/types";

interface FontSizeDialogProps {
  open: boolean;
  scan: FontScanResult;
  onApply: (fontSize: number, replaceExisting: boolean) => void;
  onSkip: () => void;
}

export function FontSizeDialog({
  open,
  scan,
  onApply,
  onSkip,
}: FontSizeDialogProps) {
  const { t } = useTranslation();
  const [fontSize, setFontSize] = useState(18);
  const [replaceExisting, setReplaceExisting] = useState(true);

  return (
    <AlertDialog open={open}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("fontSizeDialog.title")}</AlertDialogTitle>
        </AlertDialogHeader>

        <p className="text-sm text-muted-foreground">
          {t("fontSizeDialog.desc", {
            total: scan.totalTranslated,
            existing: scan.existingFontCount,
          })}
        </p>

        <div className="flex items-center gap-3 mt-2">
          <span className="shrink-0 text-sm">{t("fontSizeDialog.label")}</span>
          <Input
            type="number"
            min={8}
            max={64}
            value={fontSize}
            onChange={(e) => setFontSize(Number(e.target.value))}
            className="w-20 h-8 text-sm"
          />
        </div>

        {scan.existingFontCount > 0 && (
          <label className="flex items-center gap-2 text-sm cursor-pointer mt-1">
            <input
              type="checkbox"
              checked={replaceExisting}
              onChange={(e) => setReplaceExisting(e.target.checked)}
              className="accent-primary"
            />
            {t("fontSizeDialog.replaceLabel", {
              count: scan.existingFontCount,
            })}
          </label>
        )}

        <AlertDialogFooter className="gap-2">
          <Button variant="ghost" size="sm" onClick={onSkip}>
            {t("fontSizeDialog.skip")}
          </Button>
          <Button size="sm" onClick={() => onApply(fontSize, replaceExisting)}>
            {t("fontSizeDialog.apply")}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
