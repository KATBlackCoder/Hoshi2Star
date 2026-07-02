import { defineConfig } from "vitest/config";
import path from "path";

// Kept separate from vite.config.ts (which carries Tauri-specific settings).
export default defineConfig({
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
    },
  },
  test: {
    environment: "jsdom",
    setupFiles: ["./src/test/setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
  },
});
