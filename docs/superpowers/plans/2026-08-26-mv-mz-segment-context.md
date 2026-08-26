# MV/MZ Segment Context Implementation Plan

> **For Codex:** Execute this plan task by task with red-green-refactor tests. Do not infer context for VX Ace, Wolf RPG, or future engines: they must persist `unknown`/`NULL` until an engine-specific context extractor is implemented.

**Goal:** Preserve reliable RPG Maker MV/MZ translation context during extraction so future prompts can use semantic type, scene order, speaker, and branch boundaries without storing duplicated neighbouring text.

**Architecture:** Add a small engine-neutral persistence contract to `segments`, while MV/MZ owns the logic that populates it. Event-page extractors emit scene/order/speaker/branch facts; the project layer only normalizes and persists those facts. Other engines deliberately emit the empty contract. A later prompt builder will query neighbours by `scene_id` and `sequence_index` rather than embedding copies in every row.

**Tech Stack:** Rust, serde/serde_json, SQLx + SQLite migrations, Tauri IPC, TypeScript, Cargo tests, pnpm typecheck.

---

### Task 1: Add the durable segment-context contract

**Files:**

- Create: `src-tauri/migrations/0007_mv_mz_segment_context.sql`
- Modify: `src-tauri/src/domain/types.rs`
- Modify: `src-tauri/src/db/pool.rs`
- Modify: `src/lib/types.ts`
- Test: `src-tauri/src/db/pool.rs`

**Step 1: Write the failing migration/domain tests**

Add a migrated-DB test that asserts these columns and legacy defaults:

```rust
let row: (String, Option<String>, Option<i64>, Option<String>, Option<String>, Option<String>) =
    sqlx::query_as(
        "SELECT segment_kind, scene_id, sequence_index, speaker, branch_path, context_json \
         FROM segments WHERE id = 's1'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
assert_eq!(row, ("unknown".into(), None, None, None, None, None));
```

**Step 2: Run the focused test and verify it fails**

Run: `cargo test db::pool::tests::test_segment_context_defaults_are_backward_compatible --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because the context columns do not exist.

**Step 3: Implement the migration and mirrored types**

Migration columns:

```sql
ALTER TABLE segments ADD COLUMN segment_kind TEXT NOT NULL DEFAULT 'unknown';
ALTER TABLE segments ADD COLUMN scene_id TEXT;
ALTER TABLE segments ADD COLUMN sequence_index INTEGER;
ALTER TABLE segments ADD COLUMN speaker TEXT;
ALTER TABLE segments ADD COLUMN branch_path TEXT;
ALTER TABLE segments ADD COLUMN context_json TEXT;
CREATE INDEX IF NOT EXISTS idx_segments_scene_sequence
    ON segments(source_file_id, scene_id, sequence_index);
```

Add the six fields to Rust `Segment`, `SegmentSearchHit`, and TypeScript `Segment`. Extend `validate_patch_schema` so a newer experimental DB is accepted only if it has the complete current contract.

**Step 4: Run focused tests**

Run: `cargo test db::pool::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 2: Make MV/MZ extraction context-aware

**Files:**

- Modify: `src-tauri/src/engines/mv_mz/extractor.rs`
- Modify: `src-tauri/src/engines/mv_mz/injector.rs`
- Test: `src-tauri/src/engines/mv_mz/extractor.rs`

**Step 1: Write failing MV/MZ context tests**

Cover at least:

- MZ `101` speaker propagated to its following `401` lines;
- MV four-parameter `101` leaves `speaker = None`;
- every map event page gets a distinct stable `scene_id` and zero-based extracted `sequence_index`;
- common events and troop pages have their own scene IDs;
- choice/conditional branch arms get stable `branch_path` values and do not leak beyond their terminator;
- database records still receive kinds even when they have no event-scene context.

**Step 2: Run the focused extractor tests and verify they fail**

Run: `cargo test engines::mv_mz::extractor::tests --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because `ExtractedSegment` has no context.

**Step 3: Implement engine-owned context facts**

Add a typed MV/MZ context value:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SegmentContext {
    pub scene_id: Option<String>,
    pub sequence_index: Option<i64>,
    pub speaker: Option<String>,
    pub branch_path: Option<String>,
    pub context_json: Option<String>,
}
```

Give `SegmentKind` a stable `as_str()` representation. Pass the source filename into event-bearing dispatch so scene IDs can be namespaced as, for example, `Map008.json:event:12:page:0`. Track only branch markers defined by MV/MZ event commands; do not guess from text. Store compact engine facts such as command index and indent in `context_json` with a schema version.

Do not store neighbouring source strings. Preserve extractor ordering and assign `sequence_index` only when a segment is actually emitted.

**Step 4: Keep injection source-compatible**

Update injector call sites for the filename-aware map extraction API without changing JSON pointers or translated output.

**Step 5: Run MV/MZ extractor and injector tests**

Run: `cargo test engines::mv_mz --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 3: Persist context for MV/MZ only

**Files:**

- Modify: `src-tauri/src/commands/project.rs`
- Modify: `src-tauri/src/commands/search.rs`
- Test: `src-tauri/src/commands/project.rs`
- Test: `src-tauri/src/commands/search.rs`

**Step 1: Write failing normalization/persistence tests**

Add tests proving:

- MV/MZ normalized segments carry stable snake-case kind and extractor context;
- VX Ace and Wolf normalized segments use `segment_kind = "unknown"` with all context fields absent;
- batched insertion writes every context field;
- `get_segments`, `update_segment`, and search return the same context fields.

**Step 2: Run focused tests and verify they fail**

Run: `cargo test commands::project::tests commands::search::tests --manifest-path src-tauri/Cargo.toml`

Expected: FAIL on missing normalized fields/SQL projections.

**Step 3: Implement normalized persistence**

Replace the debug-only string with a normalized struct containing the six durable fields. Map MV/MZ extractor facts into it. Explicitly construct an empty context for VX Ace and Wolf; do not map their current debug kinds into the durable contract.

Expand the batch insert to ten bindings per row and lower the chunk size to remain below SQLite's conservative 999-parameter limit. Include context in the debug dump so extraction audits can inspect it.

Update every SQL projection that materializes `Segment` or `SegmentSearchHit`.

**Step 4: Run focused command tests**

Run: `cargo test commands::project::tests --manifest-path src-tauri/Cargo.toml`

Run: `cargo test commands::search::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 4: Verify real-game extraction without modifying originals

**Files:**

- Modify only if required by a discovered bug: files above
- Test: existing Rust and frontend test suites

**Step 1: Run formatting and static checks**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

Run: `pnpm typecheck`

Expected: PASS.

**Step 2: Run the complete backend suite**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

**Step 3: Audit copied MV and MZ fixtures**

Copy the selected games from `test/` into temporary directories, then run the existing extraction test/debug pathway only on those copies. Verify:

- extracted segment counts remain unchanged from the pre-context baseline;
- all persisted MV/MZ rows have a non-`unknown` semantic kind;
- event dialogue rows have scene/order context;
- MZ named dialogue has a speaker where `101.parameters[4]` provides one;
- no files under the original `test/` games changed;
- the personal application DB remains empty.

**Step 4: Record the boundary for the next phase**

The next prompt-builder phase may consume MV/MZ context and select nearby rows within the same scene. It must fall back to source text + glossary for engines whose durable context remains `unknown/null`.
