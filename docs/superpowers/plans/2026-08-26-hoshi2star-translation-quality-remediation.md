# Hoshi2Star Translation Quality Remediation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Empêcher les fuites de contexte et les traductions sémantiquement invalides observées avec Gemma 4, tout en conservant le provider OpenAI-compatible générique.

**Architecture:** Le moteur MV/MZ choisit une politique de prompt selon le type de segment : dialogues avec voisins bornés, choix avec branche seule, noms et termes canoniques sans voisins. Le pipeline regroupe les unités compatibles, réutilise les traductions canoniques déjà validées, applique une QA sémantique avant persistance et stocke immédiatement le score. Les rapports QA recalculent les résultats réels et l’export refuse les erreurs critiques.

**Tech Stack:** Rust, Tauri v2, SQLx/SQLite, reqwest OpenAI-compatible, React/TypeScript, Vitest, Cargo tests, MCP Tauri.

---

### Task 1: Enforce MV/MZ context policy by segment kind

**Files:**
- Modify: `src-tauri/src/llm/context/mod.rs`
- Modify: `src-tauri/src/llm/context/mv_mz.rs`
- Modify: `src-tauri/src/llm/batch.rs`
- Test: `src-tauri/src/llm/context/mv_mz.rs`
- Test: `src-tauri/src/llm/batch.rs`

- [x] **Step 1: Write failing policy tests**

Seed one `dialogue`, one `choice`, one `speaker`, and one `system_term`. Assert dialogue receives two bounded neighbours, choice keeps `branch_path` with empty neighbour vectors, and speaker/system terms have no scene, speaker, branch, or neighbours.

- [x] **Step 2: Run the focused tests and verify failure**

Run: `cargo test llm::context --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because every current MV/MZ row receives the same neighbour query.

- [x] **Step 3: Implement typed prompt policies**

Add `PromptContextClass::{Dialogue, Branch, Canonical, Isolated}` and a stable classifier for MV/MZ kinds. Apply the class while constructing `SegmentPromptContext`; only `Dialogue` may retain neighbours. Use source-only deduplication for `Canonical`, contextual deduplication for `Dialogue`/`Branch`, and kind-only isolation for other database strings.

- [x] **Step 4: Verify focused tests**

Run: `cargo test llm::context llm::batch --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 2: Make provider requests strict and contamination-resistant

**Files:**
- Modify: `src-tauri/prompts/translate/default.toml`
- Modify: `src-tauri/src/llm/provider.rs`
- Modify: `src-tauri/src/llm/pipeline.rs`
- Modify: `src-tauri/src/llm/split.rs`
- Test: `src-tauri/src/llm/provider.rs`
- Test: `src-tauri/src/llm/pipeline.rs`

- [x] **Step 1: Write failing protocol and grouping tests**

Assert duplicate, missing, unknown, and empty response IDs return `ResponseFormat`; assert dialogue requests are not mixed with speakers and contain at most ten units; assert repeated canonical source text produces one provider input and one output reused for every occurrence.

- [x] **Step 2: Run tests and verify failure**

Run: `cargo test llm::provider::tests llm::pipeline::tests --manifest-path src-tauri/Cargo.toml`

Expected: FAIL on permissive fallback parsing and mixed batching.

- [x] **Step 3: Implement strict IDs and prompt grouping**

Require exactly one `[N] translation` for every input ID, reject duplicate/unknown/free-form lines, and remove the line-count fallback. Rewrite the system prompt to state that `text` is the sole output source and all metadata are non-output reference data. Partition provider calls by context class and cap contextual dialogue groups at ten units while retaining the configured size for canonical/isolated strings.

- [x] **Step 4: Expose pipeline retry causes in metrics**

Add pipeline-level counters for response-format retries, placeholder retries, recursive splits, and semantic rejections; emit them with the existing metrics event without coupling the provider to Ollama or Gemma.

- [x] **Step 5: Verify focused tests**

Run: `cargo test llm::provider::tests llm::pipeline::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 3: Validate semantics before persistence and recalculate project QA

**Files:**
- Modify: `src-tauri/src/core/qa.rs`
- Modify: `src-tauri/src/core/report.rs`
- Modify: `src-tauri/src/llm/pipeline.rs`
- Modify: `src-tauri/src/commands/qa.rs`
- Modify: `src-tauri/src/commands/export.rs`
- Modify: `src-tauri/src/domain/types.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/components/editor/QAPanel.tsx`
- Modify: `src/locales/en.json`
- Modify: `src/locales/fr.json`
- Test: adjacent Rust and frontend tests

- [x] **Step 1: Write failing semantic QA tests**

Cover empty output, unchanged source, remaining Japanese for `ja -> fr`, suspicious short-source expansion, overlong speaker output, and copied neighbour source. Assert each critical error sets `needs_review`, persists `qa_score`, and keeps a non-empty safe fallback.

- [x] **Step 2: Implement semantic QA types**

Add serializable errors `EmptyTranslation`, `UnchangedSource`, `SourceScriptRemaining`, `SuspiciousExpansion`, and `ContextLeak`. Add `QaResult::has_critical_errors()`; line width and glossary mismatch remain warnings, while empty/placeholder/script/unchanged/expansion/leak are critical.

- [x] **Step 3: Validate before saving**

Run semantic QA for LLM and TM outputs, set `status = needs_review` for critical failures, persist `qa_score` in the same transaction, and emit the existing segment update with the real status. Reuse only project translations whose canonical segment passed critical QA.

- [x] **Step 4: Recalculate full project QA and gate export**

Make `get_qa_report` recalculate and persist every translated/review row, including empty failed targets, with exact `errors_by_type`. Before game export, run the same audit and return an actionable error when critical rows remain.

- [x] **Step 5: Update QA UI contracts and labels**

Mirror every Rust error variant in TypeScript and display localized labels in the existing shadcn/TanStack redesign without adding a second QA system.

- [x] **Step 6: Verify focused QA tests**

Run: `cargo test core::qa commands::qa core::report --manifest-path src-tauri/Cargo.toml`

Run: `pnpm test src/components/editor/QAPanel.test.tsx`

Expected: PASS.

### Task 4: Verify regressions and repeat the real-game test

**Files:**
- Modify only if a regression is found: files above

- [x] **Step 1: Run static and complete automated checks**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Run: `pnpm typecheck && pnpm test && pnpm build`

Expected: no failures; only intentional ignored Rust diagnostics and the existing Vite chunk warning are acceptable.

- [x] **Step 2: Repeat StandGirl on a fresh temporary copy through MCP**

Open `test/StandGirl-立ちんぼ六花の深い夜-`, translate `ja -> fr` with the same Ollama `gemma4:e4b` configuration, and collect extraction count, calls, tokens, retries/splits, elapsed time, CPU/RAM/GPU/VRAM, QA counts, repeated-source consistency, and archive validity.

- [x] **Step 3: Compare against the baseline**

Baseline: 1191 segments, 160 calls, 303904 tokens, about 25m15s, 8 empty, 52 unchanged, 58 Japanese remaining, 45 inconsistent repeated-source groups, 99 speaker targets for 10 source names, and 0 persisted QA scores. Report every new figure and any remaining defect.

- [x] **Step 4: Verify source safety**

Hash the original fixture before and after, validate every exported JSON, compare event command topology, and boot only a patched temporary copy. The original game and unrelated working-tree changes must remain untouched.
