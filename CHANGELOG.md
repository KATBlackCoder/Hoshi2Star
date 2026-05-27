# Changelog

All notable changes to Hoshi2Star will be documented in this file.
Format: [Keep a Changelog](https://keepachangelog.com) — [Semantic Versioning](https://semver.org).

## [Unreleased]
### Added
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
