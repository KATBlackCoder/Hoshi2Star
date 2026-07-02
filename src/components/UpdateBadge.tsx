import { useTranslation } from "react-i18next";
import { ArrowUpCircle } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useUpdaterStore } from "@/stores/updater";

// ---------------------------------------------------------------------------
// UpdateBadge — toolbar icon shown only after the user dismissed an available
// update ("Not now"). Clicking it reopens the decision (UpdateDialog).
// ---------------------------------------------------------------------------

export function UpdateBadge() {
  const { t } = useTranslation();
  const status = useUpdaterStore((s) => s.status);
  const reopen = useUpdaterStore((s) => s.reopen);

  if (status !== "dismissed") return null;

  return (
    <Button
      size="sm"
      variant="ghost"
      className="h-7 w-7 p-0 text-amber-400 hover:text-amber-300"
      title={t("updater.badge")}
      onClick={reopen}
    >
      <ArrowUpCircle className="h-4 w-4" />
    </Button>
  );
}
