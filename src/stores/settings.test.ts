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
      ollamaUrl: "https://pod.example:11434",
      ollamaModel: "custom:7b",
      batchSize: 5,
    };
    await useSettingsStore.getState().saveSettings(draft);

    expect(useSettingsStore.getState().settings).toEqual(draft);
    expect(storeData.get("ollama_model")).toBe("custom:7b");

    // The llm store must reflect the saved settings immediately.
    const cfg = useLlmStore.getState().providerConfig;
    expect(cfg.url).toBe("https://pod.example:11434");
    expect(cfg.model).toBe("custom:7b");
    expect(cfg.batchSize).toBe(5);
  });

  it("loadSettings restores persisted values over defaults", async () => {
    storeData.set("ollama_model", "persisted:3b");
    storeData.set("batch_size", 7);
    await useSettingsStore.getState().loadSettings();

    const s = useSettingsStore.getState().settings;
    expect(s.ollamaModel).toBe("persisted:3b");
    expect(s.batchSize).toBe(7);
    expect(s.ollamaUrl).toBe(DEFAULT_SETTINGS.ollamaUrl); // absent → défaut
    expect(useLlmStore.getState().providerConfig.model).toBe("persisted:3b");
  });
});
