# MV/MZ Pilot A/B Comparison Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Compare the same representative MV/MZ segments translated without and with engine-owned context, report separate provider costs and automatic QA, and present the results in a redesigned non-destructive review workspace.

**Architecture:** A state-free Tauri command creates one disposable SQLite workspace from the selected MV/MZ game, selects one deterministic sample, and runs the shared translation pipeline twice with an explicit typed context policy. The baseline pass disables prompt context while retaining MV/MZ tokenization; the contextual pass uses the existing MV/MZ context adapter. Both passes operate only on the temporary database, collect independent provider metrics, run typed QA plus source-script/context-leak checks, then close and remove the database. A focused Zustand store owns the pilot session and a dedicated React workspace composes the installed shadcn primitives into setup, progress, metrics, filters, and side-by-side review states.

**Tech Stack:** Rust, Tauri v2, SQLx/SQLite, serde, Zustand, React 19, TypeScript, shadcn/radix-nova, Tailwind CSS v4, Vitest.

---

### Task 1: Make prompt context an explicit pipeline policy

**Files:**

- Modify: `src-tauri/src/llm/provider.rs`
- Modify: `src-tauri/src/llm/pipeline.rs`
- Modify: `src-tauri/src/commands/translate.rs`
- Test: `src-tauri/src/llm/pipeline.rs`

- [x] **Step 1: Add a typed context policy**

Define `PromptContextPolicy::{Disabled, EngineOwned}` on `TranslationContext`. Keep normal translation commands on `EngineOwned`; do not encode the policy in engine-name strings.

- [x] **Step 2: Respect the policy before building engine context**

When disabled, provide one `None` context per segment without calling the adapter. When engine-owned, continue to call `context::build_for_segments` with the actual project engine.

- [x] **Step 3: Test both branches**

Assert that the provider receives only `None` contexts for the baseline and receives MV/MZ metadata/neighbours for the contextual run.

### Task 2: Implement the isolated A/B command and automatic QA

**Files:**

- Modify: `src-tauri/src/commands/pilot.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands/pilot.rs`

- [x] **Step 1: Refactor disposable-workspace lifecycle**

Share detection, extraction, sample selection, pool closure, and `TempDir` removal between preparation and translation. Preserve the command boundary without `AppState`.

- [x] **Step 2: Translate the identical sample twice**

Create one provider from the selected OpenAI-compatible preset, check health, run the baseline and contextual variants against the same `(id, source_text)` pairs, and drain metrics after each pass into separate vectors.

- [x] **Step 3: Build typed comparison and summary payloads**

Return source/context metadata, both translations, per-variant QA, changed-output status, category totals, average QA, provider calls, tokens, duration, attempts, and isolation proof.

- [x] **Step 4: Add automatic pilot-only quality checks**

Reuse `core::qa::check`, mark empty translations and remaining source script, detect verbatim neighbour leakage conservatively, and never write pilot decisions or outputs to the personal TM/database.

- [x] **Step 5: Test behavior with a deterministic mock provider**

Assert identical segment ordering, absent/present contexts, separate metrics, QA flags, no TM hits, unchanged game bytes, and removed temporary database.

### Task 3: Build the redesigned comparison workspace

**Files:**

- Create: `src/stores/pilot.ts`
- Create: `src/components/pilot/PilotComparisonWorkspace.tsx`
- Modify: `src/components/AppToolbar.tsx`
- Modify: `src/App.tsx`
- Modify: `src/lib/types.ts`
- Modify: `src/locales/fr.json`
- Modify: `src/locales/en.json`
- Test: `src/stores/pilot.test.ts`
- Test: `src/components/pilot/PilotComparisonWorkspace.test.tsx`

- [x] **Step 1: Mirror the Rust contracts and create a focused store**

Own only open/running/progress/report/error/filter/review-decision state. Register pilot progress listeners before invoking the command and always tear them down.

- [x] **Step 2: Add a guarded toolbar entry**

Show the pilot action only for an active MV/MZ project. Disable it during normal translation or glossary extraction and keep other engines outside this adapter.

- [x] **Step 3: Compose setup and progress states**

Use the installed shadcn `Button`, `Input`, `Select`, `Badge`, and `ScrollArea` primitives. Default to a conservative 12-segment sample, explain the two calls per sample, and show phase-aware progress.

- [x] **Step 4: Compose the A/B review state**

Present summary metrics, category filters, source/context facts, baseline/contextual translations side-by-side, QA indicators, and local human preference buttons. Keep decisions in memory only for this phase.

- [x] **Step 5: Cover store and UI behavior**

Test listener teardown, command arguments, reset/isolation state, MV/MZ guard, progress, category filtering, and side-by-side content.

### Task 4: Verify safety, performance, and regressions

**Files:**

- Modify only if a regression is found: files above

- [x] **Step 1: Run focused checks**

Run Rust pilot/pipeline tests, frontend pilot tests, TypeScript checking, formatting, Clippy, and ESLint.

- [x] **Step 2: Run full suites and production build**

Run all Rust tests, all Vitest tests, and the Vite production build.

- [x] **Step 3: Recheck fixture and database isolation**

Run preparation/A-B logic only against temporary copies of the MV/MZ fixtures. Compare source-tree digests and verify the personal database counts remain zero.

- [x] **Step 4: Inspect the redesigned workspace in the application**

Launch the UI against mocked IPC or a development provider, verify the setup/progress/report states at desktop sizes, and correct overflow, keyboard focus, labels, and reduced-motion behavior.
