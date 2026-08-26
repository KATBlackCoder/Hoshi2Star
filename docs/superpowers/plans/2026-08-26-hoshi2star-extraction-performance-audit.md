# Hoshi2Star Extraction Performance and Coverage Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Reduce extraction and translation overhead while proving that MV/MZ segments are genuine player-facing strings copied exactly from the original game JSON.

**Architecture:** Move blocking extraction to Tokio's blocking pool and stream extracted files through a bounded channel into batched SQLite writes. Keep TanStack Table's stable, virtualized data model, but remove avoidable array copies and coalesce translation updates. Characterization tests discover the real MV/MZ fixtures, compare every extracted JSON Pointer with its original value, and enforce coverage for supported player-facing event commands.

**Tech Stack:** Rust, Tokio, SQLx/SQLite, Tauri v2 mock runtime and MCP bridge, React 19, TypeScript, TanStack Table/Virtual, Vitest.

---

### Task 1: Real fixture discovery and extraction characterization

**Files:**
- Modify: `src-tauri/tests/e2e_project_flow.rs`
- Modify: `src-tauri/src/engines/mv_mz/extractor.rs`

- [x] **Step 1: Replace stale fixture names with structural discovery**

Add helpers that require one fixture containing `www/data/System.json` and one containing `data/System.json`. MV/MZ tests must fail with an explicit message when the workspace fixtures are missing; they must never return early while reporting success.

- [x] **Step 2: Add exact JSON Pointer verification**

For each debug-dump segment, parse the corresponding original JSON file and assert:

```rust
let original = source_json
    .pointer(key)
    .and_then(serde_json::Value::as_str)
    .expect("extracted key must point to an original string");
assert_eq!(source_text, original);
```

Run the assertion for both the MV and MZ fixtures.

- [x] **Step 3: Add player-facing event coverage verification**

Recursively enumerate event commands and require extraction for code `101` speaker names, `401` dialogue, `405` scrolling text, `102` choices, and MZ `TextPicture/set/text` plugin arguments. Compare the expected `(JSON Pointer, text)` set with the debug dump.

- [x] **Step 4: Preserve the original game title**

Change `extract_system` so `/gameTitle` stores the exact title rather than appending an Hoshi2Star suffix. Update the unit test to expect the original title.

- [x] **Step 5: Implement missing current-game text sources**

Extend `extract_event_list` with code `405` and the narrowly scoped MZ `TextPicture` command:

```rust
405 => extract_string_parameter(params, 0, SegmentKind::ScrollingText),
357 if plugin == "TextPicture" && command == "set" => {
    extract_argument("text", SegmentKind::PluginText)
}
```

Keep editor-only plugin labels and arbitrary plugin arguments out of the translatable set.

- [x] **Step 6: Run extractor and integration tests**

Run:

```bash
cargo test engines::mv_mz::extractor
cargo test --test e2e_project_flow -- --nocapture
```

Expected: unit tests pass, real MV and MZ audits execute for seconds rather than returning in milliseconds, and every extracted pointer matches its original JSON string.

### Task 2: Bounded extraction and batched project persistence

**Files:**
- Modify: `src-tauri/src/commands/project.rs`
- Test: `src-tauri/tests/e2e_project_flow.rs`

- [x] **Step 1: Characterize deterministic extraction order**

Keep the existing dump-versus-database ordered comparison and run it before refactoring.

- [x] **Step 2: Collect MV/MZ file descriptors without retaining parsed JSON**

Replace the `Vec<(..., serde_json::Value)>` collector with sorted path metadata. Read, parse, extract, and drop one JSON file at a time.

- [x] **Step 3: Stream files through a bounded channel**

Run synchronous parsing in `tokio::task::spawn_blocking`, sending at most two `ExtractedFile` values ahead with `tokio::sync::mpsc::channel(2)`. Consume files asynchronously while writing the project transaction.

- [x] **Step 4: Batch segment inserts**

Use `sqlx::QueryBuilder<Sqlite>` with chunks of 200 rows, staying below SQLite's bind-variable limit:

```rust
query.push_values(segments, |mut row, segment| {
    row.push_bind(uuid::Uuid::new_v4().to_string())
        .push_bind(file_id)
        .push_bind(&segment.key)
        .push_bind(&segment.source_text);
});
```

- [x] **Step 5: Move debug extraction off the async runtime**

Wrap detection, extraction, and JSON serialization in `spawn_blocking`, mapping both join failures and extraction failures to command errors.

- [x] **Step 6: Re-run deterministic and round-trip tests**

Expected: source-file and segment order remain identical, export still injects translations, and the personal database is never used by tests.

### Task 3: Translation-memory and persistence batching

**Files:**
- Modify: `src-tauri/src/core/tm.rs`
- Modify: `src-tauri/src/llm/batch.rs`
- Modify: `src-tauri/src/llm/pipeline.rs`

- [x] **Step 1: Add batch exact lookup tests**

Insert entries for two language pairs and assert `lookup_exact_many` returns only requested hashes for the requested pair, including an empty-input fast path.

- [x] **Step 2: Implement one-query TM lookup per translation batch**

Build a parameterized `IN` query with `QueryBuilder`, filter by `lang_pair`, and return a hash-keyed map. The pipeline batch is clamped to 100, so it remains below SQLite limits.

- [x] **Step 3: Borrow translation batches**

Change `dedup_by_hash` to accept `&[(String, String)]`. Iterate `segments.chunks(batch_size)` directly in `run_inner`, append ordered results, and remove the global `seg_map`, `all_ids`, `id_to_idx`, and grouped-ID allocations.

- [x] **Step 4: Persist each result batch in one transaction**

Open one SQLx transaction per completed provider batch, execute the updates through that transaction, and commit before emitting the frontend update event.

- [x] **Step 5: Run pipeline and TM tests**

Run:

```bash
cargo test core::tm
cargo test llm::batch
cargo test llm::pipeline
```

Expected: TM hits still skip the provider, adaptive splitting remains sequential, ordering remains stable, and updates remain crash-bounded to one in-flight batch.

### Task 4: TanStack update-path allocation reduction

**Files:**
- Modify: `src/components/editor/SegmentGrid.tsx`
- Modify: `src/components/editor/SegmentGrid.test.tsx`

- [x] **Step 1: Add a multi-event update test**

Emit multiple `h2s://llm/segments-updated` events and assert the final grid reflects all updates without losing row IDs or selection state.

- [x] **Step 2: Remove page concatenation copies**

Replace `all = all.concat(result.items)` with `all.push(...result.items)`. Keep the measured 2,000-row page because the current real fixtures top out below that per file and TanStack Virtual already bounds rendered DOM rows.

- [x] **Step 3: Coalesce translation events**

Buffer updates by segment ID in a `Map`, flush them once per animation frame, and perform one immutable list update for all events received in that frame. Cancel the frame and clear the buffer on unmount or file switch.

- [x] **Step 4: Run frontend tests and type checking**

Run:

```bash
pnpm test
pnpm typecheck
```

Expected: existing pagination, retry, density, save, selection, and translation-update behavior remains green.

### Task 5: MCP performance and content verification

**Files:**
- No source changes required after Tasks 1-4

- [x] **Step 1: Launch the Tauri debug app with the MCP bridge**

Run the normal Linux Tauri development command and connect the MCP driver to port 9223.

- [x] **Step 2: Extract temporary MV/MZ copies through MCP**

Invoke `debug_dump_segments` from the webview, capture command durations through the IPC monitor, and read process RSS before and after each single extraction. Do not invoke `open_project` against the personal database.

- [x] **Step 3: Inspect the post-fix dumps**

Confirm title strings equal `System.json`, MZ scrolling text and `TextPicture` content are present, and pure numeric/symbol/placeholder-only strings are absent.

- [x] **Step 4: Run final quality gates**

Run:

```bash
cargo fmt --check
cargo test
pnpm test
pnpm typecheck
pnpm build
git diff --check
```

Expected: all checks pass with no writes to the original game fixtures or personal database.
