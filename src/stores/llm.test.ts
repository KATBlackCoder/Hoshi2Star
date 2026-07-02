import { beforeEach, describe, expect, it } from "vitest";
import { useLlmStore } from "@/stores/llm";
import {
  DEFAULT_BATCH_SIZE,
  DEFAULT_OLLAMA_MODEL,
  DEFAULT_OLLAMA_URL,
} from "@/lib/constants";

const initialState = useLlmStore.getState();

beforeEach(() => {
  useLlmStore.setState(initialState, true);
});

describe("llm store", () => {
  it("providerConfig defaults come from lib/constants (single source of truth)", () => {
    const cfg = useLlmStore.getState().providerConfig;
    expect(cfg.url).toBe(DEFAULT_OLLAMA_URL);
    expect(cfg.model).toBe(DEFAULT_OLLAMA_MODEL);
    expect(cfg.batchSize).toBe(DEFAULT_BATCH_SIZE);
  });

  it("setProviderConfig merges partial updates", () => {
    useLlmStore.getState().setProviderConfig({ model: "custom:1b" });
    const cfg = useLlmStore.getState().providerConfig;
    expect(cfg.model).toBe("custom:1b");
    expect(cfg.url).toBe(DEFAULT_OLLAMA_URL); // untouched
    expect(cfg.batchSize).toBe(DEFAULT_BATCH_SIZE); // untouched
  });

  it("reset clears translation state but keeps providerConfig", () => {
    useLlmStore.getState().setProviderConfig({ model: "custom:1b" });
    useLlmStore.setState({
      isTranslating: true,
      translationProgress: 42,
      error: "boom",
    });
    useLlmStore.getState().reset();
    const s = useLlmStore.getState();
    expect(s.isTranslating).toBe(false);
    expect(s.translationProgress).toBe(-1);
    expect(s.error).toBeNull();
    expect(s.providerConfig.model).toBe("custom:1b");
  });
});
