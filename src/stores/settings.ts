import { create } from "zustand";
import { load } from "@tauri-apps/plugin-store";
import i18n from "i18next";
import { useLlmStore } from "@/stores/llm";
import {
  DEFAULT_BATCH_SIZE,
  DEFAULT_OLLAMA_MODEL,
  DEFAULT_OLLAMA_URL,
  type ProviderId,
} from "@/lib/constants";
import type { ResourceProfile } from "@/lib/types";

// ---------------------------------------------------------------------------
// Types & constants
// ---------------------------------------------------------------------------

export type Theme = "light" | "dark";
export type Language = "fr" | "en";

export interface AppSettings {
  providerId: ProviderId;
  ollamaUrl: string;
  ollamaModel: string;
  /** Kept in Zustand memory only; never written to plugin-store. */
  apiKey: string;
  batchSize: number;
  resourceProfile: ResourceProfile;
  theme: Theme;
  language: Language;
  defaultSourceLang: string;
  defaultTargetLang: string;
  /** Hidden by default; exposes diagnostics that are not part of the core flow. */
  developerTools: boolean;
}

export const DEFAULT_SETTINGS: AppSettings = {
  providerId: "ollama",
  ollamaUrl: DEFAULT_OLLAMA_URL,
  ollamaModel: DEFAULT_OLLAMA_MODEL,
  apiKey: "",
  batchSize: DEFAULT_BATCH_SIZE,
  resourceProfile: "balanced",
  theme: "dark",
  language: "fr",
  defaultSourceLang: "ja",
  defaultTargetLang: "fr",
  developerTools: false,
};

const STORE_FILE = "settings.json";

// ---------------------------------------------------------------------------
// DOM helper — exported for modal preview
// ---------------------------------------------------------------------------

export function applyThemeToDom(theme: Theme) {
  document.documentElement.classList.toggle("dark", theme === "dark");
}

// ---------------------------------------------------------------------------
// Store
// ---------------------------------------------------------------------------

interface SettingsState {
  settings: AppSettings;
  loadSettings: () => Promise<void>;
  saveSettings: (draft: AppSettings) => Promise<void>;
}

export const useSettingsStore = create<SettingsState>()((set) => ({
  settings: { ...DEFAULT_SETTINGS },

  loadSettings: async () => {
    const store = await load(STORE_FILE, { defaults: {}, autoSave: false });

    // Independent reads: issued together rather than one IPC round trip
    // after the other.
    const [
      storedProviderId,
      providerUrl,
      legacyOllamaUrl,
      providerModel,
      legacyOllamaModel,
      storedBatchSize,
      storedResourceProfile,
      storedTheme,
      storedLanguage,
      storedSourceLang,
      storedTargetLang,
      storedDeveloperTools,
    ] = await Promise.all([
      store.get<string>("provider_id"),
      store.get<string>("provider_url"),
      store.get<string>("ollama_url"),
      store.get<string>("provider_model"),
      store.get<string>("ollama_model"),
      store.get<number>("batch_size"),
      store.get<string>("resource_profile"),
      store.get<string>("theme"),
      store.get<string>("language"),
      store.get<string>("default_source_lang"),
      store.get<string>("default_target_lang"),
      store.get<boolean>("developer_tools"),
    ]);

    const providerId =
      (storedProviderId as ProviderId | undefined) ??
      DEFAULT_SETTINGS.providerId;
    const ollamaUrl =
      providerUrl ?? legacyOllamaUrl ?? DEFAULT_SETTINGS.ollamaUrl;
    const ollamaModel =
      providerModel ?? legacyOllamaModel ?? DEFAULT_SETTINGS.ollamaModel;
    const batchSize = storedBatchSize ?? DEFAULT_SETTINGS.batchSize;
    const resourceProfile =
      (storedResourceProfile as ResourceProfile | undefined) ??
      DEFAULT_SETTINGS.resourceProfile;
    const theme = (storedTheme as Theme | undefined) ?? DEFAULT_SETTINGS.theme;
    const language =
      (storedLanguage as Language | undefined) ?? DEFAULT_SETTINGS.language;
    const defaultSourceLang =
      storedSourceLang ?? DEFAULT_SETTINGS.defaultSourceLang;
    const defaultTargetLang =
      storedTargetLang ?? DEFAULT_SETTINGS.defaultTargetLang;
    const developerTools =
      storedDeveloperTools ?? DEFAULT_SETTINGS.developerTools;

    const loaded: AppSettings = {
      providerId,
      ollamaUrl,
      ollamaModel,
      apiKey: "",
      batchSize,
      resourceProfile,
      theme,
      language,
      defaultSourceLang,
      defaultTargetLang,
      developerTools,
    };

    set({ settings: loaded });

    applyThemeToDom(theme);
    void i18n.changeLanguage(language);
    useLlmStore.getState().setProviderConfig({
      providerId,
      url: ollamaUrl,
      model: ollamaModel,
      apiKey: undefined,
      batchSize,
      resourceProfile,
    });
  },

  saveSettings: async (draft: AppSettings) => {
    const store = await load(STORE_FILE, { defaults: {}, autoSave: false });

    await store.set("provider_id", draft.providerId);
    await store.set("provider_url", draft.ollamaUrl);
    await store.set("provider_model", draft.ollamaModel);
    await store.set("batch_size", draft.batchSize);
    await store.set("resource_profile", draft.resourceProfile);
    await store.set("theme", draft.theme);
    await store.set("language", draft.language);
    await store.set("default_source_lang", draft.defaultSourceLang);
    await store.set("default_target_lang", draft.defaultTargetLang);
    await store.set("developer_tools", draft.developerTools);
    await store.save();

    set({ settings: { ...draft } });

    applyThemeToDom(draft.theme);
    void i18n.changeLanguage(draft.language);
    useLlmStore.getState().setProviderConfig({
      providerId: draft.providerId,
      url: draft.ollamaUrl,
      model: draft.ollamaModel,
      apiKey: draft.apiKey.trim() || undefined,
      batchSize: draft.batchSize,
      resourceProfile: draft.resourceProfile,
    });
  },
}));

// Selector
export const useSettings = () => useSettingsStore((s) => s.settings);
