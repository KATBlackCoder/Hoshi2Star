/** Human-readable duration from seconds: "1m 30s", "45s", "3m". */
export function formatDuration(seconds: number): string {
  if (seconds < 60) return `${seconds}s`;
  const m = Math.floor(seconds / 60);
  const s = seconds % 60;
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

/** Relative date string from an ISO timestamp ("Today", "3d ago", etc.). */
export function relativeDate(iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  const days = Math.floor(diff / 86_400_000);
  if (days === 0) return "Today";
  if (days === 1) return "Yesterday";
  if (days < 30) return `${days}d ago`;
  const months = Math.floor(days / 30);
  if (months < 12) return `${months}mo ago`;
  return `${Math.floor(months / 12)}y ago`;
}
