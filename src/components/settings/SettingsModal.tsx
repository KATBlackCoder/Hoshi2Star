import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Moon, Sun, X } from "lucide-react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  useSettingsStore,
  useSettings,
  applyThemeToDom,
  DEFAULT_SETTINGS,
  type AppSettings,
} from "@/stores/settings";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

export function SettingsModal({ open, onClose }: SettingsModalProps) {
  const { t, i18n } = useTranslation();
  const currentSettings = useSettings();
  const { saveSettings } = useSettingsStore();

  const [draft, setDraft] = useState<AppSettings>(currentSettings);
  const [originalSettings, setOriginalSettings] =
    useState<AppSettings>(currentSettings);

  const [models, setModels] = useState<string[]>([]);
  const [modelsLoading, setModelsLoading] = useState(false);
  const [modelsError, setModelsError] = useState<string | null>(null);

  // Re-sync local draft/original from the store every time the modal opens —
  // `currentSettings` may have changed (loadSettings resolved, or a previous
  // save) since this component's state was first initialized.
  useEffect(() => {
    if (open) {
      setDraft(currentSettings);
      setOriginalSettings(currentSettings);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  async function fetchModels(url: string, { silent = false } = {}) {
    setModelsLoading(true);
    setModelsError(null);
    try {
      const list = await invoke<string[]>("get_ollama_models", { url });
      setModels(list);
      if (list.length > 0 && !list.includes(draft.ollamaModel)) {
        setDraft((d) => ({ ...d, ollamaModel: list[0] }));
      }
      if (!silent) {
        toast.success(t("settings.llm.testSuccess", { count: list.length }));
      }
    } catch {
      setModelsError(t("settings.llm.modelError"));
      setModels([]);
      if (!silent) {
        toast.error(t("settings.llm.modelError"));
      }
    } finally {
      setModelsLoading(false);
    }
  }

  useEffect(() => {
    if (open) {
      void fetchModels(draft.ollamaUrl, { silent: true });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  if (!open) return null;

  function handleCancel() {
    applyThemeToDom(originalSettings.theme);
    void i18n.changeLanguage(originalSettings.language);
    onClose();
  }

  async function handleSave() {
    await saveSettings(draft);
    onClose();
  }

  function handleReset() {
    setDraft(DEFAULT_SETTINGS);
    applyThemeToDom(DEFAULT_SETTINGS.theme);
    void i18n.changeLanguage(DEFAULT_SETTINGS.language);
  }

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/50">
      <div className="w-[420px] rounded-lg border bg-background p-5 shadow-xl">
        {/* Header */}
        <div className="mb-4 flex items-center justify-between">
          <h2 className="text-sm font-semibold">{t("settings.title")}</h2>
          <button type="button" onClick={handleCancel}>
            <X className="h-4 w-4 text-muted-foreground hover:text-foreground" />
          </button>
        </div>

        <div className="space-y-5">
          {/* Section LLM */}
          <section className="space-y-2">
            <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("settings.llm.section")}
            </h3>

            <div className="space-y-1">
              <label className="block text-xs text-muted-foreground">
                {t("settings.llm.urlLabel")}
              </label>
              <div className="flex gap-2">
                <Input
                  className="h-8 text-xs"
                  value={draft.ollamaUrl}
                  onChange={(e) =>
                    setDraft((d) => ({ ...d, ollamaUrl: e.target.value }))
                  }
                  onBlur={() =>
                    void fetchModels(draft.ollamaUrl, { silent: true })
                  }
                />
                <Button
                  size="sm"
                  variant="outline"
                  className="h-8 shrink-0 text-xs"
                  onClick={() => void fetchModels(draft.ollamaUrl)}
                  disabled={modelsLoading}
                >
                  {t("settings.llm.testButton")}
                </Button>
              </div>
            </div>

            <div className="space-y-1">
              <label className="block text-xs text-muted-foreground">
                {t("settings.llm.modelLabel")}
              </label>
              {models.length > 0 ? (
                <Select
                  value={draft.ollamaModel}
                  onValueChange={(v) =>
                    setDraft((d) => ({ ...d, ollamaModel: v }))
                  }
                >
                  <SelectTrigger className="h-8 w-full text-xs">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    {models.map((m) => (
                      <SelectItem key={m} value={m} className="text-xs">
                        {m}
                      </SelectItem>
                    ))}
                  </SelectContent>
                </Select>
              ) : (
                <>
                  <Select disabled>
                    <SelectTrigger className="h-8 w-full text-xs">
                      <SelectValue
                        placeholder={
                          modelsLoading
                            ? t("settings.llm.modelLoading")
                            : t("settings.llm.modelNone")
                        }
                      />
                    </SelectTrigger>
                    <SelectContent />
                  </Select>
                  {modelsError && (
                    <Input
                      className="mt-1.5 h-8 text-xs"
                      placeholder={t("settings.llm.modelManual")}
                      value={draft.ollamaModel}
                      onChange={(e) =>
                        setDraft((d) => ({ ...d, ollamaModel: e.target.value }))
                      }
                    />
                  )}
                </>
              )}
              {modelsError && (
                <p className="text-[11px] text-destructive">{modelsError}</p>
              )}
            </div>

            <div className="space-y-1">
              <label className="block text-xs text-muted-foreground">
                {t("settings.llm.batchSizeLabel")}
              </label>
              <Input
                type="number"
                min={1}
                max={100}
                className="h-8 text-xs"
                value={draft.batchSize}
                onChange={(e) =>
                  setDraft((d) => ({
                    ...d,
                    batchSize: Number(e.target.value),
                  }))
                }
                onBlur={() =>
                  setDraft((d) => ({
                    ...d,
                    batchSize: Math.min(100, Math.max(1, d.batchSize || 1)),
                  }))
                }
              />
              <p className="text-[11px] text-muted-foreground">
                {t("settings.llm.batchSizeHint")}
              </p>
            </div>
          </section>

          {/* Section Apparence */}
          <section className="space-y-2">
            <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("settings.appearance.section")}
            </h3>
            <div className="flex gap-2">
              <Button
                size="sm"
                variant={draft.theme === "light" ? "default" : "outline"}
                className="h-8 gap-1.5 text-xs"
                onClick={() => {
                  setDraft((d) => ({ ...d, theme: "light" }));
                  applyThemeToDom("light");
                }}
              >
                <Sun className="h-3.5 w-3.5" />
                {t("settings.appearance.light")}
              </Button>
              <Button
                size="sm"
                variant={draft.theme === "dark" ? "default" : "outline"}
                className="h-8 gap-1.5 text-xs"
                onClick={() => {
                  setDraft((d) => ({ ...d, theme: "dark" }));
                  applyThemeToDom("dark");
                }}
              >
                <Moon className="h-3.5 w-3.5" />
                {t("settings.appearance.dark")}
              </Button>
            </div>
          </section>

          {/* Section Langue */}
          <section className="space-y-2">
            <h3 className="text-xs font-medium uppercase tracking-wide text-muted-foreground">
              {t("settings.language.section")}
            </h3>
            <div className="flex gap-2">
              <Button
                size="sm"
                variant={draft.language === "fr" ? "default" : "outline"}
                className="h-8 text-xs"
                onClick={() => {
                  setDraft((d) => ({ ...d, language: "fr" }));
                  void i18n.changeLanguage("fr");
                }}
              >
                🇫🇷 FR
              </Button>
              <Button
                size="sm"
                variant={draft.language === "en" ? "default" : "outline"}
                className="h-8 text-xs"
                onClick={() => {
                  setDraft((d) => ({ ...d, language: "en" }));
                  void i18n.changeLanguage("en");
                }}
              >
                🇬🇧 EN
              </Button>
            </div>
          </section>
        </div>

        {/* Footer */}
        <div className="mt-5 flex items-center justify-between">
          <Button
            size="sm"
            variant="ghost"
            className="h-7 text-xs"
            onClick={handleReset}
          >
            {t("settings.reset")}
          </Button>
          <div className="flex gap-2">
            <Button
              size="sm"
              variant="outline"
              className="h-7 text-xs"
              onClick={handleCancel}
            >
              {t("settings.cancel")}
            </Button>
            <Button
              size="sm"
              className="h-7 text-xs"
              onClick={() => void handleSave()}
            >
              {t("settings.save")}
            </Button>
          </div>
        </div>
      </div>
    </div>
  );
}
