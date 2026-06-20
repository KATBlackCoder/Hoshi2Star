import { create } from "zustand";
import { load } from "@tauri-apps/plugin-store";
import i18n from "i18next";
import { useLlmStore } from "@/stores/llm";

// ---------------------------------------------------------------------------
// Types & constants
// ---------------------------------------------------------------------------

export type Theme = "light" | "dark";
export type Language = "fr" | "en";

export interface AppSettings {
  ollamaUrl: string;
  ollamaModel: string;
  batchSize: number;
  theme: Theme;
  language: Language;
}

export const DEFAULT_SETTINGS: AppSettings = {
  ollamaUrl: "http://localhost:11434",
  ollamaModel: "qwen3:4b-instruct-2507-q8_0",
  batchSize: 20,
  theme: "dark",
  language: "fr",
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

    const ollamaUrl =
      (await store.get<string>("ollama_url")) ?? DEFAULT_SETTINGS.ollamaUrl;
    const ollamaModel =
      (await store.get<string>("ollama_model")) ?? DEFAULT_SETTINGS.ollamaModel;
    const batchSize =
      (await store.get<number>("batch_size")) ?? DEFAULT_SETTINGS.batchSize;
    const theme =
      ((await store.get<string>("theme")) as Theme | undefined) ??
      DEFAULT_SETTINGS.theme;
    const language =
      ((await store.get<string>("language")) as Language | undefined) ??
      DEFAULT_SETTINGS.language;

    const loaded: AppSettings = {
      ollamaUrl,
      ollamaModel,
      batchSize,
      theme,
      language,
    };

    set({ settings: loaded });

    applyThemeToDom(theme);
    void i18n.changeLanguage(language);
    useLlmStore
      .getState()
      .setProviderConfig({ url: ollamaUrl, model: ollamaModel, batchSize });
  },

  saveSettings: async (draft: AppSettings) => {
    const store = await load(STORE_FILE, { defaults: {}, autoSave: false });

    await store.set("ollama_url", draft.ollamaUrl);
    await store.set("ollama_model", draft.ollamaModel);
    await store.set("batch_size", draft.batchSize);
    await store.set("theme", draft.theme);
    await store.set("language", draft.language);
    await store.save();

    set({ settings: { ...draft } });

    applyThemeToDom(draft.theme);
    void i18n.changeLanguage(draft.language);
    useLlmStore.getState().setProviderConfig({
      url: draft.ollamaUrl,
      model: draft.ollamaModel,
      batchSize: draft.batchSize,
    });
  },
}));

// Selector
export const useSettings = () => useSettingsStore((s) => s.settings);
