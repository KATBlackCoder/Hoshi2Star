// Global Vitest setup — loaded before every test file (vitest.config.ts).
import "@testing-library/jest-dom/vitest";
import { afterEach } from "vitest";
import { cleanup } from "@testing-library/react";
import { clearMocks } from "@tauri-apps/api/mocks";
import i18n from "@/lib/i18n";

// Deterministic locale for text assertions (the app defaults to fr).
void i18n.changeLanguage("fr");

// mockIPC covers invoke(), but the event plugin's unlisten() goes through
// this internal object directly — stub it so component unmounts don't reject.
// Plain assignment (writable): mockIPC's mockInternals() re-assigns it with
// `?? {}` and would throw on a read-only property.
(
  window as unknown as Record<string, unknown>
).__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener: () => {} };

afterEach(async () => {
  cleanup();
  // Unmount effects tear listeners down via `unlisten.then(fn => fn())`:
  // flush those microtasks BEFORE clearMocks() deletes the IPC handlers
  // (mocks.js also removes unregisterListener), or teardown rejects.
  await new Promise((resolve) => setTimeout(resolve, 0));
  clearMocks();
});
