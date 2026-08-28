import { Moon, Sun } from "lucide-react";
import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import type { Theme } from "@/stores/settings";

interface AppearanceSettingsProps {
  theme: Theme;
  onThemeChange: (theme: Theme) => void;
}

export function AppearanceSettings({
  theme,
  onThemeChange,
}: AppearanceSettingsProps) {
  const { t } = useTranslation();

  return (
    <section
      aria-labelledby="settings-appearance-heading"
      className="space-y-5"
    >
      <div>
        <h3 id="settings-appearance-heading" className="text-sm font-semibold">
          {t("settings.appearance.section")}
        </h3>
        <p className="mt-1 text-xs leading-5 text-muted-foreground">
          {t("settings.appearance.description")}
        </p>
      </div>

      <div className="grid gap-3 sm:grid-cols-2">
        <Button
          type="button"
          variant={theme === "light" ? "default" : "outline"}
          className="h-auto min-h-36 min-w-0 flex-col items-stretch gap-3 overflow-hidden p-3 text-left whitespace-normal"
          aria-pressed={theme === "light"}
          aria-label={t("settings.appearance.light")}
          onClick={() => onThemeChange("light")}
        >
          <span className="observatory-grid flex h-16 w-full items-center justify-center rounded-lg bg-[#f2ead7] text-[#28345d] shadow-inner">
            <Sun className="h-5 w-5" />
          </span>
          <span>
            <span className="block text-sm font-semibold">{t("settings.appearance.light")}</span>
            <span className="mt-1 block text-pretty text-[11px] leading-4 opacity-75">{t("settings.appearance.lightDescription")}</span>
          </span>
        </Button>
        <Button
          type="button"
          variant={theme === "dark" ? "default" : "outline"}
          className="h-auto min-h-36 min-w-0 flex-col items-stretch gap-3 overflow-hidden p-3 text-left whitespace-normal"
          aria-pressed={theme === "dark"}
          aria-label={t("settings.appearance.dark")}
          onClick={() => onThemeChange("dark")}
        >
          <span className="observatory-grid flex h-16 w-full items-center justify-center rounded-lg bg-[#17152f] text-[#e7bd5e] shadow-inner">
            <Moon className="h-5 w-5" />
          </span>
          <span>
            <span className="block text-sm font-semibold">{t("settings.appearance.dark")}</span>
            <span className="mt-1 block text-pretty text-[11px] leading-4 opacity-75">{t("settings.appearance.darkDescription")}</span>
          </span>
        </Button>
      </div>
    </section>
  );
}
