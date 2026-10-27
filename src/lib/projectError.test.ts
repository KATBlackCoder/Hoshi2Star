import { describe, expect, it } from "vitest";
import {
  WOLF_UNREADABLE_ARCHIVE,
  WOLF_UNREADABLE_FILES,
  openProjectErrorKey,
  wolfReadErrorDetail,
  wolfReadErrorKey,
} from "@/lib/projectError";
import fr from "@/locales/fr.json";
import en from "@/locales/en.json";

const ARCHIVE_ERROR = `${WOLF_UNREADABLE_ARCHIVE}: Data.wolf cannot be read: crypt version 350`;
const FILES_ERROR = `${WOLF_UNREADABLE_FILES}: some Wolf RPG maps or common events could not be read (Map001.mps (map parser error))`;

describe("openProjectErrorKey", () => {
  it("shows the dedicated messages for Wolf RPG read failures", () => {
    expect(openProjectErrorKey(ARCHIVE_ERROR)).toBe(
      "projectList.wolfUnreadableArchive",
    );
    expect(openProjectErrorKey(new Error(FILES_ERROR))).toBe(
      "projectList.wolfUnreadableFiles",
    );
    expect(wolfReadErrorKey(ARCHIVE_ERROR)).toBe("wolfUnreadableArchive");
    expect(wolfReadErrorDetail(ARCHIVE_ERROR)).toBe(
      "Data.wolf cannot be read: crypt version 350",
    );
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
      expect(locale.projectList.wolfUnreadableArchive).toBeTruthy();
      expect(locale.projectList.wolfUnreadableFiles).toBeTruthy();
      expect(locale.toasts.wolfUnreadableArchive).toBeTruthy();
      expect(locale.toasts.wolfUnreadableFiles).toBeTruthy();
      expect(locale.exportWolf.archives).toContain("{{archives}}");
      expect(locale.exportWolf.folded).toContain("{{count}}");
      expect(locale.exportWolf.fontAccents).toContain("{{count}}");
      expect(locale.exportWolf.pathWarning).toBeTruthy();
    }
  });
});
