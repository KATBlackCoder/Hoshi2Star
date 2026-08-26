# MV/MZ Pilot Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Prepare a non-destructive MV/MZ translation pilot with real provider metrics, a deterministic representative sample, and a disposable SQLite database that cannot alter the personal application database.

**Architecture:** `OpenAiCompatibleProvider` records typed metrics for translation and glossary calls and exposes them through a default-compatible `LlmProvider::drain_metrics` hook. A new `commands::pilot` module reuses the shared MV/MZ extractor, seeds a `tempfile::TempDir` database without receiving `AppState`, selects a deterministic stratified sample, enriches it with the existing MV/MZ context builder, then closes and deletes the database before returning. Tauri IPC and TypeScript types expose preparation results now and leave the A/B translation UI for the next increment.

**Tech Stack:** Rust, Tauri v2, SQLx/SQLite, serde, tempfile, reqwest, Zustand, Vitest, httpmock.

---

### Task 1: Record provider request metrics

**Files:**
- Modify: `src-tauri/src/llm/provider.rs`
- Modify: `src-tauri/src/llm/pipeline.rs`
- Modify: `src-tauri/src/llm/progress.rs`
- Test: `src-tauri/src/llm/provider.rs`
- Test: `src-tauri/src/llm/pipeline.rs`

- [x] **Step 1: Write failing provider metrics tests**

Add an HTTP-mock response containing OpenAI `usage` fields and assert that `drain_metrics()` returns one successful `translate` metric with model, input-unit count, prompt/completion/total tokens, duration, and attempt count. Add a failing HTTP response test and assert that it records `success = false` without inventing token counts.

- [x] **Step 2: Run the focused tests and verify failure**

Run: `cargo test llm::provider::tests --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because `ProviderCallMetrics`, response usage parsing, and `drain_metrics` do not exist.

- [x] **Step 3: Implement typed collection without changing translation results**

Define `ProviderTask::{Translate, Chat}` and:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCallMetrics {
    pub task: ProviderTask,
    pub model: String,
    pub input_units: usize,
    pub prompt_chars: usize,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub duration_ms: u64,
    pub attempts: u32,
    pub success: bool,
}
```

Store metrics behind `std::sync::Mutex<Vec<ProviderCallMetrics>>`. Add a default empty `LlmProvider::drain_metrics` implementation so existing mocks remain source-compatible. Parse optional OpenAI usage fields and record both successful and failed logical calls.

- [x] **Step 4: Emit metrics after each pipeline batch**

After `translate_batch`, drain the provider only when a Tauri handle exists and emit non-empty vectors as `h2s://llm/metrics`. A pilot or test without an app handle retains metrics for explicit collection.

- [x] **Step 5: Run focused provider and pipeline tests**

Run: `cargo test llm::provider::tests --manifest-path src-tauri/Cargo.toml`

Run: `cargo test llm::pipeline::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 2: Build deterministic MV/MZ pilot samples

**Files:**
- Create: `src-tauri/src/commands/pilot.rs`
- Modify: `src-tauri/src/commands/project.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/Cargo.toml`
- Test: `src-tauri/src/commands/pilot.rs`

- [x] **Step 1: Write failing pure sampling tests**

Create candidates for all six typed categories and assert that a 50-item sample requests quotas `20/8/8/6/6/2`, remains deterministic, never duplicates a stable `(file_name, json_key)` key, and fills shortages from the remaining categories.

- [x] **Step 2: Implement typed categories and stratified selection**

Define `PilotSampleCategory::{Dialogue, ChoiceBranch, Database, NamesUi, PlaceholderMultiline, Uncertain}`. Classify placeholder/multiline content first using `Tokenizer::tokenize`, then choices/branches, dialogue, database descriptions/messages, names/UI, and plugin/unknown content. Rank candidates by SHA-256 of `file_name + NUL + json_key`, allocate weighted quotas, and fill any deficit from unused ranked candidates.

- [x] **Step 3: Expose only the shared extraction internals needed by the pilot**

Change `ExtractedFile`, `ExtractedFileSeg`, `visit_extracted_files`, `insert_source_file`, `insert_segments`, and `normalize_language` to `pub(crate)`. Do not duplicate extraction rules and do not make them public outside the crate.

- [x] **Step 4: Seed and destroy an isolated database**

Implement `prepare_mv_mz_pilot(game_path, source_lang, target_lang, sample_size)` without `AppState`. Detect and reject every engine except `Engine::MvMz`, create `tempfile::TempDir`, initialize migrations, stream extracted files into a transaction, select the sample, build context with `llm::context::build_for_segments`, close the pool, drop the temporary directory, and return `database_removed = !db_path.exists()`.

- [x] **Step 5: Test extraction isolation with a synthetic game**

Build a minimal MV/MZ fixture in `TempDir`, snapshot every source file byte-for-byte, run preparation, and assert: the sample has context, no `.hoshi2star.json` or debug dump exists, source bytes are unchanged, and `database_removed` is true.

- [x] **Step 6: Run pilot tests**

Run: `cargo test commands::pilot::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 3: Expose IPC and frontend contracts

**Files:**
- Modify: `src-tauri/src/lib.rs`
- Modify: `src/lib/types.ts`
- Modify: `src/stores/llm.ts`
- Modify: `src/stores/llm.test.ts`

- [x] **Step 1: Register the preparation command**

Import `prepare_mv_mz_pilot` in `lib.rs` and add it to `tauri::generate_handler!`. The command remains state-free, so IPC cannot grant it access to the personal pool.

- [x] **Step 2: Mirror typed pilot and metric payloads in TypeScript**

Add `ProviderCallMetrics`, `PilotSampleCategory`, `PilotSampleSegment`, `PilotIsolation`, and `PilotPreparation` interfaces using the backend's camelCase field names.

- [x] **Step 3: Accumulate metrics in the LLM store**

Add `requestMetrics`, clear it when a translation starts, listen to `h2s://llm/metrics`, append each batch, and keep provider configuration unchanged on reset.

- [x] **Step 4: Test store metric lifecycle**

Assert that adding metrics preserves order and that reset clears metrics while retaining `providerConfig`.

- [x] **Step 5: Run frontend checks**

Run: `pnpm test src/stores/llm.test.ts`

Run: `pnpm typecheck`

Expected: PASS.

### Task 4: Verify safety and regression

**Files:**
- Modify only if a regression is found: files above
- Test: `src-tauri/tests/e2e_project_flow.rs`

- [x] **Step 1: Run static checks**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`

Run: `git diff --check`

Expected: PASS.

- [x] **Step 2: Run complete test suites**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Run: `pnpm test`

Run: `pnpm build`

Expected: PASS with only the existing intentional ignored Rust diagnostics and Vite chunk-size warning.

- [x] **Step 3: Run preparation against copied MV and MZ fixtures**

Prepare a 50-item sample from temporary copies of both fixtures. Assert each result has exactly 50 unique stable keys, only MV/MZ context logic, no source writes, and a deleted temporary database. Do not call any provider and do not modify the personal database.

- [x] **Step 4: Verify the personal database remains empty**

Run a read-only SQLite count for `projects`, `source_files`, `segments`, `tm_entries`, and `glossary_terms`.

Expected: `0|0|0|0|0`.
