# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

> **Always read `CONTEXT.md` before coding** — it is the authoritative reference for stack versions, architecture, conventions, and critical mistakes to avoid. Read `ROADMAP.md` before adding any feature to verify the current phase and task dependencies.

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
- X11: `export WEBKIT_DISABLE_DMABUF_RENDERER=1`
- Wayland: `export __NV_DISABLE_EXPLICIT_SYNC=1`
- Or use `pnpm tauri:linux` for X11 compositing workaround

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

Current MVP target: Phase F2 — MV/MZ + LLM pre-translation + TM exact match + QA placeholders.
