import { useTranslation } from "react-i18next";
import { AlertTriangle, WrapText } from "lucide-react";
import type { FitScan } from "@/lib/types";

// ---------------------------------------------------------------------------
// ExportFitSummary — what text fitting does to this export (confirm dialog)
// ---------------------------------------------------------------------------

export function ExportFitSummary({ scan }: { scan: FitScan }) {
  const { t } = useTranslation();
  const sum = (key: "rewrapped" | "newPage" | "shrunk" | "tooLong") =>
    scan.dialogue[key] + scan.descriptions[key];
  const tooLong = sum("tooLong");
  return (
    <div className="flex items-start gap-2 rounded-lg border p-3 text-left text-xs">
      {tooLong > 0 ? (
        <AlertTriangle className="mt-0.5 h-3.5 w-3.5 shrink-0 text-amber-500" />
      ) : (
        <WrapText className="mt-0.5 h-3.5 w-3.5 shrink-0 text-muted-foreground" />
      )}
      <p>
        {t("textFit.exportSummary", {
          rewrapped: sum("rewrapped"),
          newPage: sum("newPage"),
          shrunk: sum("shrunk"),
          tooLong,
        })}
      </p>
    </div>
  );
}
