/**
 * Game engines as stored in `Project.engine`, and what each one supports.
 * One predicate per capability: each evolves on its own (plan DD, F3).
 */

/** The engines the backend stores (`engines/detector.rs`, `Engine::db_str`). */
export type Engine = "mv_mz" | "vx_ace" | "wolf";

type EngineName = string | null | undefined;

/** Optional text categories ("Text coverage"): RPG Maker MV/MZ. */
export function hasTextCoverage(engine: EngineName): boolean {
  return engine === "mv_mz";
}

/** Text fitting at export: RPG Maker MZ and Wolf RPG messages. */
export function hasTextFit(engine: EngineName): boolean {
  return engine === "mv_mz" || engine === "wolf";
}

/** The in-game preview of a fitted text: RPG Maker MZ only. */
export function hasTextFitPreview(engine: EngineName): boolean {
  return engine === "mv_mz";
}

/** The translation pilot (developer tool): RPG Maker MV/MZ. */
export function hasPilot(engine: EngineName): boolean {
  return engine === "mv_mz";
}

/** The debug export (developer tool): RPG Maker MV/MZ and Wolf RPG. */
export function hasDebugExport(engine: EngineName): boolean {
  return engine === "mv_mz" || engine === "wolf";
}
