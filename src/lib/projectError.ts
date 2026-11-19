/**
 * Map an `open_project` failure to the i18n key of the toast to show.
 *
 * The backend returns a plain error string; the cases worth distinguishing for
 * the user are an unrecognized game engine and a Wolf RPG game that cannot be
 * read (text archive still present, no readable database, unreadable maps). Kept in one place so the call sites (AppToolbar +
 * ProjectList resume/open, export) can't drift apart if the backend error text
 * or the i18n keys change.
 *
 * The toast itself (and `t()`) stays in the component — this is pure mapping.
 */
export function openProjectErrorKey(err: unknown): string {
  const wolf = wolfReadErrorKey(err);
  if (wolf) return `projectList.${wolf}`;
  return String(err).includes("could not identify game engine")
    ? "projectList.engineNotFound"
    : "projectList.openError";
}

/**
 * Markers the backend puts at the start of a Wolf RPG read failure (plan UW):
 * a `.wolf` archive holding text is still in the game folder
 * (`WOLF_ARCHIVE_PRESENT` in `engines/wolf/sources.rs`), no database declares
 * the encoding (`WOLF_NO_DATABASE` in `engines/detector.rs`), or maps / common
 * events cannot be read (`WOLF_UNREADABLE_FILES` in `engines/wolf/extractor.rs`).
 */
export const WOLF_ARCHIVE_PRESENT = "wolf_archive_present";
export const WOLF_NO_DATABASE = "wolf_no_database";
export const WOLF_UNREADABLE_FILES = "wolf_unreadable_files";
/**
 * A Wolf RPG game still packed in its archives (`WOLF_PACKED` in
 * `engines/wolf/sources.rs`, plan UWF): the decryption is offered.
 */
export const WOLF_PACKED = "wolf_packed";

const WOLF_READ_ERRORS = [
  [WOLF_ARCHIVE_PRESENT, "wolfArchivePresent"],
  [WOLF_NO_DATABASE, "wolfNoDatabase"],
  [WOLF_UNREADABLE_FILES, "wolfUnreadableFiles"],
  [WOLF_PACKED, "wolfPacked"],
] as const;

/** Key suffixes of the Wolf RPG read failures. */
export type WolfReadErrorKey = (typeof WOLF_READ_ERRORS)[number][1];

/** i18n key suffix (`projectList.*`, `toasts.*`) of a Wolf RPG read failure. */
export function wolfReadErrorKey(err: unknown): WolfReadErrorKey | null {
  const text = String(err);
  const found = WOLF_READ_ERRORS.find(([marker]) => text.includes(marker));
  return found ? found[1] : null;
}

/** The backend detail after a Wolf marker (archives, database or files). */
export function wolfReadErrorDetail(err: unknown): string | undefined {
  const text = String(err);
  for (const [marker] of WOLF_READ_ERRORS) {
    const at = text.indexOf(marker);
    if (at >= 0) return text.slice(at + marker.length).replace(/^:\s*/, "");
  }
  return undefined;
}

/**
 * Kinds of `unpack_wolf_game` failure: each backend message starts with
 * `wolf_unpack_<kind>` (`UnpackError` in `engines/wolf/unpack.rs`,
 * `WOLF_UNPACK_BUSY` in `commands/wolf_unpack.rs`), mapped to the key suffix
 * of `wolfUnpack.errors.*`.
 */
const WOLF_UNPACK_ERRORS = {
  cancelled: "cancelled",
  busy: "busy",
  no_launcher: "noLauncher",
  not_writable: "notWritable",
  data_folder_present: "dataFolderPresent",
  destination_exists: "destinationExists",
  pro_unsupported: "proUnsupported",
  extraction_failed: "extractionFailed",
  protection_failed: "protectionFailed",
  unprotect_failed: "unprotectFailed",
  not_readable: "notReadable",
  move_failed: "moveFailed",
} as const;

const WOLF_UNPACK_MARKER = /wolf_unpack_([a-z_]+?):\s*/;

/** i18n key of a decryption failure, `wolfUnpack.errors.generic` if unknown. */
export function wolfUnpackErrorKey(err: unknown): string {
  const kind = WOLF_UNPACK_MARKER.exec(String(err))?.[1];
  const key =
    kind && Object.prototype.hasOwnProperty.call(WOLF_UNPACK_ERRORS, kind)
      ? WOLF_UNPACK_ERRORS[kind as keyof typeof WOLF_UNPACK_ERRORS]
      : "generic";
  return `wolfUnpack.errors.${key}`;
}

/** True when the decryption failed because the user stopped it. */
export function isWolfUnpackCancelled(err: unknown): boolean {
  return WOLF_UNPACK_MARKER.exec(String(err))?.[1] === "cancelled";
}

/** The backend detail after the decryption marker, or the whole message. */
export function wolfUnpackErrorDetail(err: unknown): string {
  const text = String(err).replace(/^Error:\s*/, "");
  const match = WOLF_UNPACK_MARKER.exec(text);
  return match ? text.slice(match.index + match[0].length) : text;
}
