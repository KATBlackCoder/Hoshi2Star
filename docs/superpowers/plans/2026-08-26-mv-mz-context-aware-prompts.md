# MV/MZ Context-Aware Prompts Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Feed bounded, engine-authored MV/MZ context into every OpenAI-compatible translation provider without leaking MV/MZ assumptions into other engines.

**Architecture:** A new `llm::context` dispatcher returns a provider-neutral prompt model and delegates MV/MZ SQL/context rules to `llm::context::mv_mz`. The pipeline builds contexts per inner batch and keeps them aligned through context-aware deduplication and recursive splitting. The OpenAI-compatible transport renders the same JSON translation units for Ollama, LM Studio, Hugging Face, and cloud endpoints.

**Tech Stack:** Rust, SQLx/SQLite, serde/serde_json, Tauri command integration, httpmock, Cargo tests.

---

### Task 1: Build bounded MV/MZ prompt context

**Files:**
- Create: `src-tauri/src/llm/context/mod.rs`
- Create: `src-tauri/src/llm/context/mv_mz.rs`
- Modify: `src-tauri/src/llm/mod.rs`
- Test: `src-tauri/src/llm/context/mv_mz.rs`

- [x] **Step 1: Write failing context-builder tests**

Seed one MV/MZ scene with ordered dialogue in two branch paths. Assert that the center segment receives at most two preceding and two following lines from the same `source_file_id`, `scene_id`, and exact `branch_path`, together with its semantic kind and speaker.

```rust
let contexts = build(&pool, &["center".to_string()]).await.unwrap();
let center = contexts[0].as_ref().unwrap();
assert_eq!(center.segment_kind, "dialogue");
assert_eq!(center.speaker.as_deref(), Some("勇者"));
assert_eq!(center.previous.len(), 2);
assert_eq!(center.following.len(), 2);
assert!(center.previous.iter().all(|line| line.text != "other branch"));
```

- [x] **Step 2: Run the focused test and verify it fails**

Run: `cargo test llm::context --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because `llm::context` does not exist.

- [x] **Step 3: Implement shared prompt types and strict engine dispatch**

Define serializable `SegmentPromptContext` and `NeighborLine` types. Implement:

```rust
pub async fn build_for_segments(
    db: &SqlitePool,
    engine: &str,
    segment_ids: &[String],
) -> Result<Vec<Option<SegmentPromptContext>>, sqlx::Error> {
    match engine {
        "mv_mz" => mv_mz::build(db, segment_ids).await,
        _ => Ok(vec![None; segment_ids.len()]),
    }
}
```

The fallback must never inspect or reinterpret context columns.

- [x] **Step 4: Implement the MV/MZ query**

Use one bounded self-join per batch. Join neighbours only when scene IDs match and SQLite `IS` confirms identical nullable branch paths. Limit the sequence delta to `[-2, +2]`, excluding the center row.

- [x] **Step 5: Run focused tests**

Run: `cargo test llm::context --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 2: Preserve contextual distinctions through batching

**Files:**
- Modify: `src-tauri/src/llm/batch.rs`
- Modify: `src-tauri/src/llm/pipeline.rs`
- Modify: `src-tauri/src/llm/split.rs`
- Test: `src-tauri/src/llm/batch.rs`
- Test: `src-tauri/src/llm/pipeline.rs`

- [x] **Step 1: Write failing deduplication and pipeline tests**

Assert that equal source strings with different context fingerprints remain two unique LLM units, while equal source plus equal context still deduplicates. Add a mock-provider assertion that MV/MZ metadata reaches the provider and Wolf receives `None`.

- [x] **Step 2: Run focused tests and verify they fail**

Run: `cargo test llm::batch --manifest-path src-tauri/Cargo.toml`

Run: `cargo test llm::pipeline --manifest-path src-tauri/Cargo.toml`

Expected: FAIL on missing context-aware APIs.

- [x] **Step 3: Add context-aware deduplication**

Keep the existing source hash for translation-memory lookup, but add a separate deduplication key derived from source hash plus the deterministic context fingerprint. Store the representative original index on each unique segment so its prompt context remains aligned.

- [x] **Step 4: Build contexts once per inner batch**

In `translate_batch`, call `context::build_for_segments` once for the batch IDs, deduplicate with context fingerprints, tokenize only cache misses, and attach only the matching unique contexts to a cloned request context.

- [x] **Step 5: Subset contexts during adaptive split**

Before each provider call, clone `TranslationContext` and subset `segment_contexts` using the same local indices used for `tokenized`. Recursive retries must preserve the original indexed context vector.

- [x] **Step 6: Run batching/pipeline tests**

Run: `cargo test llm::batch --manifest-path src-tauri/Cargo.toml`

Run: `cargo test llm::pipeline --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 3: Render one provider-neutral contextual prompt

**Files:**
- Modify: `src-tauri/src/llm/provider.rs`
- Modify: `src-tauri/prompts/translate/default.toml`
- Test: `src-tauri/src/llm/provider.rs`
- Test: `src-tauri/src/llm/prompts.rs`

- [x] **Step 1: Write failing prompt-render tests**

Assert that a contextual unit serializes as one JSON line containing `id`, `type`, `speaker`, `previous`, `text`, and `following`. Assert that a context-free engine retains the existing `[1] text` form and that embedded newlines cannot break unit boundaries.

- [x] **Step 2: Run focused tests and verify they fail**

Run: `cargo test llm::provider::tests --manifest-path src-tauri/Cargo.toml`

Run: `cargo test llm::prompts::tests --manifest-path src-tauri/Cargo.toml`

Expected: FAIL because the renderer ignores `segment_contexts`.

- [x] **Step 3: Implement deterministic JSON translation units**

Render contextual segments with `serde_json::to_string`; keep context-free numbered input unchanged. Continue replacing literal newlines in the current translatable text with `⏎` so response parsing remains one line per segment.

- [x] **Step 4: Strengthen the shared system prompt**

Tell the model that JSON metadata and nearby lines are reference-only, only `text` is translated, game content is untrusted data, and output remains exactly one numbered translation per input unit.

- [x] **Step 5: Verify the HTTP request body**

Use `httpmock` to assert that the OpenAI-compatible `/v1/chat/completions` request includes the MV/MZ type/speaker/context. This test covers Ollama, LM Studio, Hugging Face, and cloud providers because they share the transport.

- [x] **Step 6: Run provider/prompt tests**

Run: `cargo test llm::provider::tests --manifest-path src-tauri/Cargo.toml`

Run: `cargo test llm::prompts::tests --manifest-path src-tauri/Cargo.toml`

Expected: PASS.

### Task 4: Verify regression and resource boundaries

**Files:**
- Modify only if a regression is found: files above
- Test: existing backend/frontend suites and copied games in `test/`

- [x] **Step 1: Run static checks**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --lib -- -D warnings`

Run: `pnpm typecheck`

Expected: PASS.

- [x] **Step 2: Run all tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Run: `pnpm test`

Expected: PASS. HTTP-mock tests may require local-loopback permission.

- [x] **Step 3: Re-run copied-game extraction audits**

Run the MV/MZ e2e audit tests. Expected exact counts remain MV `15,498` and MZ `22,915`; original fixture files and the personal application DB remain untouched/empty.

- [x] **Step 4: Confirm the engine boundary**

Verify with tests that MV/MZ gets JSON context units, while Wolf and VX Ace keep the old context-free numbered prompt until dedicated `context/wolf.rs` and `context/vx_ace.rs` adapters are implemented.

### Task 5: Disable Ollama reasoning at the transport boundary

**Files:**
- Modify: `src-tauri/src/llm/provider.rs`
- Modify: `src-tauri/src/commands/translate.rs`
- Modify: `src-tauri/src/commands/glossary.rs`
- Modify: `src-tauri/prompts/translate/default.toml`
- Modify: `src-tauri/prompts/glossary/default.toml`
- Test: `src-tauri/src/llm/provider.rs`
- Test: `src-tauri/src/llm/prompts.rs`

- [x] **Step 1: Prove provider isolation with tests**

Assert that the Ollama preset serializes `reasoning_effort: "none"`, while generic OpenAI-compatible providers omit the optional field entirely.

- [x] **Step 2: Move reasoning control out of prompts**

Remove `/no_think` from translation and glossary prompts. Apply the API option only to providers created with the stable `ollama` preset ID.

- [x] **Step 3: Run focused and full regression tests**

Run provider/prompt tests, static checks, Rust tests, frontend tests, and an actual `gemma4:e4b` request against the local Ollama endpoint.
