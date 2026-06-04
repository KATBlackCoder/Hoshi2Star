# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Always read `CONTEXT.md` before coding** — it is the authoritative reference for stack versions, architecture, conventions, and critical mistakes to avoid. Read `ROADMAP.md` before adding any feature to verify the current phase and task dependencies. Re-read `tasks/lessons.md` at the start of each session to avoid repeating past mistakes.

---

## Agent Behavior

### 1. Plan Mode Default
- Enter plan mode for **ANY non-trivial task** (3+ steps, architectural decisions, new Rust modules)
- Write the plan to `tasks/todo.md` with checkable items **before touching any code**
- If something breaks mid-task: **STOP**, re-plan, don't keep pushing
- Use plan mode for verification steps too, not just building
- Write detailed specs upfront to reduce ambiguity — this is a Tauri/Rust project, mistakes are expensive to revert

### 2. Subagent Strategy
- Use subagents liberally to keep the main context window clean
- Offload codebase exploration, research, and parallel analysis to subagents
- One focused task per subagent — no multi-purpose agents
- For complex problems (e.g., LLM pipeline design, engine parser architecture): throw more compute via subagents

### 3. Self-Improvement Loop
- After **ANY correction** from the user: append the lesson to `tasks/lessons.md`
- Format: `[YYYY-MM-DD] Mistake → Root cause → Rule to follow`
- Ruthlessly iterate on these lessons — the goal is zero repeated mistakes
- Re-read `tasks/lessons.md` at session start before touching any code

### 4. Verification Before Done
- **Never mark a task complete without proving it works**
- Mandatory verification gate: `pnpm typecheck && cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings && cargo test --manifest-path src-tauri/Cargo.toml`
- Diff behavior between `main` and your changes when relevant
- Ask yourself: *"Would a senior Rust/Tauri engineer approve this?"*
- Run tests, check logs, demonstrate correctness — don't assume

### 5. Scope Discipline
- **Never touch code outside the explicit scope of the task**
- No opportunistic refactors — log them as a separate task in `tasks/todo.md` instead
- Minimal footprint: if a change isn't required by the task, don't make it
- No temporary fixes, no `// TODO: fix later` hacks — senior developer standards

### 6. Inward Elegance
- For non-trivial changes, pause and ask: *"Is there a more elegant solution?"*
- If a fix feels hacky: *"Knowing everything I know now, implement the elegant solution"*
- Skip this for typos and obvious fixes — don't over-engineer small changes

### 7. Autonomous Execution
- When asked to fix a bug: **just fix it** — don't ask for hand-holding on obvious steps
- Point at errors and failing tests, then resolve them autonomously
- No failing `cargo clippy` or `pnpm typecheck` without being told how to fix

---

## Slash Commands

| Command | Action |
|---------|--------|
| `/plan` | Write plan to `tasks/todo.md` with checkable items before starting |
| `/start` | Re-read `CONTEXT.md`, `ROADMAP.md`, `tasks/lessons.md` before any implementation |
| `/track` | Mark items complete in `tasks/todo.md` as you progress |
| `/verify` | Run full gate: typecheck + clippy + tests + diff review |
| `/lesson` | Append correction to `tasks/lessons.md` with date, mistake, root cause, rule |

---

## What this project is

**Hoshi2Star** (星 → ★) is a CAT (Computer-Assisted Translation) editor + LLM orchestrator for Japanese RPG fan games. Desktop app built with **Tauri v2** (NOT v1), **React 19**, and **Rust**. Currently in skeleton/template state — real feature implementation starts at phase F1.

---

## Commands

```bash
# Development
pnpm tauri dev                  # Start Vite dev server + Tauri window
pnpm tauri:linux                # Linux dev with WebKit/NVIDIA workarounds

# Build
pnpm tauri build                # Production bundle

# Frontend checks
pnpm typecheck                  # tsc --noEmit (TypeScript strict check)
pnpm lint                       # ESLint + prettier check
pnpm test                       # Vitest

# Rust checks (run from repo root)
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml

# Run a single Rust test
cargo test --manifest-path src-tauri/Cargo.toml <test_name>

# Add shadcn components (ALWAYS use this, never npm/yarn)
pnpm dlx shadcn@latest add <component>
```

**Pre-commit gate** (enforced by hooks): `cargo fmt && cargo clippy -D warnings && pnpm typecheck`

Use **pnpm only** — never `npm install` or `yarn add`.

---

## Architecture

5-layer Rust backend, React frontend, all communication via Tauri IPC `invoke()`:

```
┌─────────────────────────────────────────────┐
│  CAT UI (React)   src/                      │
│  Zustand stores · TanStack Table · shadcn   │
├─────────────────────────────────────────────┤
│  LLM Layer        src-tauri/src/llm/        │
│  tokenizer · provider router · pipeline     │
├─────────────────────────────────────────────┤
│  Core Layer       src-tauri/src/core/       │
│  tm.rs · glossary.rs · qa.rs · diff.rs     │
├─────────────────────────────────────────────┤
│  Engine Layer     src-tauri/src/engines/    │
│  mv_mz/ · vx_ace/ · wolf/ · bakin/         │
├─────────────────────────────────────────────┤
│  Export Layer     src-tauri/src/export/     │
│  reinjector · patch diff · .h2s format     │
└─────────────────────────────────────────────┘
```

**Key boundaries:**
- All data access from TypeScript goes through `invoke()` — no direct DB access from JS
- Rust entry point is `lib.rs`, not `main.rs` (required for future mobile builds — ADR-005)
- Events from Rust to TS: `app.emit("h2s://<domain>/<action>", payload)` — always prefix with `h2s://`
- Security: `src-tauri/capabilities/default.json` (ACL JSON) — not `allowlist` in `tauri.conf.json`
- All Rust `#[tauri::command]` functions registered in a single `generate_handler![...]` in `lib.rs`

**State management:** Zustand typed slices (`src/stores/`) — no Redux, no React Context for global state

**Database:** SQLite via `sqlx` async (ADR-001) — pool in `AppState`, migrations in `src-tauri/migrations/`

---

## Rust command pattern

```rust
#[tauri::command]
async fn my_command(
    param: String,                        // String, not &str
    state: tauri::State<'_, AppState>,
) -> Result<MyType, String> {             // Always Result<T, String>
    todo!()
}
// Must be added to the single generate_handler![...] in lib.rs
```

**TypeScript call:**
```typescript
import { invoke } from '@tauri-apps/api/core'   // v2 — NOT from '@tauri-apps/api/tauri'
const result = await invoke<MyType>('my_command', { param: 'value' })
```

---

## Critical mistakes to avoid

| Wrong | Correct |
|-------|---------|
| `import { invoke } from '@tauri-apps/api/tauri'` | `from '@tauri-apps/api/core'` |
| `"allowlist"` in `tauri.conf.json` | `capabilities/default.json` with explicit permissions |
| `tauri = { version = "1" }` | `version = "2"` with separate plugins |
| `npm install` / `yarn add` | `pnpm add` |
| `npx shadcn-ui@latest add ...` | `pnpm dlx shadcn@latest add ...` |
| `fn cmd(s: &str) -> String` | `fn cmd(s: String) -> Result<String, String>` |
| Multiple `generate_handler![...]` calls | Single one in `lib.rs` |
| Sending `\V[12]` placeholders to LLM | Tokenize to `⟦ph_001⟧` first, restore after (ADR-002) |
| `.unwrap()` outside tests | Custom `H2sError` enum via `thiserror` |

**Linux GPU workarounds** (CachyOS, NVIDIA):
- Wayland (session actuelle): `export __NV_DISABLE_EXPLICIT_SYNC=1`
- X11: `export WEBKIT_DISABLE_DMABUF_RENDERER=1`
- Or use `pnpm tauri:linux` (configured for Wayland)

---

## Domain vocabulary

Use these exact terms in code, comments, and types:

| Term | Meaning |
|------|---------|
| `Project` | A game being translated (1 engine, N files) |
| `SourceFile` | Extracted game file (`Map001.json`, `Data.wolf`, etc.) |
| `Segment` | One translatable text unit (source + target + status) |
| `TM` | Translation Memory — global SQLite database (cross-project, ADR-003) |
| `Glossary` | Source/target term pairs by domain |
| `Engine` | Supported game engine: `mv_mz`, `vx_ace`, `wolf`, `bakin` |
| `Placeholder` | Escape codes to preserve (`\V[n]`, `\C[n]`, `\N[n]`) |
| `Patch` | Exportable diff for distributing translations |
| `Pipeline` | LLM pass sequence: translate → review → tone → qa |
| `Provider` | LLM backend (Ollama, OpenAI, DeepSeek, DeepL) |

---

## Naming conventions

- Tauri commands: `snake_case` (Rust) → `camelCase` in TS args object
- Events: `h2s://<domain>/<action>` (e.g., `h2s://qa/segment-error`)
- Rust files: `snake_case.rs`, dirs: `snake_case/`
- TS files: `PascalCase.tsx` for components, `camelCase.ts` for utilities
- Import aliases: `@/components`, `@/stores`, `@/lib` — never `../../../`
- shadcn components in `src/components/ui/` — add via CLI, do not manually edit

---

## Development phase status

See `ROADMAP.md` for current task status. Before implementing any feature, verify:
1. Which phase is currently active (`[~]`)
2. That prerequisite phases are complete (no Wolf engine before F2 is done)
3. The exit criterion for the active phase

> Do NOT rely on the phase status written here — always read `ROADMAP.md` directly for the source of truth.