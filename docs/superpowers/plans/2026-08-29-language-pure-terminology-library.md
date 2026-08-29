# Language-Pure Terminology Library — Implemented Plan

**Status:** Implemented and verified on 2026-08-29.

**Goal:** Keep terminology as a reusable lexical library: short units that belong to the declared source language, with no decoration, foreign run, identifier prefix, or complete sentence.

## Implemented invariants

- Japanese entries contain Japanese scripts only. Katakana loanwords such as `エネミー` are valid Japanese terms.
- Latin-only noise such as `Bad`, `Clear`, `EXP`, and `GP` is rejected for a Japanese source.
- Mixed input is split by language ownership: `EXP獲得` yields `獲得`; `-----エネミー` yields `エネミー`.
- A structured engine seed is kept only when it resolves to one lexical span and is not a sentence. Multi-unit messages fall back to the morphological analyzer.
- Automatic entries share one canonical default-sense row per source language and normalized text; engine metadata may promote a generic semantic type.
- Manual create/update operations use the same language-purity and lexical validation.
- Completed rescans archive obsolete automatic entries without deleting manual/imported entries or overwriting translations.

## Implemented files

- `src-tauri/src/core/terminology/language.rs`
- `src-tauri/src/core/terminology/analyzer/filters.rs`
- `src-tauri/src/core/terminology/scanner.rs`
- `src-tauri/src/core/terminology/repository.rs`
- `src-tauri/tests/terminology_scan.rs`
- `src-tauri/tests/terminology_commands.rs`

## Verified examples

| Source input | Active Japanese terminology |
| --- | --- |
| `Bad EXP GP` | none |
| `EXP獲得` | `獲得` |
| `HP回復量` | `回復量` |
| `-----エネミー` | `エネミー` |
| `ぐにつけあがってくるんだからね` | rejected as a long fragment |

The extraction filter version is `language-pure-lexical-v5`, forcing existing projects to be rescanned under these rules.
