import { t } from "i18next";

/** Human-readable duration from milliseconds: "850ms", "4.2s", "1m 30s", "3m". */
export function formatDurationMs(ms: number): string {
  if (ms > 0 && ms < 1_000) return `${Math.round(ms)}ms`;
  const seconds = ms / 1_000;
  if (seconds < 60) return `${Number(seconds.toFixed(1))}s`;
  const total = Math.round(seconds);
  const m = Math.floor(total / 60);
  const s = total % 60;
  return s > 0 ? `${m}m ${s}s` : `${m}m`;
}

/** Short label for a game engine identifier. */
export function engineLabel(engine: string): string {
  switch (engine) {
    case "mv_mz":
      return "MV/MZ";
    case "vx_ace":
      return "VX Ace";
    case "wolf":
      return "Wolf RPG";
    case "bakin":
      return "Bakin";
    default:
      return engine;
  }
}

/** Relative date from an ISO timestamp, in the app language ("Today", "3d ago"…). */
export function relativeDate(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const days = Math.floor(diff / 86_400_000);
  if (days === 0) return t("relativeDate.today");
  if (days === 1) return t("relativeDate.yesterday");
  if (days < 30) return t("relativeDate.days", { count: days });
  const months = Math.floor(days / 30);
  if (months < 12) return t("relativeDate.months", { count: months });
  return t("relativeDate.years", { count: Math.floor(months / 12) });
}
