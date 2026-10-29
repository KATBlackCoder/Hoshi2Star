import { describe, expect, it } from "vitest";
import {
  WOLF_ARCHIVE_PRESENT,
  WOLF_NO_DATABASE,
  WOLF_UNREADABLE_FILES,
  openProjectErrorKey,
  wolfReadErrorDetail,
  wolfReadErrorKey,
} from "@/lib/projectError";
import fr from "@/locales/fr.json";
import en from "@/locales/en.json";

const ARCHIVE_ERROR = `${WOLF_ARCHIVE_PRESENT}: Data.wolf, Data/MapData.wolf still in the game folder`;
const DATABASE_ERROR = `${WOLF_NO_DATABASE}: no readable Wolf RPG database in Data/BasicData`;
const FILES_ERROR = `${WOLF_UNREADABLE_FILES}: some Wolf RPG maps or common events could not be read (Map001.mps (map parser error))`;

describe("openProjectErrorKey", () => {
  it("shows the dedicated messages for Wolf RPG read failures", () => {
    expect(openProjectErrorKey(ARCHIVE_ERROR)).toBe(
      "projectList.wolfArchivePresent",
    );
    expect(openProjectErrorKey(DATABASE_ERROR)).toBe(
      "projectList.wolfNoDatabase",
    );
    expect(openProjectErrorKey(new Error(FILES_ERROR))).toBe(
      "projectList.wolfUnreadableFiles",
    );
    expect(wolfReadErrorKey(ARCHIVE_ERROR)).toBe("wolfArchivePresent");
    expect(wolfReadErrorDetail(ARCHIVE_ERROR)).toBe(
      "Data.wolf, Data/MapData.wolf still in the game folder",
    );
    expect(wolfReadErrorDetail(DATABASE_ERROR)).toBe(
      "no readable Wolf RPG database in Data/BasicData",
    );
  });

  it("no longer knows the removed archive marker", () => {
    const removed = "wolf_unreadable_archive: Data.wolf cannot be read";
    expect(wolfReadErrorKey(removed)).toBeNull();
    expect(openProjectErrorKey(removed)).toBe("projectList.openError");
  });

  it("keeps the existing keys for other failures", () => {
    expect(openProjectErrorKey("could not identify game engine")).toBe(
      "projectList.engineNotFound",
    );
    expect(openProjectErrorKey("boom")).toBe("projectList.openError");
    expect(wolfReadErrorKey("boom")).toBeNull();
    expect(wolfReadErrorDetail("boom")).toBeUndefined();
  });

  it("has the Wolf messages in both languages", () => {
    for (const locale of [fr, en]) {
      for (const key of [
        "wolfArchivePresent",
        "wolfNoDatabase",
        "wolfUnreadableFiles",
      ] as const) {
        expect(locale.projectList[key]).toBeTruthy();
        expect(locale.toasts[key]).toBeTruthy();
      }
      expect(locale.projectList).not.toHaveProperty("wolfUnreadableArchive");
      expect(locale.toasts).not.toHaveProperty("wolfUnreadableArchive");
      expect(locale.projectList.engineNotFound).toContain("UberWolf");
      expect(locale.exportWolf).not.toHaveProperty("archives");
      expect(locale.exportWolf.folded).toContain("{{count}}");
      expect(locale.exportWolf.fontAccents).toContain("{{count}}");
      expect(locale.exportWolf.pathWarning).toBeTruthy();
    }
  });
});
