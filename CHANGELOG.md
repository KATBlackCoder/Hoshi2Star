# Changelog

All notable changes to Hoshi2Star will be documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com) — [Semantic Versioning](https://semver.org).

## [Unreleased]

## [0.4.7] — 2026-07-04

### Fixed
- Fix Wolf RPG v3.5 database exports being silently corrupted — `Data/BasicData/DataBase.dat` and `CDataBase.dat` on v3.5 games (e.g. Inko) are LZ4-compressed (version byte `0xC4`), but `serialize_dat` always wrote the plain (uncompressed) layout while preserving the `0xC4` header, producing a file whose header claims LZ4 but whose body is raw bytes; the Wolf RPG Editor runtime then refuses to load the game ("database file may be corrupted or an old version"). `serialize_dat` now recompresses via a new `dat_parser::compress_lz4_dat` (symmetric to the existing decompression path) whenever the original header selected the LZ4 version; new regression test `test_inject_dat_lz4_v3_recompresses` locks the fix. Games already exported with the broken version need a clean (untranslated) copy of these two files restored before re-exporting — Hoshi2Star cannot reconstruct them from translated text alone

## [0.4.6] — 2026-07-04

### Added
- Add project-wide segment search (concordance search) — a new search bar in the FILES panel searches every file of the project at once (Enter to run, min 2 chars, scope source/target/both), unlike the grid search which only filters the open file; results replace the grid, grouped by file with separator headers, showing key, source, target, status badge and QA score, every match is loaded in successive 500-row batches (the first batch renders immediately, the rest append in the background with a discreet progress indicator — same pattern as the grid's full-file loading, no truncation); hits are selectable across files (global select-all, per-file tri-state checkbox on each file header, per-segment — all using the shadcn/Radix Checkbox) and "Translate N lines" reuses the existing explicit-ids batch path (cross-file batches confirmed working end-to-end: one LLM batch, project-level stats refresh, results re-run on completion); file groups are collapsible (chevron on each header, selection preserved while collapsed) and a sticky replica of the current file's header stays pinned at the top of the scroll until the next file takes over; clicking a hit navigates to its file in the normal grid with the segment highlighted, unless at least one hit is checked, in which case clicking a row toggles its selection instead. New Rust command `search_segments` (SQL LIKE with escaped wildcards, project-scoped join to `source_files`, batched offset loading), new `search` Zustand store, `ProjectSearchBar` + `GlobalSearchResults` components; 7 Rust + 18 Vitest tests

### Changed
- Migrate linting to ESLint 10 flat config — new `eslint.config.js` (`@eslint/js` + `typescript-eslint` recommended, react-hooks flat `recommended-latest`, react-refresh vite preset), `pnpm lint` script drops the removed `--ext` flag; shadcn-owned `src/components/ui/` is excluded and the pre-existing `set-state-in-effect` / `only-export-components` findings are downgraded to warnings pending a separate refactor (the previous setup had no config file at all, so `pnpm lint` could not even start)

### Fixed
- Fix name box names (`\n<ハルカ>`) never being translated — the tokenizer's Groupe G turned the whole Yanfly name box code into one opaque token, so the LLM never saw the name and it stayed in Japanese in the exported game; tokenization is now split into two passes: a dedicated first pass tokenizes only the structure (`\n<` and `>` become a pair of tokens, the name stays inline and translatable, glossary terms apply), then the engine regex tokenizes the rest including codes nested inside the name box (`\n<\C[6]ハルカ>`); a new structural check in `restore` rejects LLM responses that reorder the pair tokens (`TokenizerError::BrokenNameBox`) so they go through the normal retry path, and pure name-box segments are now correctly extracted as translatable
- Fix TM saving a duplicate row on every segment save — `idx_tm_hash_lang` was non-unique and `tm::insert` minted a fresh UUID per call, so its `INSERT OR REPLACE` never conflicted; migration `0005` dedups existing rows (newest per `(source_hash, lang_pair)` wins) and recreates the index as `UNIQUE`, and `insert` now upserts via `ON CONFLICT DO UPDATE` (row id preserved), so fuzzy TM suggestions no longer show the same source multiple times
- Fix live QA check ignoring the glossary and the project engine — `qa_check_segment` received no project context and passed an empty glossary to the QA engine, so `GlossaryMismatch` (−15) never fired while typing and Wolf projects were width-checked against the MV/MZ 720 px box instead of 520 px; the command now takes a `projectId`, loads glossary terms via the same `relevant_terms` helper as the batch pipeline and resolves the engine from `projects.engine` (explicit param > DB > `mv_mz`), with `QAPanel` sending the active project id

### Changed
- Bump CI release actions to their Node 24 majors (`actions/checkout@v7`, `actions/setup-node@v6`) to clear the GitHub Actions Node 20 deprecation warning; this release also serves as the first end-to-end validation of the in-app auto-updater (0.4.4 → 0.4.5)

## [0.4.4] — 2026-07-02

### Added
- Add in-app auto-updates via `tauri-plugin-updater` + `tauri-plugin-process` (ADR-007) — on startup the app checks GitHub Releases (`latest.json`) for a newer signed build and, if found, shows an `UpdateDialog` (release notes + Yes/No); **Yes** downloads with a live progress bar then offers "Restart now / Later"; **No** dismisses and drops a toolbar badge (`UpdateBadge`) to reopen the choice, persisting the dismissed version so the same update is not re-proposed every launch (a newer version overrides the dismissal). New Zustand `updater` store (state machine `idle→checking→available→downloading→ready`, plus `dismissed`/`error`), new `updater_supported` Tauri command gating the UI to auto-updatable installs only (Windows NSIS + Linux AppImage — deb/rpm and dev builds show no update UI), signed artifacts (`bundle.createUpdaterArtifacts`, minisign pubkey in config, private key via CI secret), Windows `installMode: passive`; 11 Vitest cases lock the store's state machine with the plugin calls mocked

## [0.4.3] — 2026-07-02

### Added
- Add automated test suite (audit remediation Phase 7) — Vitest front-end tests (`vitest.config.ts`, `src/test/setup.ts`, `pnpm test`/`pnpm test:watch` scripts) covering Zustand stores and the components/hooks that lock the Phase 3/5 fixes, plus Rust end-to-end integration tests (`src-tauri/tests/e2e_project_flow.rs`) driving `open_project → update_segment → export_project` through the real command layer for MV and Wolf CommonEvent fixtures (the latter locks the Phase 1 Common Events reinjection); `pnpm test` added to the mandatory verification gate
- Add compile-time embedded LLM prompt templates — `prompts/translate/default.toml` and `prompts/glossary/default.toml` replace hardcoded Rust strings; new `llm/prompts.rs` module exposes `translate_for()`, `glossary_for()`, `lang_code_to_name()`, and `PromptTemplate::render()`; per-language files (`fr.toml`, `es.toml`, …) can be added without touching provider or glossary code; LLM now receives full language names (`"Japanese"` / `"English"`) instead of BCP-47 codes
- Add `\FS[N]` font-size prefix support for RPG Maker MV/MZ export — `scan_font_status` now returns `engine` in `FontScanResult`; `apply_font_prefix` dispatches `\f[N]` (Wolf) or `\FS[N]` (MV/MZ) based on engine; font dialog shows the correct code and a note that MV requires a message plugin (VisuStella/Yanfly)
- Add `strip_font_prefixes(projectId)` Tauri command — strips any `\f[N]` or `\FS[N]` prefix from all segments in a project DB (cleanup for pre-fix exports); called automatically on Skip when `existingFontCount > 0`
- Add `isExporting` loading state to Export All toolbar button — button disabled with `Loader2` spinner during export, restored on success or error

### Fixed
- Fix "translated" meaning two different things across the UI (audit issue #8, Option 2) — `SourceFile.translatedCount` now counts `status = 'translated'` strictly (it previously counted any non-empty target, so a file full of unreviewed LLM fallbacks looked complete in the FileTree while the toolbar said otherwise); new `SourceFile.needsReviewCount` and `ProjectStats.reviewedCount` fields make the four statuses sum to the total, frozen by a DB-backed counter-semantics test
- Fix half-width katakana counted as full-width in QA line measurement — `is_fullwidth` mapped the whole U+FF00–FFEF block to 2 units; now only the real full-width sub-ranges (U+FF00–FF60, U+FFE0–FFE6) count as 2, so half-width kana text no longer triggers false `LineTooLong` errors
- Fix QA report "X segments with errors / Y checked" always showing X == Y — `collect_qa_details` now returns the real number of segments examined and the HTML header uses it as denominator
- Fix dead glossary check in the QA report — `export_qa_report` now loads the project glossary and passes it to the QA engine, so `GlossaryMismatch` errors actually appear in the report (its stat, filter and badge already existed but could never fire)
- Fix SegmentGrid silently truncating files at 5000 segments — `loadSegments` now fetches every page (2000 per call) until the backend's real `total` is reached, with a stale-load guard so switching files mid-load cannot mix results; footer and status counts now cover the whole file
- Fix batch selection translating the wrong segments after filtering — row selection is now keyed by segment id (`getRowId`) instead of row index, and is reset whenever the QA filter or search query changes (checked-then-hidden rows can no longer stay silently selected)
- Fix unhandled promise rejections in the editor UI — a failing `get_project_stats` no longer wipes every project card's stats bar (`Promise.allSettled`), a failed segment save now shows an error toast (new `segmentGrid.saveError` i18n key), and the QA/TM export dialogs guard the `save()` dialog call
- Fix Wolf binary parsers panicking or aborting the whole process on malformed input — 11 hardened sites across `decrypt/legacy_xor.rs` (Huffman/LZ output sizes capped at 1 GiB, TOC `name_offset` bounds-checked, v8 Huffman slices guarded, all attacker-controlled offset math moved to `checked_add`/`try_from`), `v3_format/map.rs` (tile size `width*height*layers*4` computed with `checked_mul`, new `TileSizeOverflow` error, allocation bounded by input), and `dat_parser.rs` (`read_bytes` rejects forged lengths before allocating; `with_capacity` counts bounded by remaining input — a forged `.project` type_count used to abort the app with a 309 GB allocation); each site covered by a malformed-input regression test
- Fix Wolf RPG Common Events translations silently dropped from the export ZIP — the injector had no `CommonEvent.dat` path and `export.rs` bucketed `CommonEvents/*` keys per event name (all mapping to one physical file); added `inject_common_events` (v2.x binary splice + v3.5 LZ4 parse/dump/recompress) plus a `"CommonEvents"` dispatch arm, and `injection_bucket()` now collapses every common event into the single `CommonEvent.dat` bucket so all translations survive
- Fix font-size prefix being permanently written to DB (`persist_font_size`) — prefix is now applied **in-memory at export time only** (never touches `target_text` in the DB); segments in the editor no longer show `\f[N]`/`\FS[N]` prefixes after an export
- Fix Export All font dialog shown for all engines — dialog now only appears for `wolf` and `mv_mz` projects; VX Ace and Bakin skip straight to export
- Fix `scan_font_status` engine mismatch — scan now detects any prefix type (`\f[N]` or `\FS[N]`) regardless of project engine, so cross-engine leftovers (e.g. Wolf prefix on a MV/MZ project) are correctly counted and stripped
- Fix missing spinner during Skip — `setIsExporting(true)` now fires before `await strip_font_prefixes`, so the toolbar button shows its loading state for the entire skip + export sequence
- Fix stale `%` progress badge in toolbar — `SegmentGrid` now reloads `activeProjectStats` on `h2s://llm/completed` alongside segments and source files

### Changed
- Centralize engine dispatch (audit remediation Phase 8, iso-behaviour — no test modified; ADR-006) — the scattered `match engine` metadata blocks and `file_type.starts_with("wolf_"|"vx_")` checks are replaced by methods on the `Engine` enum (`db_str`, `data_dir`, `game_title`, with the per-engine title readers moved into `engines/detector.rs`) and a `filter::classify_file_type → FileClass` router used by `export_project`, `collect_wolf_zip_entries` and `debug_inject_file`; `open_project` is decomposed (shared `extract_project` normalizer + `insert_source_file`/`insert_segment` transaction helpers) and now shares its extractor with `debug_dump_segments`, locked by a new characterization test. Adding an engine (Bakin) is now one detect arm plus one arm in each `Engine`/`FileClass` match
- Deduplicate copy-pasted logic across the backend and UI (audit remediation Phase 6, iso-behaviour — no test modified) — new `manifest::refresh_stats` is now the single project-stats recompute point (3 sites), `glossary::relevant_terms` the single glossary-filter helper (2 sites) with a shared `TERM_COLUMNS` SELECT list and an N+1 dedup fetched once into a `HashSet`, `engines::filter::is_map_file` the single `Map*` filename test (4 sites), and `QaError::penalty`/`type_key` replace the score-penalty and report-key match arms; on the front, `openProjectErrorKey`, the `useExportToFile` hook, the `STATUS_SUMMARY` chip convention and the `GlossaryExtractionDonePayload` type replace their duplicated inline copies
- Show per-file progress counters in the FileTree (`✓ translated · ⚠ needs review`) and hide the debug-inject button until every segment is strictly translated — no more inviting the user to inject unreviewed LLM fallbacks
- Render the `reviewed` status (blue) in the project list stats bar so the bar sums to 100%
- Unify the default Ollama model on `gemma4:e4b` — the backend fallback (`q4_K_M`), the settings default (`q8_0`) and the LLM store initial value (`qwen3:4b`) were three different models; single TS source of truth in `src/lib/constants.ts`, mirrored by `DEFAULT_OLLAMA_MODEL` in Rust
- Align CLAUDE.md/CONTEXT.md with reality: the project is mature (no longer "skeleton state") and the error convention is `Result<T, String>` at IPC boundaries + per-module `thiserror` enums (the prescribed global `H2sError` never existed)
- Limit MV/MZ font-size prefix to `Map*.json` files only — `scan_font_status` and `export_project` filter to `file_type = "map"` for MV/MZ engine; actors, items, system, etc. are not prefixed
- Parameterize `fontSizeDialog` i18n strings — `\f[N]` / `\FS[N]` no longer hardcoded; replaced by `{{code}}` interpolation variable; new `hintMvMz` key added (EN + FR)

## [0.4.2] — 2026-06-20

### Fixed
- Fix SIGILL crash on AppImage launch — replace `target-cpu=native` in `.cargo/config.toml` with explicit CPU feature flags (`+aes,+sse2,+sse4.1,+sse4.2,+pclmul`); `native` compiled for the CI runner's exact microarchitecture, producing binaries that crash on any different CPU; the explicit flags satisfy the minimum required by marshal-rs → gxhash (AES-NI + SSE2) while remaining portable across x86-64 v2+ machines

## [0.4.1] — 2026-06-20

### Fixed
- Fix AppImage crash when switching Ollama URL to an HTTPS endpoint (RunPod) — replace `default-tls` (native-tls/OpenSSL) with `rustls-tls` in reqwest; OpenSSL's C-level `abort()` on certificate or TLS init failure bypassed Rust error handling entirely; `rustls` is a pure-Rust implementation with no system library dependency

## [0.4.0] — 2026-06-20

### Added
- Add `engines/filter.rs` — shared filter module for all engine extractors: `is_pure_number()`, `is_pure_symbol()`, `needs_translation(text, engine)`. Single gate replacing per-extractor ad-hoc checks; engine-specific placeholder tokenization via `TokEngine` parameter
- Add `needs_translation` filters in MV/MZ extractor: pure digits (`5`, `100`), pure punctuation/symbols (`…`, `・・・`, `-`, `？？？`, `！！！！`), disabled RPG Maker choices (`-`) are now skipped at extraction (~102 noise segments eliminated on reference MV game)
- Add Ollama health check (`GET /api/tags`) before glossary extraction and before translation (`translate_segments` + `translate_all_segments`) — emits a clear "Ollama inaccessible — vérifiez qu'il est démarré" error instead of a raw reqwest TCP error
- Add 1-retry with 2 s delay to `OllamaProvider::chat()` (used by glossary extraction) — previously a single failed request aborted with no retry
- Add `debug_dump_segments` universal debug extraction command (replaces `debug_dump_wolf_segments`) — dispatches by engine (MV/MZ, VX Ace, Wolf) and produces a unified JSON with `engine`, `total_files`, `total_segments`, `by_kind` histogram, and per-file segment list
- Add Groupe F placeholder codes (`\FF[a_0_001]`, `\AA[FF]`, `\F[code]`) to `RE_MVMZ` and `RE_MZONLY` tokenizer — pure plugin-code segments are auto-skipped via `needs_translation`
- Add `" by Hoshi2Star"` branding suffix to `GameTitle` at MV/MZ extraction
- Add project deletion confirmation dialog (`AlertDialog`) in `ProjectList` with i18n keys `confirmDeleteTitle/Desc/Cancel/Confirm` (EN + FR)
- Add exact-one-translation constraint to glossary extraction LLM prompt — eliminates slash-separated double translations (`エール→Aura/Elixir`) from small models

### Changed
- Refactor all three engine extractors (MV/MZ, VX Ace, Wolf) to delegate all filter logic to `engines::filter::needs_translation` — removed duplicate `is_placeholder_only` / `is_wolf_placeholder_only` local functions; VX Ace gains symbol/number filtering it previously lacked
- Remove `CommonEventName` from MV/MZ extraction — developer labels (`HUDピクチャ消去`) were extracted as translatable text; now skipped with a comment

### Added
- Add `debug_inject_file(sourceFileId, fontSize?, replaceExisting)` Tauri command — injects a single fully-translated file back into the game (Wolf or future engines); enforces completeness (all segments must be translated before proceeding)
- Add per-file "Debug Inject" button (`FlaskConical` icon, hover-only) in `FileTree.tsx` — visible only when `totalCount > 0 && translatedCount === totalCount`; rows restructured from `<button>` to `<div role="button">` to fix invalid nested-button DOM
- Add `scan_font_status(sourceFileId?, projectId?)` Tauri command — counts segments already prefixed with `\f[N]` vs total translated; drives `FontSizeDialog` before any injection or export
- Add `FontSizeDialog` component (`src/components/FontSizeDialog.tsx`) — `AlertDialog`-based; always shown before inject/export; numeric input for font size (default 18, range 8–64); optional "replace existing" checkbox when `existingFontCount > 0`; locale keys `fontSizeDialog.*` (EN + FR)
- Add `\f[N]` font size prefix management in Wolf RPG export flow — `apply_font_prefix()` applies/replaces `\f[N]` at start of each translated segment; `persist_font_size()` writes the prefix to DB before injection so re-injection picks it up automatically; wired into both `debug_inject_file` and `export_project`
- Add `FontScanResult { existingFontCount, totalTranslated }` IPC type (Rust + TypeScript)
- Add `translated_count` and `total_count` to `SourceFile` (Rust + TypeScript) — computed by `get_source_files` via `LEFT JOIN segments` + `SUM(CASE …)` / `COUNT`; annotated `#[sqlx(default)]` so existing queries (`export_project`) remain unaffected
- Add `extract_wolf_speakers(projectId)` Tauri command — scans Wolf `BasicData/*.dat` database segments for speaker names (field `name` on `character` / `actor` / `人物` type names), deduplicates, and inserts them as project-local glossary entries; wired to a new "Speakers" button in `GlossaryPanel`; locale keys `glossaryPanel.extractSpeakers*` (EN + FR)
- Add Wolf RPG extractor skip filters for Inko v2.0 — `extract_common_events` (v2 + v3) skips events whose command string starts with `X[` or `zz`; `extract_database_segments` skips entries whose value contains `自動ｼｽﾃﾑ初期化`; tests: `test_extract_database_skips_auto_init`, updated `test_real_inko_common_events_v3`
- Add `^@\d+\n` pattern to `RE_WOLF` tokenizer (anchored at start of string) — tokenizes Wolf speaker-tag lines like `@0\n`, `@12\n`; three unit tests: round-trip, two-digit, mid-text unchanged
- Add a "Batch size (segments per call)" setting (1–100, default 20) in Settings → LLM — `ProviderConfig.batch_size` flows through `TranslationContext.batch_size` to `batch::group_segments` in `pipeline::run_inner`, replacing the hardcoded `DEFAULT_BATCH_SIZE`. Clamped to `[1, 100]` before grouping; persisted in `settings.json` as `batch_size` (`#[serde(default)]` for backward compatibility with older config files)

### Fixed
- Fix `.wolf` v8 archives bundling a duplicate `BasicData/` (e.g. a bonus "complete initial state" sample folder under `データ集/`) shadowing the real database files — `legacy_xor::WolfFile` now carries the full reconstructed archive path (`path` field, via the existing `DARC_DIRECTORY` parent-chain walk), and `extract_dat_pairs_from_archives` only accepts files directly under a top-level `BasicData/`. Previously the duplicate `DataBase.dat` (257 bytes, non-standard header) shadowed the real one (85738 bytes) and surfaced as a spurious "encrypted database" error, while the duplicate `CDataBase.dat` (also 257 bytes, valid header) silently shadowed the real 90631-byte one. Also fixes `SysDataBaseBasic` never being skipped on the archive path (case-sensitive comparison against an already-lowercased stem)
- Replace the misleading "encrypted database not supported in F4-03 (deferred to F4-05)" error (F4-05 is complete and never covered this) with a neutral message reporting the unexpected indicator byte

### Added
- Add Tenmon 天文 design theme (phase 1) — night-indigo palette with violet primary and gold `--star` token (`text-star`/`bg-star` utilities), subtle CSS starfield on dark mode, matching day-sky light variant
- Add `@fontsource-variable/noto-sans-jp` to the sans font chain for proper CJK rendering
- Add three design-direction HTML demos in `docs/design/` (Tenmon, Washi, Yoru) with PNG previews in `docs/screenshots/`
- Add Tenmon phase 2 visuals: gold QA score ring (`QAScoreRing` in `QAPanel`), segment-status recap in the `SegmentGrid` footer (per-status counts with dot styling), and a "constellation" translation progress bar in `AppToolbar` (gradient track, milestone nodes, pulsing comet)

### Changed
- Restyle `AppToolbar` — gold ★ logo with glow, "Translate" promoted to primary button (action hierarchy), project name + engine as pill chip, progress bar with violet→gold gradient
- Change segment status badges from plain colored text to dot + label (cyan translated, gold diamond reviewed, amber needs-review, muted untranslated)
- Restyle placeholder highlights as bordered cyan mono chips and glossary highlights as gold dashed underline (replacing green background)
- Unify all panel headers (FileTree, TM, QA, Glossary, SegmentGrid) with uppercase letter-spaced style

### Fixed
- Fix placeholder highlight chips never appearing on Wolf RPG projects — `SourceCell` now picks an engine-specific regex (`PH_RE_WOLF` mirroring the Rust tokenizer's `RE_WOLF`, vs. `PH_RE_SOURCE` for MV/MZ) via the new `getPlaceholderRegex(engine)` in `lib/constants.ts`

### Added
- Add `h2s://llm/segments-updated` event emitted after each persisted batch — `SegmentGrid` merges `targetText`/`status` in place, so rows turn "Translated" batch by batch during long runs (no DB refetch, keeps sort/selection/scroll)
- Add `engines/wolf/decrypt/wolfx.rs` seam for WolfX archives (Wolf v3.5+ Pro, ChaCha20) — returns a guidance error by design: decrypt with UberWolf first, then open the plain `Data/` directory (no native ChaCha20, no bundled sidecar: unconfirmed UberWolf license, Windows-only binary)

### Changed
- Move `engines/wolf/decryptor.rs` to `engines/wolf/decrypt/legacy_xor.rs` (XOR DXA v5/v6/v8 logic unchanged) — decryption variants now live under `wolf/decrypt/`, one submodule per encryption scheme

### Fixed
- Fix "Translate All" progress bar hitting 100% after each file — `pipeline::run` now threads a global `(done_offset, global_total)` so `h2s://llm/progress` reports one continuous percentage across all files

### Changed
- `llm::pipeline::run_inner`/`run` now persist each batch's `target_text`/`status` to the DB immediately (`persist_batch_results`) instead of after the whole pipeline finishes — a crash mid-translation only loses the in-flight batch
- Automatic cooldown ("Translate All") moved from a per-file check (`last_cooldown_at` in `translate_all_segments`) to a per-batch check inside `pipeline::run_inner` via the new `CooldownState`/`maybe_rest` — large files (e.g. Wolf `CommonEvent.dat`, ~80 batches) now actually pause mid-file once the threshold elapses; added `CoolingPayload` (`remainingSecs`) to `progress.rs`. No frontend changes (`h2s://llm/*` event shapes unchanged)

### Added
- Add Wolf RPG v3.x `CommonEvent.dat` extraction (Inko's header/UTF-8 layer): fork of `wolfrpg-map-parser` (`KATBlackCoder/wolfrpg-map-parser`, branch `fix/wolf-v3-format`) adds `check_common_events_magic()` (validates v2.x `0x00/0x8F` and v3.x `0x55/0x93` headers), a thread-local `UTF8_MODE` for UTF-8-vs-Shift-JIS string decoding, and LZ4 decompression of the v3.x event table (bytes 11..15 = decompressed size, bytes 19..EOF = LZ4 block); `[patch.crates-io]` re-introduced to point at this fork
- Same fork fixes Honoka's (v2.x) `CommonEvent.dat`, which previously panicked on missing `0x04D20000`/`0x09D20000` CallCommonEvent signatures — now extracts 2195 segments (`test_real_honoka_common_events`)

### Fixed
- Fix Wolf RPG v3.x (LZ4-compressed `.dat` databases): `dat_parser::parse_database` now decompresses the LZ4 block (version byte `0xC4`) before parsing — `DataBase.dat`/`CDataBase.dat`/`SysDatabase.dat` extract correctly for Inko (438 segments); `.mps` maps and `CommonEvent.dat` v3.x format incompatibilities remain open (see `docs/references/wolfrpg-format-compatibility.md`)

### Added
- Add `extract_common_events()` — parses `CommonEvent.dat` via `wolfrpg_map_parser::common_events_parser::parse_bytes` (SJIS-decoded internally); wrapped in `catch_unwind` for panic safety; adds `CommonEventMessage` variant to `WolfSegmentKind`; wired into `extract_all_wolf` and `extract_wolf_project` with unencrypted + `.wolf` archive fallback via `load_common_event_bytes()`

### Changed
- Remove `[patch.crates-io]` fork of `wolfrpg-map-parser` and `src-tauri/vendor/` (−219 files) — revert to 0.6.0 vanilla; D2/D3 commands (CallCommonEvent/ReserveCommonEvent) are pure control flow with no text content; `catch_unwind` already handles any remaining unknown-command panics gracefully at the map level

### Fixed
- Fix Wolf RPG map extraction panic on unknown command `0x09D20000` (CallCommonEvent with 8 integer arguments) — `catch_unwind` absorbs the panic and skips the affected map file; no text loss since D2/D3 commands carry no translatable content

### Added
- Add Wolf RPG full integration (F4-05): `open_project` and `export_project` commands now support Wolf RPG — engine detection, segment extraction from `.dat`/`.mps` files, and export back to `Data/MapData/` + `Data/BasicData/`
- Add transparent DXA archive fallback in Wolf extractor/injector — tries unencrypted `Data/MapData/` and `Data/BasicData/` first, then decrypts `.wolf` archives; `load_mps_for_stem` and `load_dat_for_stem` expose this for the injector
- Add `extract_all_wolf()` — groups extracted segments by source file, returning `Vec<(file_name, file_type, Vec<WolfSegment>)>`
- Add Wolf RPG game title detection from `Game.ini` (`GameTitle=` line, Shift-JIS/UTF-8 decode)
- Add `PossibleWolfX` error variant in Wolf decryptor — emitted when all XOR keys fail; instructs users to pre-decrypt with UberWolf before opening
- Add engine parameter to `qa::check()` — Wolf uses a 520 px text box vs 720 px for MV/MZ; `LineWidthConfig::wolf_default()` added
- Add `wolf_map` and `wolf_database` icons in `FileTree` — violet `Map` icon for `.mps` files, violet `Database` icon for `.dat` files
- Add Wolf RPG engine layer documentation in `docs/architecture.md` — extractor, injector, decryptor, encoding, dat_parser modules; WolfX limitation noted

### Added
- Add Wolf RPG binary injector `inject_map()` — sequential scan+splice (Approach B); wolfrpg-map-parser exposes no byte offsets in public structs; replaces ShowMessage/ShowChoice ReadString payloads in order
- Add Wolf RPG Database injector `inject_dat()` — two-file format (.project schema read-only, .dat values rewritten); full re-serialization via `serialize_dat()`; returns only new .dat bytes, never modifies .project
- Add `encode_for_wolf()` — Shift-JIS guard for Wolf v2 (rejects accented/emoji chars with `InjectorError::Encoding`); UTF-8 pass-through for v3+
- Add `inject_all()` — Option A export strategy: writes decrypted .mps/.dat directly to `Data/MapData/` and `Data/BasicData/`; Wolf RPG reads `Data/` with priority over .wolf archives; Option B (DXA re-pack) deferred to F5
- Add round-trip tests: extract→inject identity (.mps + .dat), extract→translate→inject→re-parse (.mps + .dat), inject_all creates files, inject_all does not overwrite .wolf archives (15 new tests, 247 total)
- Add Wolf RPG Database parser `parse_database()` — reads `.project` (schema: type names, field names, `indexInfo`) + `.dat` (int/string values) binary pairs; reverse-engineered from WolfTL (Sinflower, MIT); supports Shift-JIS and UTF-8 magic, rejects LZ4 compression (`0xC4`) with `Unsupported` error
- Add `extract_database_segments()` — converts parsed `DatFile` to `WolfSegment` list; filters by known translatable field names (`name`, `description`, `note`, `message`, `text`) or Japanese character presence (hiragana/katakana/CJK); key format `Database/{db_name}/{type_idx}/{entry_idx}/{field_name}`
- Add `extract_common_events()` — stub returning `Ok(vec![])` pending F4-05; CommonEvents format (0x8E/0x8F/0x91/0x92 sections + 100 fixed strings per event) is too complex to implement safely without a real test fixture
- Add `extract_wolf_project()` — orchestrator combining `.mps` map extraction, `.project`+`.dat` database pairs, and CommonEvents stub; per-file errors are logged and skipped without aborting
- Add `load_dat_files()` — walks `BasicData/` directory for `.project`/`.dat` file pairs by stem matching
- Add Wolf RPG DXA decryptor `extract_all()` — complete archive extraction pipeline (v5/v6/v8): key discovery via WOLF_KEYS table or GuessKeyV6, header decrypt, TOC parse, per-file XOR decrypt; returns `WolfArchive` with decoded filenames
- Add `guess_key_v6()` — automatic XOR key recovery from null high bytes of 64-bit header fields (known-plaintext attack); validates via dual cross-check + index_size plausibility
- Add `parse_index()` — DXA TOC parser for v5 (32-bit, 0x2C entries) and v6/v8 (64-bit, 0x40 entries); decrypts in place, filters directory entries
- Add Wolf RPG XOR-12 key table — hardcoded keys for v2.01, v2.10, v2.20, v2.255 (Honoka), and `no_key` constant
- Add `key_conv()` — symmetric XOR-12 decryption/encryption; handles Wolf RPG offset bug (file data uses `unpacked_size` as key offset)
- Add Wolf RPG DXA v5/v6/v8 header parsing — `read_header()` unified reader; CodePage detection maps 932→Shift-JIS (v2), 65001→UTF-8 (v3+)
- Add Wolf RPG engine detection (`Engine::Wolf`) — detects `Game.exe`/`Game.ini` + `BasicData/` or `Data/*.wolf` or `Data/MapData/*.mps`
- Add `WolfVersion` struct with `is_utf8()` — v2=Shift-JIS, v3+=UTF-8; `guess_wolf_version_from_structure()` defaults to v2.0 (TODO F4-02: read DXA CodePage)
- Add `find_wolf_data_dir()` — tries `Data/` (Windows) then `data/` (Linux fallback)
- Add `Engine::Wolf` tokenizer mode with Wolf RPG placeholder patterns: `\r[Base,Ruby]` ruby, DB refs `\udb/\cdb/\sdb`, `\sysS/\sys/\self/\cself`, `\space/\v?[n]`, multi-char codes, standard `\v/c/s/f/i`, `\m[n]`, alignment `<L>/<C>/<R>`, `\A+/\A-`, no-arg display control codes — 11 unit tests
- Add `engines/wolf/` module scaffold with empty stubs for `decryptor.rs`, `extractor.rs`, `injector.rs`
- Add `encoding_rs = "0.8"` and `wolfrpg-map-parser = "0.6"` to Cargo dependencies

## [0.3.2] - 2026-06-05

### Added
- Add `docs/architecture.md` — full architecture documentation: 5-layer ASCII diagram, module descriptions for all Rust and TypeScript modules, data-flow sequences for open/translate/restore, ADR summary table

### Changed
- Extract `AppToolbar` component from `App.tsx` — toolbar buttons, `CooldownBadge`, `TranslationTimer`, progress bar; reads store state directly
- Extract `AppDialogs` component from `App.tsx` — all conditional modals (`SettingsModal`, `AboutModal`, `TranslateAllDialog`, export `AlertDialog` x2, glossary `AlertDialog`)
- Extract `useAppHandlers` hook from `App.tsx` — all async handlers (`handleTranslate`, `handleTranslateAll`, `handleExportAll`, `handleExportConfirm`, `handleGlossaryConfirm`, `handleGlossaryDecline`) + local dialog state; `App.tsx` reduced from 632 to 141 lines
- Extract `buildHighlightedNodes` to `src/lib/highlight-utils.tsx` — new signature `(text, glossaryTerms: string[], phRe: RegExp)` makes the function independently testable; `columns.tsx` reduced from 328 to 257 lines
- Split `llm/pipeline.rs` (718 lines) into `llm/pipeline.rs` (orchestration: `run` / `run_inner` / `translate_batch`), `llm/split.rs` (`llm_translate_with_split` + recursive split logic), `llm/progress.rs` (`ProgressPayload`, `PlaceholderWarningPayload`)
- Move QA error label functions from `core/report.rs` into `impl QaError { pub fn label(&self, lang: &str) -> String }` in `core/qa.rs`; `report.rs` calls `escape_xml(&err.label(lang))` at the HTML render site

- Split `commands/project.rs` (1 539 lines) into `commands/project.rs` (~727 lines, CRUD project/files/segments), `commands/translate.rs` (translate_segments, translate_all_segments, get_ollama_models), `commands/export.rs` (export_project, export_qa_report, export_tm, export_debug_json), `commands/qa.rs` (qa_check_segment, get_qa_report, get_tm_suggestions)
- Extract domain types to `src-tauri/src/domain/types.rs` — Project, SourceFile, Segment, ProviderConfig, QaReport, ProjectStats, OpenProjectResult, PaginatedSegments; `commands/glossary.rs` updated to import from `domain::types` instead of `commands::project`
- Extract `PH_RE` placeholder regex to `src/lib/constants.ts` — single source of truth shared by `App.tsx` and `columns.tsx`; each call site uses `clonePH_RE()` to get a fresh `RegExp` with reset `lastIndex`
- Extract format helpers `formatDuration`, `engineLabel`, `relativeDate` to `src/lib/format.ts` — removed duplicate local definitions from `FileTree.tsx` and `ProjectList.tsx`
- Create `src-tauri/src/utils/` module with `text::escape_xml` and `time::now_iso8601` — merged duplicate `xml_escape`/`html_escape` private fns from `core/tm.rs` and `core/report.rs` into a single public utility; extracted `now_iso8601` from `core/manifest.rs`
- Refactor `stores/llm.ts` — replace 5 module-level `UnlistenFn` variables and 7 identical teardown blocks with `setupTranslationListeners()` helper and a single `activeTeardown` ref; `startTranslation` and `startTranslateAll` now share all event-handling logic via callbacks

### Added
- Add "Export All" button in toolbar (`Download` icon) — checks project completeness before exporting; if untranslated segments remain, shows a blocking `AlertDialog` with the untranslated count (Close only); if all translated, shows a confirmation dialog (file + segment count) before exporting
- Add `get_project_stats` Tauri command — returns `{ fileCount, totalSegments, untranslatedCount }` for a project via a single SQLite query using `?1` positional binding
- Add `toolbar.exportAll*` i18n keys (EN + FR)
- Add "Translate All" button in toolbar (`Languages` icon) — on click, fetches project stats, opens `TranslateAllDialog` with untranslated count + file count and two adjustable cooldown inputs (work duration default 20 min, rest duration default 3 min), then launches whole-project translation
- Add `translate_all_segments` Tauri command — translates all untranslated segments across all project files sequentially in a background `tokio::spawn` task; after each file, checks elapsed time against threshold and if exceeded, emits `h2s://llm/cooling { remainingSecs }` once per second during the rest phase; updates manifest stats and emits `h2s://llm/completed` at the end
- Add `isCooling: boolean` and `cooldownRemaining: number` state + `startTranslateAll` action + `coolingUnlisten` listener to `useLlmStore` — listens to `h2s://llm/cooling` events and updates cooling state
- Add `useIsCooling` and `useCooldownRemaining` selectors to `llm.ts`
- Add `CooldownBadge` component inline in `Toolbar` — displays `Snowflake` icon + `MM:SS` countdown in blue during cooldown phase
- Add `TranslateAllDialog` component (`src/components/TranslateAllDialog.tsx`) — stats preview + two numeric inputs for threshold and cooldown duration
- Add `toolbar.translateAll*` i18n keys (EN + FR)

## [0.3.1] - 2026-06-04
### Added
- Add About modal (ⓘ button in toolbar) — tagline, author, MIT license, Bitcoin + Ethereum donation addresses with copy buttons, GitHub link
- Add `about.*` i18n keys (EN + FR)
- Add glossary extraction prompt on new project open — `AlertDialog` appears when `wasRestored: false`, fires `extract_glossary_terms` on confirm, shows a slim non-blocking banner between toolbar and editor while extraction runs, disables Translate button (with explanatory label) until `h2s://glossary/extraction-done` event received
- Add `pendingGlossaryExtract` and `isExtractingGlossary` flags to `useProjectStore` with `usePendingGlossaryExtract` / `useIsExtractingGlossary` selectors
- Add `glossaryPrompt.*` i18n keys (EN + FR) — title, description, yes/no, extracting banner, blocked button label, extractDone (with count), extractDone_zero, extractError
- Add Settings modal (⚙ button in toolbar) — LLM config (Ollama URL + model), theme toggle (light/dark), language toggle (EN/FR), persisted via tauri-plugin-store to settings.json in app data dir
- Add settings loaded on app startup from settings.json (merge with defaults for first launch)
- Add Translate button auto-opens Settings if no model is configured (toast + auto-open)
- Add "Retry N failed" yellow button in SegmentGrid toolbar — retranslates all `needs_review` segments in one click (count from full segment list, not filtered view)
- Add `retranslateNeedsReview` i18n key (EN + FR)

### Changed
- Move LLM configuration from modal on Translate button to persistent Settings modal (⚙)
- Move language toggle from toolbar to Settings modal
- Move theme toggle to Settings modal
- Translate button now starts translation directly (no intermediate modal) when model is configured

### Removed
- Remove LlmConfigModal component — replaced by SettingsModal

### Fixed
- Fix LLM batch translation permanently failing when `ResponseFormat` exhausts MAX_RETRIES — replaced flat retry loop with recursive `llm_translate_with_split` (Box::pin): on exhausted retries, batch splits in half and each half is retried independently; single-segment terminal failures fall back to `needs_review` instead of blocking the whole batch
- Fix `eprintln!` in pipeline replaced with `log::warn!` for consistent structured logging

## [0.3.0] - 2026-05-29
### Added
- Add per-row Translate button in SegmentGrid — retranslates a single segment without opening the LLM config modal
- Add checkbox selection column in SegmentGrid — select 2+ segments to show a batch "Translate N lines" button next to the filter dropdown
- Add `ProjectList` panel — displayed when no project is open, lists all known projects from DB with Continue and Delete actions
- Add `list_projects` Tauri command — returns all projects sorted by most recently updated
- Add `delete_project` Tauri command — removes project row (cascade deletes files + segments) and deletes `.hoshi2star.json` manifest file
- Add `translation_secs` column to `source_files` table (migration `0004_source_files_translation_secs.sql`) — persists per-file translation duration across sessions
- Add Groupe E plugin placeholder pattern `\+word[n]` / `\-word[n]` to tokenizer `RE_MVMZ` — covers common community plugin codes such as `\+switch[269]`
- Add test `test_plugin_codes_tokenized` for Groupe E patterns in `tokenizer.rs`
- Add `project.translationSecs` field to `SourceFile` TypeScript type
- Add project management i18n keys `projectList.*` (EN + FR)
- Add `segmentGrid.translateRow` / `translateSelected` / `noModelConfigured` i18n keys (EN + FR)
- Add project manifest `.hoshi2star.json` written at game folder root on `open_project` success (stores project ID, title, engine, file count, segment count)
- Add smart restore: if manifest + DB entry match on re-open, project returned immediately without re-extracting (`wasRestored: true`)
- Add toast "Project restored — continuing where you left off" on smart restore (i18n EN/FR)
- Add manifest stats auto-update after each `update_segment` (manual segment save)
- Add manifest stats update once at end of `translate_segments` batch (before `h2s://llm/completed` event)
- Add `log` crate to Rust dependencies for manifest warning messages
- Add QA HTML report export — standalone self-contained file with inline CSS/JS, no external dependencies
- Add `collect_qa_details()` in new `core/report.rs` — recalculates `qa::check()` at export time, returns only segments with `score < 100`
- Add `generate_qa_html()` — dark-themed HTML with error stats, file/score/type filters (JS inline), bilingual (EN/FR)
- Add `export_qa_report` Tauri command — fetches project title, collects QA details, writes HTML via `tokio::fs::write`
- Add Export QA Report button (FileDown icon) in QAPanel header with `tauri-plugin-dialog` save dialog and sonner toast
- Add QA filter toolbar in SegmentGrid — Select with All / QA Errors / Critical (< 70) / Untranslated / Needs Review
- Add `filteredSegments` useMemo in SegmentGrid — client-side filtering on in-memory segments, resets on file change
- Fix virtualizer `count: rows.length` bug (was `segments.length` — mismatch when filter active)
- Add footer "X / Y segments" display in SegmentGrid when filter is active
- Add TM fuzzy matching with Levenshtein distance (normalised score, threshold 80 %, limit 5 suggestions)
- Add `TmSuggestion` type with `score: f32` and `match_type: "exact" | "fuzzy"` (Rust + TS)
- Add `lookup_fuzzy()` in `tm.rs` — in-memory scan, sorted by score descending (acceptable up to ~5k entries)
- Add `generate_tmx()` — produces TMX 1.4 XML compatible with OmegaT, Trados, memoQ (no XML crate)
- Add `export_tm` Tauri command — writes global TM to a `.tmx` file at a user-chosen path
- Add Exact/Fuzzy badge in TMPanel — green "Exact" for score 1.0, yellow "~XX%" for fuzzy matches
- Add Export TM button (Download icon) in TMPanel header with `tauri-plugin-dialog` save dialog and sonner toast
- Add glossary system: two-level CRUD (global + project-local) backed by SQLite `0003_glossary.sql`
- Add LLM auto-extraction of glossary terms from Actors/Skills/Items/States name fields (`extract_terms_from_project`)
- Add glossary injection into LLM translation prompt (up to 30 terms, `TranslationContext.glossary_terms`)
- Add `QaError::GlossaryMismatch` check (−15 pts) when a known source term is not translated using its glossary target
- Add `GlossaryPanel` with inline CRUD (add/edit/delete), auto-extract button, i18n (EN/FR)
- Add glossary term highlight (green) in SegmentGrid source column
- Add `GlossaryPanel` as third resizable panel in SidePanel (TM=40 / QA=30 / Glossary=30)
- Add 5 Tauri IPC commands: `get_glossary`, `add_glossary_term`, `update_glossary_term`, `delete_glossary_term`, `extract_glossary_terms`
- Add `chat()` method to `LlmProvider` trait for single-turn raw completion
- Add shadcn `AlertDialog` and `Input` components
- Add RPG Maker VX Ace engine support (marshal-rs, non-packaged `.rvdata2` projects)
- Add `vx_ace/extractor.rs` — reads Actors, Armors, Weapons, Skills, Items, Enemies, Classes, CommonEvents, MapInfos, Maps, System from `.rvdata2`
- Add `vx_ace/injector.rs` — re-serialises translated content back to Ruby Marshal binary
- Add VX Ace file type icons in FileTree (amber color scheme, 12 `vx_*` types)
- Add `Engine::VxAce` variant to detector with `Data/` → `data/` fallback for Linux case-sensitivity
- Add Git branch workflow to development conventions in CONTEXT.md

### Changed
- Replace Zustand `fileTranslationTimes` in-memory store with DB-persisted `translation_secs` on `source_files` — translation duration now survives app restarts
- Disable VX Ace engine detection (code preserved in `engines/vx_ace/`, reactivation planned post-Wolf RPG)
- Refocus roadmap: Wolf RPG F4 as absolute priority over VX Ace and other engines
- Rename F3 phase: "Polissage + Glossaire + TM fuzzy + beta privée" (VX Ace removed from scope)
- Rename F4 phase: "Wolf RPG (priorité absolue)" with explicit rationale (~40% of untranslated JP games on DLsite)
- Add engine priority table to ROADMAP.md

### Fixed
- Fix translation duration badge disappearing after reopening a project — duration now read from `source_files.translation_secs` (DB) instead of ephemeral Zustand store
- Fix `translate_segments` partial-move compile error when `file_id` passed via `if let Some(fid)` then reused in async block
- Fix placeholder validation failures now falling back to `needs_review` status instead of blocking the batch — `h2s://llm/placeholder-warning` event emitted per segment, toast shown in UI
- Fix incorrect segment_id reported in placeholder validation errors (was always the first segment of the batch)
- Reduce glossary injection to relevant terms only (filtered by batch content, max 20; fallback: 10 shortest) — improves LLM attention on placeholder preservation
- Strengthen system prompt with explicit CRITICAL RULE block for ⟦ph_N⟧ token preservation
- Fix `clippy::type_complexity` in `collect_rvdata2_files` via `RvData2Entry` type alias

## [0.2.1] - 2026-05-25
### Added
- Dynamic Ollama model selector in LLM config modal
- Translation timer per file in FileTree (session only)
- Export button in toolbar with toast notifications
- Bilingual interface EN/FR with i18n (i18next)
- VRAM-based model recommendations in documentation

### Changed
- QA line length check: fixed 50-char limit replaced by pixel-based system (full-width vs half-width chars)
- Default recommended model: qwen3:4b → qwen3:4b-instruct-2507-q8_0 (instruct variant)
- README.md and README.fr.md with screenshots and bilingual documentation

### Fixed
- LLM error events were silent (no toast shown to user)
- Spinner stuck when all segments already translated (missing h2s://llm/completed event)
- qwen3 thinking mode polluting parser output (/no_think directive added)
- ResponseFormat errors not retried (added to retry loop)
- Segments with only placeholders incorrectly extracted
- Empty and whitespace-only segments extracted
- QA panel not showing error details
- ResizablePanelGroup panels incorrectly sized (react-resizable-panels v4: numbers = px not %)
- FileTree showing icons without filenames (missing serde rename_all camelCase on Rust structs)
- translate_segments silently doing nothing (fileId never passed to backend)
- SegmentGrid not refreshing after LLM translation

## [0.2.0] - 2026-05-24
### Added
- RPG Maker MV/MZ engine support (JSON extraction and injection)
- .rpgmvp/.rpgmvo decryption (XOR + System.json key)
- Engine auto-detection from game folder structure
- SQLite database with projects/source_files/segments
- Placeholder tokenizer (⟦ph_N⟧ format, Rust-side)
- Lowercase escape codes support (\n[n], \c[n])
- LLM pipeline: tokenize → batch → translate → QA → restore (Ollama provider, 3-retry on failure)
- Translation Memory with SHA-256 exact match
- QA engine: placeholder check, line width, UTF-8 BOM
- 3-panel CAT editor: FileTree | SegmentGrid | TM+QA
- TanStack Table v8 with virtual scroll (10k+ rows)
- Zustand stores for editor, project and LLM state
- TanStack Query for async Tauri invoke() calls
- GitHub Actions CI/CD for Linux + Windows builds

[0.4.7]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.4.7
[0.4.6]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.4.6
[0.4.5]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.4.5
[0.4.4]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.4.4
[0.4.3]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.4.3
[0.4.2]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.4.2
[0.4.1]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.4.1
[0.3.2]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.3.2
[0.3.1]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.3.1
[0.3.0]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.3.0
[0.2.1]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.2.1
[0.2.0]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.2.0
