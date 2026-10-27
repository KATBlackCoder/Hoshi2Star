import { useState } from "react";
import { useTranslation } from "react-i18next";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogDescription,
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
  const wolf = scan.engine === "wolf";
  const gameFontSize = scan.gameFontSize ?? null;
  // Wolf: start from the game's own size (a fixed 18 enlarges every text and
  // makes it overflow; 12 is the default of the calibrated 2.23 game). Empty
  // when unknown: "Skip" keeps the game's size.
  const [fontSize, setFontSize] = useState<number | null>(
    wolf ? gameFontSize : 18,
  );
  const [replaceExisting, setReplaceExisting] = useState(true);

  const code = wolf ? "\\f[N]" : "\\FS[N]";

  return (
    <AlertDialog open={open}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t("fontSizeDialog.title")}</AlertDialogTitle>
          <AlertDialogDescription>
            {t("fontSizeDialog.desc", {
              total: scan.totalTranslated,
              existing: scan.existingFontCount,
              code,
            })}
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="flex items-center gap-3 mt-2">
          <span className="shrink-0 text-sm">
            {t("fontSizeDialog.label", { code })}
          </span>
          <Input
            type="number"
            min={8}
            max={64}
            value={fontSize ?? ""}
            onChange={(e) =>
              setFontSize(e.target.value === "" ? null : Number(e.target.value))
            }
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
              code,
            })}
          </label>
        )}

        {scan.engine === "mv_mz" && (
          <p className="text-xs text-muted-foreground mt-1">
            {t("fontSizeDialog.hintMvMz")}
          </p>
        )}

        {wolf && (
          <p className="text-xs text-muted-foreground mt-1">
            {gameFontSize === null
              ? t("fontSizeDialog.hintWolfUnknown")
              : t("fontSizeDialog.hintWolfGame", { size: gameFontSize })}
          </p>
        )}

        <AlertDialogFooter className="gap-2">
          <Button variant="ghost" size="sm" onClick={onSkip}>
            {t("fontSizeDialog.skip")}
          </Button>
          <Button
            size="sm"
            disabled={fontSize === null}
            onClick={() => {
              if (fontSize !== null) onApply(fontSize, replaceExisting);
            }}
          >
            {t("fontSizeDialog.apply")}
          </Button>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
