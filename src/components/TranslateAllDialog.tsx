import { useState } from "react";
import { useTranslation } from "react-i18next";
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
import { Input } from "@/components/ui/input";

interface TranslateAllDialogProps {
  open: boolean;
  segmentCount: number;
  fileCount: number;
  onStart: (thresholdMins: number, cooldownMins: number) => void;
  onCancel: () => void;
}

export function TranslateAllDialog({
  open,
  segmentCount,
  fileCount,
  onStart,
  onCancel,
}: TranslateAllDialogProps) {
  const { t } = useTranslation();
  const [threshold, setThreshold] = useState("20");
  const [cooldown, setCooldown] = useState("3");

  function handleStart() {
    const thresholdMins = Math.max(1, parseInt(threshold, 10) || 20);
    const cooldownMins = Math.max(0, parseInt(cooldown, 10) || 3);
    onStart(thresholdMins, cooldownMins);
  }

  return (
    <AlertDialog open={open}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>
            {t("toolbar.translateAllDialogTitle")}
          </AlertDialogTitle>
          <AlertDialogDescription>
            {t("toolbar.translateAllDialogDesc", {
              count: segmentCount,
              files: fileCount,
            })}
          </AlertDialogDescription>
        </AlertDialogHeader>

        <div className="grid gap-3 py-2">
          <div className="grid grid-cols-2 items-center gap-3">
            <label
              htmlFor="ta-threshold"
              className="text-xs text-muted-foreground"
            >
              {t("toolbar.translateAllThreshold")}
            </label>
            <Input
              id="ta-threshold"
              type="number"
              min={1}
              className="h-7 text-xs"
              value={threshold}
              onChange={(e) => setThreshold(e.target.value)}
            />
          </div>
          <div className="grid grid-cols-2 items-center gap-3">
            <label
              htmlFor="ta-cooldown"
              className="text-xs text-muted-foreground"
            >
              {t("toolbar.translateAllCooldown")}
            </label>
            <Input
              id="ta-cooldown"
              type="number"
              min={0}
              className="h-7 text-xs"
              value={cooldown}
              onChange={(e) => setCooldown(e.target.value)}
            />
          </div>
        </div>

        <AlertDialogFooter>
          <AlertDialogCancel onClick={onCancel}>
            {t("toolbar.translateAllCancel")}
          </AlertDialogCancel>
          <AlertDialogAction onClick={handleStart}>
            {t("toolbar.translateAllStart")}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
