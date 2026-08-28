import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { BrainCircuit, Languages, Palette, Wrench } from "lucide-react";
import { toast } from "sonner";
import { AppearanceSettings } from "@/components/settings/AppearanceSettings";
import { LanguageSettings } from "@/components/settings/LanguageSettings";
import { ProviderSettings } from "@/components/settings/ProviderSettings";
import { DeveloperSettings } from "@/components/settings/DeveloperSettings";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogTitle,
} from "@/components/ui/dialog";
import { PROVIDER_PRESETS, type ProviderId } from "@/lib/constants";
import { cn } from "@/lib/utils";
import {
  DEFAULT_SETTINGS,
  applyThemeToDom,
  type AppSettings,
  type Language,
  type Theme,
  useSettings,
  useSettingsStore,
} from "@/stores/settings";

interface SettingsModalProps {
  open: boolean;
  onClose: () => void;
}

type SettingsSection = "provider" | "languages" | "appearance" | "developer";

const SECTIONS = [
  { id: "provider", labelKey: "settings.tabs.provider", icon: BrainCircuit },
  { id: "languages", labelKey: "settings.tabs.languages", icon: Languages },
  { id: "appearance", labelKey: "settings.tabs.appearance", icon: Palette },
  { id: "developer", labelKey: "settings.tabs.developer", icon: Wrench },
] as const;

export function SettingsModal({ open, onClose }: SettingsModalProps) {
  const { t, i18n } = useTranslation();
  const currentSettings = useSettings();
  const saveSettings = useSettingsStore((state) => state.saveSettings);
  const [draft, setDraft] = useState<AppSettings>(currentSettings);
  const [originalSettings, setOriginalSettings] =
    useState<AppSettings>(currentSettings);
  const [activeSection, setActiveSection] =
    useState<SettingsSection>("provider");
  const [models, setModels] = useState<string[]>([]);
  const [modelsLoading, setModelsLoading] = useState(false);
  const [modelsError, setModelsError] = useState<string | null>(null);
  useEffect(() => {
    if (!open) return;
    setDraft(currentSettings);
    setOriginalSettings(currentSettings);
    setActiveSection("provider");
    void fetchModels(currentSettings, { silent: true });
    // Re-synchronise only when the modal opens. Changes while editing belong
    // to the local draft until Save or Cancel is selected.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  async function fetchModels(settings: AppSettings, { silent = false } = {}) {
    setModelsLoading(true);
    setModelsError(null);
    try {
      const list = await invoke<string[]>("get_provider_models", {
        providerConfig: {
          providerId: settings.providerId,
          url: settings.ollamaUrl,
          model: settings.ollamaModel,
          apiKey: settings.apiKey.trim() || undefined,
          batchSize: settings.batchSize,
          resourceProfile: settings.resourceProfile,
        },
      });
      setModels(list);
      if (list.length > 0 && !list.includes(settings.ollamaModel)) {
        setDraft((current) => ({ ...current, ollamaModel: list[0] }));
      }
      if (!silent) {
        toast.success(t("settings.llm.testSuccess", { count: list.length }));
      }
    } catch {
      setModelsError(t("settings.llm.modelError"));
      setModels([]);
      if (!silent) toast.error(t("settings.llm.modelError"));
    } finally {
      setModelsLoading(false);
    }
  }

  function updateDraft(patch: Partial<AppSettings>) {
    setDraft((current) => ({ ...current, ...patch }));
  }

  function handleProviderChange(providerId: ProviderId) {
    const preset = PROVIDER_PRESETS.find(
      (candidate) => candidate.id === providerId,
    );
    if (!preset) return;
    setModels([]);
    setModelsError(null);
    updateDraft({
      providerId,
      ollamaUrl: preset.url,
      ollamaModel: preset.model,
      apiKey: "",
    });
  }

  function handleThemeChange(theme: Theme) {
    updateDraft({ theme });
    applyThemeToDom(theme);
  }

  function handleLanguageChange(language: Language) {
    updateDraft({ language });
    void i18n.changeLanguage(language);
  }

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
    setDraft({ ...DEFAULT_SETTINGS });
    applyThemeToDom(DEFAULT_SETTINGS.theme);
    void i18n.changeLanguage(DEFAULT_SETTINGS.language);
  }

  return (
    <Dialog open={open} onOpenChange={(nextOpen) => !nextOpen && handleCancel()}>
      <DialogContent
        className="flex h-[min(42rem,calc(100vh-1.5rem))] w-[min(50rem,calc(100vw-1.5rem))] max-w-none flex-col gap-0 overflow-hidden p-0"
        onCloseAutoFocus={(event) => {
          event.preventDefault();
          document.getElementById("settings-trigger")?.focus();
        }}
      >
        <div className="flex min-h-14 shrink-0 items-center justify-between px-5">
          <DialogTitle id="settings-title" className="text-base font-semibold">
            {t("settings.title")}
          </DialogTitle>
          <DialogDescription className="sr-only">
            {t("settings.sections")}
          </DialogDescription>
          <span className="pr-10 text-[10px] uppercase tracking-[0.16em] text-muted-foreground">星詠み工房</span>
        </div>

        <div className="flex min-h-0 min-w-0 flex-1 flex-col border-y min-[620px]:flex-row">
          <nav
            role="tablist"
            aria-label={t("settings.sections")}
            aria-orientation="vertical"
            className="flex shrink-0 gap-1 overflow-x-auto border-b bg-muted/20 p-2 [scrollbar-width:none] min-[620px]:w-44 min-[620px]:flex-col min-[620px]:overflow-x-visible min-[620px]:border-b-0 min-[620px]:border-r"
          >
            {SECTIONS.map(({ id, labelKey, icon: Icon }, index) => (
              <button
                key={id}
                id={`settings-tab-${id}`}
                type="button"
                role="tab"
                aria-selected={activeSection === id}
                aria-controls={`settings-panel-${id}`}
                tabIndex={activeSection === id ? 0 : -1}
                className={cn(
                  "flex min-h-10 shrink-0 items-center gap-2 rounded-lg px-3 text-left text-xs font-medium transition-[background-color,color,transform] active:scale-[0.96] min-[620px]:w-full",
                  activeSection === id
                    ? "bg-accent text-foreground shadow-[var(--shadow-surface)]"
                    : "text-muted-foreground hover:bg-accent/55 hover:text-foreground",
                )}
                onClick={() => setActiveSection(id)}
                onKeyDown={(event) => {
                  const delta = event.key === "ArrowDown" || event.key === "ArrowRight" ? 1 : event.key === "ArrowUp" || event.key === "ArrowLeft" ? -1 : 0;
                  if (!delta) return;
                  event.preventDefault();
                  const next = SECTIONS[(index + delta + SECTIONS.length) % SECTIONS.length];
                  setActiveSection(next.id);
                  document.getElementById(`settings-tab-${next.id}`)?.focus();
                }}
              >
                <Icon className="h-4 w-4 shrink-0" />
                {t(labelKey)}
              </button>
            ))}
          </nav>

          <div
            id={`settings-panel-${activeSection}`}
            role="tabpanel"
            aria-labelledby={`settings-tab-${activeSection}`}
            className="min-h-0 min-w-0 flex-1 overflow-y-auto p-4 sm:p-6"
          >
            {activeSection === "provider" ? (
              <ProviderSettings
                draft={draft}
                models={models}
                modelsLoading={modelsLoading}
                modelsError={modelsError}
                onChange={updateDraft}
                onProviderChange={handleProviderChange}
                onTest={() => void fetchModels(draft)}
                onUrlBlur={() => void fetchModels(draft, { silent: true })}
              />
            ) : activeSection === "languages" ? (
              <LanguageSettings
                draft={draft}
                onChange={updateDraft}
                onInterfaceLanguageChange={handleLanguageChange}
              />
            ) : activeSection === "appearance" ? (
              <AppearanceSettings
                theme={draft.theme}
                onThemeChange={handleThemeChange}
              />
            ) : (
              <DeveloperSettings
                enabled={draft.developerTools}
                onEnabledChange={(developerTools) =>
                  updateDraft({ developerTools })
                }
              />
            )}
          </div>
        </div>

        <div className="flex min-h-16 shrink-0 flex-wrap items-center justify-between gap-2 px-4 sm:px-5">
          <Button type="button" variant="ghost" size="sm" onClick={handleReset}>
            {t("settings.reset")}
          </Button>
          <div className="flex gap-2">
            <Button
              type="button"
              variant="outline"
              size="sm"
              onClick={handleCancel}
            >
              {t("settings.cancel")}
            </Button>
            <Button type="button" size="sm" onClick={() => void handleSave()}>
              {t("settings.save")}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
