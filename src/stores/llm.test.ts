import { beforeEach, describe, expect, it, vi } from "vitest";

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  listen: vi.fn(),
  toast: {
    success: vi.fn(),
    warning: vi.fn(),
    info: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock("@tauri-apps/api/core", () => ({ invoke: mocks.invoke }));
vi.mock("@tauri-apps/api/event", () => ({ listen: mocks.listen }));
vi.mock("sonner", () => ({ toast: mocks.toast }));

import {
  RUN_TOAST_ID,
  notifyRunCompleted,
  notifyRunError,
  useLlmStore,
  type RunSummary,
} from "@/stores/llm";
import fr from "@/locales/fr.json";
import en from "@/locales/en.json";
import {
  DEFAULT_BATCH_SIZE,
  DEFAULT_OLLAMA_MODEL,
  DEFAULT_OLLAMA_URL,
} from "@/lib/constants";

const initialState = useLlmStore.getState();

type Handler = (event: { payload: unknown }) => void;
let handlers: Map<string, Handler>;

beforeEach(() => {
  useLlmStore.setState(initialState, true);
  vi.clearAllMocks();
  handlers = new Map();
  mocks.listen.mockImplementation(async (name: string, cb: Handler) => {
    handlers.set(name, cb);
    return () => handlers.delete(name);
  });
  mocks.invoke.mockResolvedValue(undefined);
});

const SUMMARY: RunSummary = {
  count: 0,
  translated: 0,
  needsReview: 0,
  failed: 0,
  qaErrorsByType: {},
  cancelled: false,
  repaired: 0,
};

function toastCalls() {
  return Object.values(mocks.toast).reduce(
    (total, fn) => total + fn.mock.calls.length,
    0,
  );
}

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

  describe("R1 — one notice per run", () => {
    it("never listens to per-segment warnings", async () => {
      await useLlmStore.getState().startTranslateAll("p1", 60, 0);
      expect(handlers.has("h2s://llm/placeholder-warning")).toBe(false);
      expect(handlers.has("h2s://llm/completed")).toBe(true);
      expect(handlers.has("h2s://llm/error")).toBe(true);
    });

    it("a provider outage ends the run with a single error toast", async () => {
      await useLlmStore.getState().startTranslateAll("p1", 60, 0);
      handlers.get("h2s://llm/error")!({
        payload: {
          message: "connection refused",
          providerUnavailable: true,
          processed: 72,
          total: 1191,
        },
      });
      // Teardown removed every listener: nothing else can toast for this run.
      expect(handlers.size).toBe(0);
      expect(mocks.toast.error).toHaveBeenCalledTimes(1);
      expect(toastCalls()).toBe(1);
      const [title, options] = mocks.toast.error.mock.calls[0];
      expect(title).toContain("72");
      expect(title).toContain("1191");
      expect(options).toMatchObject({
        id: RUN_TOAST_ID,
        description: "connection refused",
      });
      const state = useLlmStore.getState();
      expect(state.isTranslating).toBe(false);
      expect(state.error).toBe("connection refused");
    });

    it("a finished run shows one summary toast", async () => {
      await useLlmStore.getState().startTranslation(["a", "b"]);
      handlers.get("h2s://llm/completed")!({
        payload: {
          ...SUMMARY,
          count: 1000,
          translated: 950,
          needsReview: 46,
          failed: 4,
          qaErrorsByType: { context_leak: 46 },
        },
      });
      expect(toastCalls()).toBe(1);
      const [title, options] = mocks.toast.warning.mock.calls[0];
      expect(title).toContain("950");
      expect(title).toContain("46");
      expect(options.description).toContain("contexte recopié ×46");
      expect(useLlmStore.getState().isTranslating).toBe(false);
    });

    it("a clean run is a success notice", () => {
      notifyRunCompleted({ ...SUMMARY, count: 3, translated: 3 });
      expect(mocks.toast.success).toHaveBeenCalledTimes(1);
      expect(toastCalls()).toBe(1);
    });

    it("repaired name rows are reported, not hidden", () => {
      notifyRunCompleted({
        ...SUMMARY,
        count: 80,
        translated: 80,
        repaired: 82,
        cancelled: true,
      });
      expect(toastCalls()).toBe(1);
      expect(mocks.toast.info.mock.calls[0][1].description).toContain("82");
    });

    it("a cancelled run says the rest is untouched", () => {
      notifyRunCompleted({ ...SUMMARY, count: 1, translated: 1, cancelled: true });
      expect(mocks.toast.info).toHaveBeenCalledTimes(1);
      expect(mocks.toast.info.mock.calls[0][0]).toContain("Tout traduire");
    });

    it("a non-provider error keeps the generic title", () => {
      notifyRunError({
        message: "database is locked",
        providerUnavailable: false,
        processed: 0,
        total: 10,
      });
      expect(mocks.toast.error.mock.calls[0][0]).toBe("Erreur de traduction");
    });

    it("stop asks the backend once and waits for the run to end", async () => {
      await useLlmStore.getState().startTranslateAll("p1", 60, 0);
      mocks.invoke.mockClear();
      await useLlmStore.getState().cancelTranslation();
      await useLlmStore.getState().cancelTranslation();
      expect(mocks.invoke).toHaveBeenCalledTimes(1);
      expect(mocks.invoke).toHaveBeenCalledWith("cancel_translation");
      expect(useLlmStore.getState().isCancelling).toBe(true);
      expect(useLlmStore.getState().isTranslating).toBe(true);

      handlers.get("h2s://llm/completed")!({
        payload: { ...SUMMARY, count: 8, translated: 8, cancelled: true },
      });
      expect(useLlmStore.getState().isCancelling).toBe(false);
      expect(useLlmStore.getState().isTranslating).toBe(false);
    });

    it("run notices exist in both languages", () => {
      const keys = (value: object): string[] =>
        Object.entries(value).flatMap(([key, child]) =>
          typeof child === "object" && child !== null
            ? keys(child).map((sub) => `${key}.${sub}`)
            : [key],
        );
      expect(keys(en.llmRun).sort()).toEqual(keys(fr.llmRun).sort());
    });
  });
});
