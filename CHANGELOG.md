# Changelog

All notable changes to Hoshi2Star will be documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com) — [Semantic Versioning](https://semver.org).

## [Unreleased]
### Added
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
- Disable VX Ace engine detection (code preserved in `engines/vx_ace/`, reactivation planned post-Wolf RPG)
- Refocus roadmap: Wolf RPG F4 as absolute priority over VX Ace and other engines
- Rename F3 phase: "Polissage + Glossaire + TM fuzzy + beta privée" (VX Ace removed from scope)
- Rename F4 phase: "Wolf RPG (priorité absolue)" with explicit rationale (~40% of untranslated JP games on DLsite)
- Add engine priority table to ROADMAP.md

### Fixed
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

[0.2.1]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.2.1
[0.2.0]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.2.0
