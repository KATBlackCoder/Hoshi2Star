/**
 * Map an `open_project` failure to the i18n key of the toast to show.
 *
 * The backend returns a plain error string; the only case worth distinguishing
 * for the user is an unrecognized game engine. Kept in one place so the three
 * call sites (AppToolbar + ProjectList resume/open) can't drift apart if the
 * backend error text or the i18n keys change.
 *
 * The toast itself (and `t()`) stays in the component — this is pure mapping.
 */
export function openProjectErrorKey(err: unknown): string {
  return String(err).includes("could not identify game engine")
    ? "projectList.engineNotFound"
    : "projectList.openError";
}
