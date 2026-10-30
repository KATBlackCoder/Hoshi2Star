import { afterEach, describe, expect, it, vi } from "vitest";
import i18n from "@/lib/i18n";
import { formatDurationMs, relativeDate } from "@/lib/format";

describe("formatDurationMs (audit F5: one function, unit in the name)", () => {
  it("keeps the compact file-tree format for whole seconds", () => {
    expect(formatDurationMs(0)).toBe("0s");
    expect(formatDurationMs(45_000)).toBe("45s");
    expect(formatDurationMs(90_000)).toBe("1m 30s");
    expect(formatDurationMs(180_000)).toBe("3m");
  });

  it("keeps sub-second and tenth-of-second precision for short calls", () => {
    expect(formatDurationMs(850)).toBe("850ms");
    expect(formatDurationMs(4_240)).toBe("4.2s");
    expect(formatDurationMs(75_300)).toBe("1m 15s");
  });
});

describe("relativeDate (audit F6: translated)", () => {
  const NOW = new Date("2026-10-29T12:00:00Z");
  const daysAgo = (days: number) =>
    new Date(NOW.getTime() - days * 86_400_000).toISOString();

  afterEach(async () => {
    vi.useRealTimers();
    await i18n.changeLanguage("fr");
  });

  it("speaks French when the app is in French", async () => {
    vi.useFakeTimers({ now: NOW });
    await i18n.changeLanguage("fr");
    expect(relativeDate(daysAgo(0))).toBe("Aujourd’hui");
    expect(relativeDate(daysAgo(1))).toBe("Hier");
    expect(relativeDate(daysAgo(3))).toBe("il y a 3 j");
    expect(relativeDate(daysAgo(60))).toBe("il y a 2 mois");
    expect(relativeDate(daysAgo(400))).toBe("il y a 1 an");
    expect(relativeDate(daysAgo(800))).toBe("il y a 2 ans");
  });

  it("keeps the English wording in English", async () => {
    vi.useFakeTimers({ now: NOW });
    await i18n.changeLanguage("en");
    expect(relativeDate(daysAgo(0))).toBe("Today");
    expect(relativeDate(daysAgo(1))).toBe("Yesterday");
    expect(relativeDate(daysAgo(3))).toBe("3d ago");
    expect(relativeDate(daysAgo(60))).toBe("2mo ago");
    expect(relativeDate(daysAgo(800))).toBe("2y ago");
  });
});
