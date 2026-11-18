import { describe, expect, it } from "vitest";
import {
  WOLF_PACKED,
  openProjectErrorKey,
  wolfReadErrorKey,
  wolfUnpackErrorDetail,
  wolfUnpackErrorKey,
} from "@/lib/projectError";
import fr from "@/locales/fr.json";
import en from "@/locales/en.json";

const PACKED_ERROR = `${WOLF_PACKED}: Data.wolf: this Wolf RPG game is still packed in .wolf archives; decrypt it before opening it`;

/** Every string value of a locale tree, with its dotted key. */
function strings(tree: unknown, prefix = ""): [string, string][] {
  if (typeof tree === "string") return [[prefix, tree]];
  return Object.entries(tree as Record<string, unknown>).flatMap(
    ([key, value]) => strings(value, prefix ? `${prefix}.${key}` : key),
  );
}

describe("Wolf RPG decryption errors (plan UWF)", () => {
  it("recognises a packed game at open", () => {
    expect(wolfReadErrorKey(PACKED_ERROR)).toBe("wolfPacked");
    expect(openProjectErrorKey(PACKED_ERROR)).toBe("projectList.wolfPacked");
    expect(openProjectErrorKey(PACKED_ERROR)).not.toBe(
      "projectList.engineNotFound",
    );
  });

  it("maps each decryption failure to its message and keeps the detail", () => {
    const error =
      "Error: wolf_unpack_data_folder_present: Data.wolf and a non-empty Data/ are both in the game folder";
    expect(wolfUnpackErrorKey(error)).toBe(
      "wolfUnpack.errors.dataFolderPresent",
    );
    expect(wolfUnpackErrorDetail(error)).toBe(
      "Data.wolf and a non-empty Data/ are both in the game folder",
    );
    expect(wolfUnpackErrorKey("wolf_unpack_busy: already running")).toBe(
      "wolfUnpack.errors.busy",
    );
    expect(wolfUnpackErrorKey("wolf_unpack_constructor: odd")).toBe(
      "wolfUnpack.errors.generic",
    );
    expect(wolfUnpackErrorKey("worker failed")).toBe(
      "wolfUnpack.errors.generic",
    );
    expect(wolfUnpackErrorDetail("worker failed")).toBe("worker failed");
  });

  it("has every decryption message in both languages", () => {
    const kinds = [
      "cancelled",
      "busy",
      "noLauncher",
      "notWritable",
      "dataFolderPresent",
      "destinationExists",
      "proUnsupported",
      "extractionFailed",
      "protectionFailed",
      "unprotectFailed",
      "notReadable",
      "moveFailed",
      "generic",
    ];
    for (const locale of [fr, en]) {
      for (const kind of kinds) {
        expect(
          locale.wolfUnpack.errors[
            kind as keyof typeof locale.wolfUnpack.errors
          ],
        ).toBeTruthy();
      }
      expect(locale.projectList.wolfPacked).toBeTruthy();
      expect(locale.toasts.wolfPacked).toBeTruthy();
    }
    expect(strings(fr.wolfUnpack).map(([key]) => key)).toEqual(
      strings(en.wolfUnpack).map(([key]) => key),
    );
  });

  it("mentions UberWolf only as a fallback", () => {
    const fallbacks = [
      "projectList.engineNotFound",
      "projectList.wolfArchivePresent",
      "projectList.wolfNoDatabase",
      "projectList.wolfPacked",
      "wolfUnpack.errors.dataFolderPresent",
      "wolfUnpack.errors.proUnsupported",
    ];
    for (const locale of [fr, en]) {
      const mentions = strings(locale)
        .filter(([, text]) => text.includes("UberWolf"))
        .map(([key]) => key);
      expect(mentions.sort()).toEqual([...fallbacks].sort());
    }
  });
});
