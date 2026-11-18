import { invoke } from "@tauri-apps/api/core";

/** The application itself. */
export const appApi = {
  updaterSupported: () => invoke<boolean>("updater_supported"),
};
