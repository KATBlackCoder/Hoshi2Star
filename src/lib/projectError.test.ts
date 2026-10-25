import { describe, expect, it } from "vitest";
import {
  WOLF_V2_UNSUPPORTED,
  isWolfV2Unsupported,
  openProjectErrorKey,
} from "@/lib/projectError";
import fr from "@/locales/fr.json";
import en from "@/locales/en.json";

const WOLF_ERROR = `${WOLF_V2_UNSUPPORTED}: Wolf RPG v2 maps and common events are not supported yet (Map001.mps)`;

describe("openProjectErrorKey", () => {
  it("shows the dedicated message for a Wolf RPG v2 game", () => {
    expect(openProjectErrorKey(WOLF_ERROR)).toBe("projectList.wolfV2Unsupported");
    expect(isWolfV2Unsupported(new Error(WOLF_ERROR))).toBe(true);
  });

  it("keeps the existing keys for other failures", () => {
    expect(openProjectErrorKey("could not identify game engine")).toBe(
      "projectList.engineNotFound",
    );
    expect(openProjectErrorKey("boom")).toBe("projectList.openError");
    expect(isWolfV2Unsupported("boom")).toBe(false);
  });

  it("has the Wolf v2 messages in both languages", () => {
    expect(fr.projectList.wolfV2Unsupported).toBeTruthy();
    expect(en.projectList.wolfV2Unsupported).toBeTruthy();
    expect(fr.toasts.wolfV2Unsupported).toBeTruthy();
    expect(en.toasts.wolfV2Unsupported).toBeTruthy();
  });
});
