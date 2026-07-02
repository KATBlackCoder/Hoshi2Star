import { beforeEach, describe, expect, it, vi } from "vitest";

// ---------------------------------------------------------------------------
// Mocks for the Tauri plugins the updater store depends on.
// ---------------------------------------------------------------------------

let supported = true;
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(async (cmd: string) =>
    cmd === "updater_supported" ? supported : undefined,
  ),
}));

// In-memory stand-in for @tauri-apps/plugin-store.
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

// Controllable check() result.
type DownloadCb = (e: {
  event: "Started" | "Progress" | "Finished";
  data?: { contentLength?: number; chunkLength?: number };
}) => void;
let nextUpdate: {
  version: string;
  body?: string;
  downloadAndInstall: (cb: DownloadCb) => Promise<void>;
} | null = null;
vi.mock("@tauri-apps/plugin-updater", () => ({
  check: vi.fn(async () => nextUpdate),
}));

const relaunch = vi.fn(async () => {});
vi.mock("@tauri-apps/plugin-process", () => ({
  relaunch: () => relaunch(),
}));

import { check } from "@tauri-apps/plugin-updater";
import { useUpdaterStore } from "@/stores/updater";

const initial = useUpdaterStore.getState();

/** Fake Update whose download replays Started → Progress×2 → Finished. */
function makeUpdate(version: string, body?: string, opts?: { fail?: boolean }) {
  return {
    version,
    body,
    downloadAndInstall: async (cb: DownloadCb) => {
      cb({ event: "Started", data: { contentLength: 100 } });
      cb({ event: "Progress", data: { chunkLength: 40 } });
      cb({ event: "Progress", data: { chunkLength: 60 } });
      if (opts?.fail) throw new Error("boom");
      cb({ event: "Finished" });
    },
  };
}

beforeEach(() => {
  storeData.clear();
  supported = true;
  nextUpdate = null;
  relaunch.mockClear();
  useUpdaterStore.setState(initial, true);
});

describe("updater store", () => {
  it("an available update → status 'available' with version + notes", async () => {
    nextUpdate = makeUpdate("0.4.4", "notes here");
    await useUpdaterStore.getState().checkForUpdate();

    const s = useUpdaterStore.getState();
    expect(s.status).toBe("available");
    expect(s.version).toBe("0.4.4");
    expect(s.notes).toBe("notes here");
  });

  it("no update → stays idle", async () => {
    nextUpdate = null;
    await useUpdaterStore.getState().checkForUpdate();
    expect(useUpdaterStore.getState().status).toBe("idle");
  });

  it("does nothing when the install is not self-updatable (deb/rpm/dev)", async () => {
    supported = false;
    nextUpdate = makeUpdate("0.4.4");
    await useUpdaterStore.getState().checkForUpdate();

    expect(useUpdaterStore.getState().status).toBe("idle");
    expect(useUpdaterStore.getState().version).toBeNull();
  });

  it("dismiss persists the version; reopen restores the choice", async () => {
    nextUpdate = makeUpdate("0.4.4");
    await useUpdaterStore.getState().checkForUpdate();

    await useUpdaterStore.getState().dismiss();
    expect(useUpdaterStore.getState().status).toBe("dismissed");
    expect(storeData.get("dismissed_version")).toBe("0.4.4");

    useUpdaterStore.getState().reopen();
    expect(useUpdaterStore.getState().status).toBe("available");
  });

  it("a previously dismissed version is not re-proposed on next check", async () => {
    storeData.set("dismissed_version", "0.4.4");
    nextUpdate = makeUpdate("0.4.4");
    await useUpdaterStore.getState().checkForUpdate();
    expect(useUpdaterStore.getState().status).toBe("dismissed");
  });

  it("a newer version overrides a stale dismissal", async () => {
    storeData.set("dismissed_version", "0.4.4");
    nextUpdate = makeUpdate("0.4.5");
    await useUpdaterStore.getState().checkForUpdate();
    expect(useUpdaterStore.getState().status).toBe("available");
  });

  it("download accumulates progress then becomes ready", async () => {
    nextUpdate = makeUpdate("0.4.4");
    await useUpdaterStore.getState().checkForUpdate();
    await useUpdaterStore.getState().startDownload();

    const s = useUpdaterStore.getState();
    expect(s.contentLength).toBe(100);
    expect(s.downloaded).toBe(100);
    expect(s.status).toBe("ready");
  });

  it("postpone from ready closes without re-proposing (idle)", async () => {
    nextUpdate = makeUpdate("0.4.4");
    await useUpdaterStore.getState().checkForUpdate();
    await useUpdaterStore.getState().startDownload();
    useUpdaterStore.getState().postpone();
    expect(useUpdaterStore.getState().status).toBe("idle");
  });

  it("a failing download → status 'error'", async () => {
    nextUpdate = makeUpdate("0.4.4", undefined, { fail: true });
    await useUpdaterStore.getState().checkForUpdate();
    await useUpdaterStore.getState().startDownload();
    expect(useUpdaterStore.getState().status).toBe("error");
  });

  it("applyAndRestart calls relaunch()", async () => {
    await useUpdaterStore.getState().applyAndRestart();
    expect(relaunch).toHaveBeenCalledOnce();
  });

  it("stays silent (idle) when the check throws, e.g. offline", async () => {
    vi.mocked(check).mockRejectedValueOnce(new Error("offline"));
    await useUpdaterStore.getState().checkForUpdate();
    expect(useUpdaterStore.getState().status).toBe("idle");
  });
});
