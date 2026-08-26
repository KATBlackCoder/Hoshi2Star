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

      <div className="grid grid-cols-2 gap-3">
        <Button
          type="button"
          variant={theme === "light" ? "default" : "outline"}
          className="h-24 flex-col gap-2"
          aria-pressed={theme === "light"}
          onClick={() => onThemeChange("light")}
        >
          <Sun className="h-5 w-5" />
          {t("settings.appearance.light")}
        </Button>
        <Button
          type="button"
          variant={theme === "dark" ? "default" : "outline"}
          className="h-24 flex-col gap-2"
          aria-pressed={theme === "dark"}
          onClick={() => onThemeChange("dark")}
        >
          <Moon className="h-5 w-5" />
          {t("settings.appearance.dark")}
        </Button>
      </div>
    </section>
  );
}
