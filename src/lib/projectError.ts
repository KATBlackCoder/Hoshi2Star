/**
 * Map an `open_project` failure to the i18n key of the toast to show.
 *
 * The backend returns a plain error string; the cases worth distinguishing for
 * the user are an unrecognized game engine and a Wolf RPG game whose archives
 * or maps cannot be read. Kept in one place so the call sites (AppToolbar +
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
 * Markers the backend puts at the start of a Wolf RPG read failure:
 * an archive the game reads cannot be read (`WOLF_UNREADABLE_ARCHIVE` in
 * `engines/wolf/sources.rs`), or maps / common events cannot be read
 * (`WOLF_UNREADABLE_FILES` in `engines/wolf/extractor.rs`).
 */
export const WOLF_UNREADABLE_ARCHIVE = "wolf_unreadable_archive";
export const WOLF_UNREADABLE_FILES = "wolf_unreadable_files";

/** i18n key suffix (`projectList.*`, `toasts.*`) of a Wolf RPG read failure. */
export function wolfReadErrorKey(
  err: unknown,
): "wolfUnreadableArchive" | "wolfUnreadableFiles" | null {
  const text = String(err);
  if (text.includes(WOLF_UNREADABLE_ARCHIVE)) return "wolfUnreadableArchive";
  if (text.includes(WOLF_UNREADABLE_FILES)) return "wolfUnreadableFiles";
  return null;
}

/** The backend detail after a Wolf marker (archive or files, reason). */
export function wolfReadErrorDetail(err: unknown): string | undefined {
  const text = String(err);
  for (const marker of [WOLF_UNREADABLE_ARCHIVE, WOLF_UNREADABLE_FILES]) {
    const at = text.indexOf(marker);
    if (at >= 0) return text.slice(at + marker.length).replace(/^:\s*/, "");
  }
  return undefined;
}
