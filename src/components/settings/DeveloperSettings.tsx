import { Bug, FlaskConical } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";

interface DeveloperSettingsProps {
  enabled: boolean;
  onEnabledChange: (enabled: boolean) => void;
}

export function DeveloperSettings({
  enabled,
  onEnabledChange,
}: DeveloperSettingsProps) {
  const { t } = useTranslation();

  return (
    <section
      aria-labelledby="settings-developer-heading"
      className="flex flex-col gap-5"
    >
      <div>
        <div className="flex items-center gap-2">
          <h3 id="settings-developer-heading" className="text-sm font-semibold">
            {t("settings.developer.section")}
          </h3>
          <Badge variant="secondary">
            {t("settings.developer.debugBadge")}
          </Badge>
        </div>
        <p className="mt-1 text-pretty text-xs leading-5 text-muted-foreground">
          {t("settings.developer.description")}
        </p>
      </div>

      <label
        htmlFor="developer-tools-enabled"
        className="group flex min-h-20 cursor-pointer items-start gap-3 rounded-xl border bg-card/45 p-4 transition-[background-color,border-color,box-shadow] hover:bg-accent/30 has-focus-visible:border-ring has-focus-visible:ring-3 has-focus-visible:ring-ring/50"
      >
        <Checkbox
          id="developer-tools-enabled"
          checked={enabled}
          onCheckedChange={(checked) => onEnabledChange(checked === true)}
          aria-label={t("settings.developer.enable")}
          aria-describedby="developer-tools-description"
        />
        <span className="flex min-w-0 flex-1 items-start gap-3">
          <span className="grid size-9 shrink-0 place-items-center rounded-lg bg-muted text-muted-foreground">
            <Bug className="size-4" aria-hidden="true" />
          </span>
          <span className="min-w-0">
            <span className="block text-sm font-medium">
              {t("settings.developer.enable")}
            </span>
            <span
              id="developer-tools-description"
              className="mt-1 block text-pretty text-xs leading-5 text-muted-foreground"
            >
              {t("settings.developer.enableDescription")}
            </span>
          </span>
        </span>
      </label>

      <div className="flex items-start gap-3 rounded-xl bg-muted/40 p-4">
        <FlaskConical className="mt-0.5 size-4 shrink-0 text-muted-foreground" />
        <p className="text-pretty text-xs leading-5 text-muted-foreground">
          {t("settings.developer.pilotExplanation")}
        </p>
      </div>
    </section>
  );
}
