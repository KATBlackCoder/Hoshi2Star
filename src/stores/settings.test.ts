import { beforeEach, describe, expect, it, vi } from "vitest";

// In-memory stand-in for @tauri-apps/plugin-store — one map per STORE_FILE.
const storeData = new Map<string, unknown>();
vi.mock("@tauri-apps/plugin-store", () => ({
  load: vi.fn(async () => ({
    get: async (key: string) => storeData.get(key),
    set: async (key: string, value: unknown) => {
      storeData.set(key, value);
    },
    save: async () => {},
  })),
}));

import { DEFAULT_SETTINGS, useSettingsStore } from "@/stores/settings";
import { useLlmStore } from "@/stores/llm";
import { DEFAULT_OLLAMA_MODEL } from "@/lib/constants";

const initialSettings = useSettingsStore.getState();
const initialLlm = useLlmStore.getState();

beforeEach(() => {
  storeData.clear();
  useSettingsStore.setState(initialSettings, true);
  useLlmStore.setState(initialLlm, true);
});

describe("settings store", () => {
  it("defaults use the unified model constant", () => {
    expect(DEFAULT_SETTINGS.ollamaModel).toBe(DEFAULT_OLLAMA_MODEL);
    expect(useSettingsStore.getState().settings).toEqual(DEFAULT_SETTINGS);
  });

  it("loadSettings falls back to defaults when the store file is empty", async () => {
    await useSettingsStore.getState().loadSettings();
    expect(useSettingsStore.getState().settings).toEqual(DEFAULT_SETTINGS);
  });

  it("saveSettings persists the draft and pushes providerConfig into the llm store", async () => {
    const draft = {
      ...DEFAULT_SETTINGS,
      providerId: "huggingface" as const,
      ollamaUrl: "https://pod.example:11434",
      ollamaModel: "custom:7b",
      apiKey: "secret-memory-only",
      batchSize: 5,
      resourceProfile: "eco" as const,
    };
    await useSettingsStore.getState().saveSettings(draft);

    expect(useSettingsStore.getState().settings).toEqual(draft);
    expect(storeData.get("provider_id")).toBe("huggingface");
    expect(storeData.get("provider_model")).toBe("custom:7b");
    expect(storeData.get("resource_profile")).toBe("eco");
    expect(storeData.has("api_key")).toBe(false);
    expect(storeData.get("default_source_lang")).toBe("ja");
    expect(storeData.get("default_target_lang")).toBe("fr");
    expect(storeData.get("developer_tools")).toBe(false);

    // The llm store must reflect the saved settings immediately.
    const cfg = useLlmStore.getState().providerConfig;
    expect(cfg.url).toBe("https://pod.example:11434");
    expect(cfg.model).toBe("custom:7b");
    expect(cfg.providerId).toBe("huggingface");
    expect(cfg.apiKey).toBe("secret-memory-only");
    expect(cfg.batchSize).toBe(5);
    expect(cfg.resourceProfile).toBe("eco");
  });

  it("loadSettings restores persisted values over defaults", async () => {
    storeData.set("ollama_model", "persisted:3b");
    storeData.set("batch_size", 7);
    storeData.set("resource_profile", "fast");
    storeData.set("default_source_lang", "ko");
    storeData.set("default_target_lang", "fr");
    storeData.set("developer_tools", true);
    await useSettingsStore.getState().loadSettings();

    const s = useSettingsStore.getState().settings;
    expect(s.ollamaModel).toBe("persisted:3b");
    expect(s.batchSize).toBe(7);
    expect(s.resourceProfile).toBe("fast");
    expect(s.defaultSourceLang).toBe("ko");
    expect(s.defaultTargetLang).toBe("fr");
    expect(s.developerTools).toBe(true);
    expect(s.ollamaUrl).toBe(DEFAULT_SETTINGS.ollamaUrl); // absent → défaut
    expect(useLlmStore.getState().providerConfig.model).toBe("persisted:3b");
    expect(useLlmStore.getState().providerConfig.resourceProfile).toBe("fast");
  });
});
