import { useTranslation } from "react-i18next";
import { Button } from "@/components/ui/button";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { GAME_LANGUAGES } from "@/lib/constants";
import type { AppSettings, Language } from "@/stores/settings";

interface LanguageSettingsProps {
  draft: AppSettings;
  onChange: (patch: Partial<AppSettings>) => void;
  onInterfaceLanguageChange: (language: Language) => void;
}

export function LanguageSettings({
  draft,
  onChange,
  onInterfaceLanguageChange,
}: LanguageSettingsProps) {
  const { t } = useTranslation();

  return (
    <section aria-labelledby="settings-languages-heading" className="space-y-6">
      <div>
        <h3 id="settings-languages-heading" className="text-sm font-semibold">
          {t("settings.tabs.languages")}
        </h3>
        <p className="mt-1 text-xs leading-5 text-muted-foreground">
          {t("settings.translation.hint")}
        </p>
      </div>

      <div className="space-y-3 rounded-xl border bg-card/45 p-4">
        <h4 className="text-xs font-semibold uppercase tracking-[0.1em] text-muted-foreground">
          {t("settings.translation.section")}
        </h4>
        <div className="grid gap-3 sm:grid-cols-2">
          <div className="space-y-1.5">
            <label className="text-xs font-medium">
              {t("settings.translation.source")}
            </label>
            <Select
              value={draft.defaultSourceLang}
              onValueChange={(defaultSourceLang) =>
                onChange({ defaultSourceLang })
              }
            >
              <SelectTrigger className="min-h-10 w-full text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {GAME_LANGUAGES.map((language) => (
                  <SelectItem key={language.code} value={language.code}>
                    {language.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
          <div className="space-y-1.5">
            <label className="text-xs font-medium">
              {t("settings.translation.target")}
            </label>
            <Select
              value={draft.defaultTargetLang}
              onValueChange={(defaultTargetLang) =>
                onChange({ defaultTargetLang })
              }
            >
              <SelectTrigger className="min-h-10 w-full text-xs">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {GAME_LANGUAGES.map((language) => (
                  <SelectItem key={language.code} value={language.code}>
                    {language.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          </div>
        </div>
      </div>

      <div className="space-y-3 rounded-xl border bg-card/45 p-4">
        <h4 className="text-xs font-semibold uppercase tracking-[0.1em] text-muted-foreground">
          {t("settings.language.section")}
        </h4>
        <div className="flex gap-2">
          <Button
            type="button"
            variant={draft.language === "fr" ? "default" : "outline"}
            aria-pressed={draft.language === "fr"}
            onClick={() => onInterfaceLanguageChange("fr")}
          >
            🇫🇷 Français
          </Button>
          <Button
            type="button"
            variant={draft.language === "en" ? "default" : "outline"}
            aria-pressed={draft.language === "en"}
            onClick={() => onInterfaceLanguageChange("en")}
          >
            🇬🇧 English
          </Button>
        </div>
      </div>
    </section>
  );
}
