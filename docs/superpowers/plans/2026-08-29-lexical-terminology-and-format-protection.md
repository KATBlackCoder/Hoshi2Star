# Lexical Terminology and Format Protection — Implemented Plan

**Status:** Implemented and verified on 2026-08-29.

**Goal:** Separate lexical terminology from translation formatting. The glossary stores words; the segment translator preserves structural characters and RPG Maker control codes.

## Implemented behavior

- Repeated ASCII edge decoration is converted to immutable placeholders before model translation and restored byte-for-byte afterward.
- Japanese orthographic marks such as `ー` and `々` remain part of words and are never treated as decoration.
- Ordinary sentence punctuation is not over-protected, so language-specific punctuation can still be translated naturally.
- Automatic terminology rejects decorated tokens, foreign-script runs, all-hiragana fragments longer than ten characters, and general units longer than 24 characters.
- Completed rescans archive automatic entries that no longer have an occurrence or translation.
- Extraction/rescan never overwrites an existing target translation.

## Regression examples

| Pipeline | Input | Stored/sent | Result |
| --- | --- | --- | --- |
| Terminology | `-----エネミー` | `エネミー` | clean lexical entry |
| Translation | `-----エネミー` | placeholder + `エネミー` | `-----Enemy` |
| Terminology | `EXP獲得` | `獲得` | foreign run discarded |
| Terminology | long dialogue fragment | rejected | no glossary pollution |

## Implemented files

- `src-tauri/src/llm/tokenizer.rs`
- `src-tauri/src/llm/pipeline.rs`
- `src-tauri/src/core/terminology/language.rs`
- `src-tauri/src/core/terminology/analyzer/filters.rs`
- `src-tauri/src/core/terminology/scanner.rs`
- `src-tauri/tests/terminology_scan.rs`
