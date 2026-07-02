import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";
import { load } from "@tauri-apps/plugin-store";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";

// ---------------------------------------------------------------------------
// State machine
// ---------------------------------------------------------------------------
//
//   idle ──check──▶ available ──yes──▶ downloading ──▶ ready ──▶ (relaunch)
//     ▲                │  ▲                                │
//     └── no update ───┘  └── no ──▶ dismissed ──reopen────┘
//
// `dismissed` persists the version so the same release is not re-proposed on the
// next launch; the toolbar badge lets the user reopen the decision.

export type UpdaterStatus =
  | "idle" // no update, not checked, or install not self-updatable
  | "checking"
  | "available" // update found, decision pending
  | "dismissed" // user chose "not now" — badge reopens the choice
  | "downloading"
  | "ready" // downloaded + installed, awaiting relaunch
  | "error";

const STORE_FILE = "updater.json";
const DISMISSED_KEY = "dismissed_version";

interface UpdaterState {
  status: UpdaterStatus;
  version: string | null;
  notes: string | null;
  downloaded: number;
  contentLength: number;
  /** Plugin handle for the pending update (never persisted). */
  update: Update | null;

  checkForUpdate: () => Promise<void>;
  startDownload: () => Promise<void>;
  dismiss: () => Promise<void>;
  reopen: () => void;
  /** After a `ready` install, close without restarting — it applies next launch. */
  postpone: () => void;
  applyAndRestart: () => Promise<void>;
}

export const useUpdaterStore = create<UpdaterState>()((set, get) => ({
  status: "idle",
  version: null,
  notes: null,
  downloaded: 0,
  contentLength: 0,
  update: null,

  checkForUpdate: async () => {
    // Only self-updatable installs (Windows, or Linux AppImage). A deb/rpm
    // install or a dev build reports false → we never propose an update.
    const supported = await invoke<boolean>("updater_supported").catch(
      () => false,
    );
    if (!supported) return;

    set({ status: "checking" });

    let update: Update | null = null;
    try {
      update = await check();
    } catch (e) {
      // Offline, missing manifest, or dev endpoint — stay silent.
      console.warn("[h2s] update check failed:", e);
      set({ status: "idle" });
      return;
    }

    if (!update) {
      set({ status: "idle" });
      return;
    }

    // Respect a previously dismissed version for this same release.
    const store = await load(STORE_FILE, { defaults: {}, autoSave: false });
    const dismissed = await store.get<string>(DISMISSED_KEY);

    set({
      update,
      version: update.version,
      notes: update.body ?? null,
      status: dismissed === update.version ? "dismissed" : "available",
    });
  },

  startDownload: async () => {
    const { update } = get();
    if (!update) return;

    set({ status: "downloading", downloaded: 0, contentLength: 0 });
    try {
      await update.downloadAndInstall((event) => {
        switch (event.event) {
          case "Started":
            set({ contentLength: event.data.contentLength ?? 0 });
            break;
          case "Progress":
            set((s) => ({ downloaded: s.downloaded + event.data.chunkLength }));
            break;
          case "Finished":
            break;
        }
      });
      set({ status: "ready" });
    } catch (e) {
      console.error("[h2s] update download failed:", e);
      set({ status: "error" });
    }
  },

  dismiss: async () => {
    const { version } = get();
    if (version) {
      const store = await load(STORE_FILE, { defaults: {}, autoSave: false });
      await store.set(DISMISSED_KEY, version);
      await store.save();
    }
    set({ status: "dismissed" });
  },

  reopen: () => {
    if (get().version) set({ status: "available" });
  },

  postpone: () => set({ status: "idle" }),

  applyAndRestart: async () => {
    await relaunch();
  },
}));

// ---------------------------------------------------------------------------
// Selectors
// ---------------------------------------------------------------------------

export const useUpdaterStatus = () => useUpdaterStore((s) => s.status);

/** Download progress in [0, 100], or 0 when the total size is unknown. */
export const useUpdaterProgress = () =>
  useUpdaterStore((s) =>
    s.contentLength > 0
      ? Math.min(100, Math.round((s.downloaded / s.contentLength) * 100))
      : 0,
  );
