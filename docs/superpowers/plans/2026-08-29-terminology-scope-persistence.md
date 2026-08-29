# Terminology Scope and Persistence Implementation Plan

> **Execution choice:** The user selected inline implementation in the current task.

**Status:** Implemented and verified on 2026-08-29.

**Goal:** Make every existing terminology translation immediately usable and editable, remove review/lock states completely, and use independent project/global scope as the only persistence lifecycle.

**Architecture:** The absence of a translation row means “untranslated”; the presence of a row means “usable”. A project translation belongs to one project, a global translation survives project deletion, and the same term/language may have both rows. Explicit translation may write project, global, or both. Explicit promotion copies a project row globally without deleting it and never overwrites a different global target. Extraction and rescans only maintain source terms and occurrences; they never overwrite targets.

**Tech stack:** Rust, Tauri 2 IPC, SQLite/sqlx migrations, React, TanStack Query/Table, Zustand, shadcn, i18next, Vitest, cargo test, Tauri MCP.

---

## Product invariants

1. There is no `proposed`, `approved`, or `locked` status in schema, Rust, IPC, UI, filters, badges, or statistics.
2. A validated translation row is immediately available to the matching language pair and scope.
3. Every target remains editable. Only an explicit translate/retranslate command may replace a target in its selected scope.
4. Extraction, rescan, cleanup, project changes, and model changes never overwrite an existing target.
5. Project/global are independent rows. “Projet + global” writes both rows; promotion copies project to global and retains project.
6. Promotion conflict policy is deterministic:
   - no global row: copy;
   - same normalized target: no-op and count as already global;
   - different global target: do not overwrite and report a conflict.
7. Promotion exists for one row, selected rows, and every row matching the current server-side filters; the last form never ships 10k–50k IDs through IPC.
8. Deleting a project cascades only its project translations; global copies survive.
9. Legacy `approved` and `locked` rows migrate as ordinary translations. Legacy `proposed` rows are removed and become untranslated because their quality was never trusted. The pre-migration database backup remains the recovery source.
10. `contextual`, `preferred`, and `required` remain optional application-strength metadata and are independent from persistence.

## Task 1 — Safe schema migration

**Files:**
- Create `src-tauri/migrations/0009_terminology_translation_scope.sql`
- Modify `src-tauri/src/db/pool.rs` tests

- [x] Rebuild `terminology_translations` without `review_status`.
- [x] Copy only legacy `approved` and `locked` rows, preserving IDs, scopes, target text, enforcement, provenance, timestamps, and target variants.
- [x] Drop legacy proposals and their variants; recreate project/global uniqueness indexes and foreign keys.
- [x] Prove fresh installs and v8 upgrades reach v9 with `PRAGMA foreign_key_check` clean.

## Task 2 — Remove review state from domain and resolver

**Files:**
- Modify `src-tauri/src/core/terminology/types.rs`
- Modify `src-tauri/src/core/terminology/repository.rs`
- Modify `src-tauri/src/core/terminology/resolver.rs`
- Modify relevant Rust tests

- [x] Delete `ReviewStatus` and every serialized `review_status` field.
- [x] Replace review counts with total, untranslated, project translations, and global translations.
- [x] Resolve all existing rows with project-over-global precedence.
- [x] Keep enforcement as independent metadata.

## Task 3 — Translation writes and non-overwrite guarantees

**Files:**
- Modify `src-tauri/src/core/terminology/translator.rs`
- Modify translation job/command types and tests

- [x] Rename proposal-oriented summaries to translated counts.
- [x] Write validated targets directly; permit explicit retranslation to update only the requested scope.
- [x] Add project/global/both scope. For both, validate the complete model response before writing either row, then write both in one transaction.
- [x] Verify rescans and source cleanup never alter target rows.

## Task 4 — Global promotion API

**Files:**
- Modify `src-tauri/src/core/terminology/repository.rs`
- Modify `src-tauri/src/commands/terminology.rs`
- Modify `src-tauri/src/lib.rs`
- Modify `src-tauri/tests/terminology_commands.rs`

- [x] Add one/selection promotion accepting project translation IDs.
- [x] Add filtered promotion accepting the exact `TerminologyQuery`, resolving candidates inside SQLite/Rust rather than accepting a huge ID array.
- [x] Return `{ copied, alreadyGlobal, conflicts, skipped }` and preserve all conflicting global targets.
- [x] Test project deletion after promotion and EN/FR/project isolation.

## Task 5 — Frontend lifecycle simplification

**Files:**
- Modify `src/features/terminology/api.ts`
- Modify terminology dialogs, table, toolbar, workspace, store, tests, and locales

- [x] Remove review status filters, badges, fields, counters, and wording.
- [x] Show “Sans traduction” only when no effective row exists; show project/global scope explicitly.
- [x] Add translation scope choices: project, global, project + global.
- [x] Add accessible actions to make one row, selected rows, or all filtered project rows global.
- [x] Confirm bulk actions with the current filter summary and display copy/no-op/conflict counts.
- [x] Keep edit, translate/retranslate, archive, permanent delete, select-page, and delete-selection controls keyboard accessible.

## Task 6 — End-to-end verification

- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`
- [x] `cargo test --manifest-path src-tauri/Cargo.toml`
- [x] `pnpm typecheck`
- [x] `pnpm lint` (0 error; 8 existing warnings)
- [x] `pnpm test`
- [x] `pnpm build`
- [x] Tauri MCP at 1450×760 and both themes: no global overflow; the translate dialog exposes three scopes; no review/lock UI remains; actions are keyboard-addressable; long table content scrolls inside its bounded region.

## Done criteria

- Schema and code contain no review-state lifecycle.
- A translated term is usable immediately and editable later.
- Project/global persistence behaves independently and conflicts never overwrite silently.
- Bulk filtered promotion scales without a client-side ID list.
- Old untrusted proposals are not silently activated; trusted legacy targets survive migration.
- All automated tests and real Tauri visual/path checks pass.
