# Journal F4-05 — Wolf RPG Full Integration

**Date:** 2026-06-07  
**Branch:** `feat/f4-05-wolf-integration`  
**Status:** ✅ MERGED to main

---

## What was done

F4-05 wired the Wolf RPG engine (F4-01..04 extractor/injector/decryptor) into
the full Hoshi2Star pipeline: Tauri IPC commands, QA engine parameter, UI icons,
and architecture docs.

### Steps completed

1. **extractor.rs — archive fallback**: `load_mps_files` and `load_dat_files` now
   try `Data/MapData/` and `Data/BasicData/` first; fall back to decrypting `.wolf`
   DXA archives transparently. New helpers: `load_mps_for_stem`, `load_dat_for_stem`,
   `extract_all_wolf` (returns `Vec<(file_name, file_type, Vec<WolfSegment>)>`).

2. **injector.rs — archive-aware inject_all**: First export on encrypted games
   now works — `inject_all` uses `load_mps_for_stem`/`load_dat_for_stem` instead
   of direct `std::fs::read`.

3. **project.rs — open_project Wolf arm**: `guess_wolf_version_from_structure` →
   `extract_all_wolf` → INSERT `source_files` (file_type `wolf_map`/`wolf_database`)
   + segments. `read_wolf_game_title()` helper reads `Game.ini` (Shift-JIS/UTF-8).

4. **export.rs — export_project Wolf**: Detects `file_type.starts_with("wolf_")`,
   delegates to `export_project_wolf` which groups translations by file key and
   calls `wolf_inject_all`.

5. **qa.rs — engine parameter**: `qa::check()` now takes `engine: &str`. Wolf uses
   520 px text box (`LineWidthConfig::wolf_default()`), MV/MZ uses 720 px. All 3
   callers and 15 internal tests updated.

6. **decryptor.rs — PossibleWolfX**: `find_key` emits `PossibleWolfX` when all
   known XOR keys fail — tells users to pre-decrypt with UberWolf.

7. **FileTree.tsx**: `wolf_map` → violet `Map` icon; `wolf_database` → violet
   `Database` icon. `Database` imported from lucide-react.

8. **docs/architecture.md**: Wolf RPG engine layer documented (5 submodules);
   `detector.rs` updated; "not started" note replaced with WolfX limitation.

---

## E2E findings

### Both original test games: WolfX blocked

Both 月咲流ホノカver1.03 and Densyanai Inko ver2.0 use WolfX encryption (v3.5+).
Our `PossibleWolfX` error fires correctly; `extract_files_from_archives` silently
returns empty, project is created with 0 source files. Correct defensive behavior.

### Pre-decrypted 月咲流ホノカver1.03 (hoshi-trans/engine_test/)

- **Databases**: 2 files extracted (CDataBase.dat + DataBase.dat), 379 segments ✅
- **Maps**: 24 `.mps` files found; ALL fail with `wolfrpg-map-parser` panic on
  unknown command code `0x09D20000` — caught by `catch_unwind`, skipped with warning ⚠️
- **UI**: FileTree shows violet Database icons, toolbar badge `wolf`, QA `379/379 ok` ✅

### Known limitations discovered

| Issue | Root cause | Planned fix |
|-------|-----------|-------------|
| WolfX archives not decrypted | WolfX uses hash-based key, not XOR-12 | F5: `decryptor_v3.rs` |
| `.mps` maps panic on cmd `0x09D20000` | `wolfrpg-map-parser 0.6.0` unknown command | Upstream issue or fork |
| `Data.wolf` root archive: `HeaderTooShort` | DXA v8 variant with different layout? | Investigate post-F5 |

---

## Commits on this branch

1. `656b3ab` feat(wolf/extractor): archive fallback + extract_all_wolf + stem loaders
2. `7eb7765` feat(wolf/commands): open_project + export_project Wolf RPG integration
3. `d8125eb` feat(core/qa): engine parameter + Wolf line-width config
4. `c4c398d` feat(ui): FileTree Wolf RPG icons — wolf_map violet Map, wolf_database violet Database
5. `c6fb3c1` docs(architecture): Wolf RPG engine layer + WolfX limitation note
6. `6f9d358` test(e2e): F4-05 Wolf RPG — database extraction + FileTree violet icons verified

---

## Test count

247 unit tests pass (unchanged from F4-04).
