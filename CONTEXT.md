# Hoshi2Star — CONTEXT.md

> **Hoshi2Star** (星 → ★) — Éditeur CAT + orchestrateur LLM pour la traduction
> fan de jeux RPG japonais. Transforme la 星 (hoshi) en ★ (star) : texte JP → toutes langues.
> App desktop Tauri v2, solo dev, production-ready, longévité 2–3 ans minimum.

---

## Dev Journal

`docs/journal/` — historique des sessions de développement.
Format fichiers : `YYYY-MM-DD-<sujet>.md` (ex: `2026-05-24-f0-setup.md`).
Template : `docs/journal/TEMPLATE.md`.

**Règles pour Claude Code :**
- Lire le journal le plus récent au début de chaque session
- Créer une nouvelle entrée à la fin de chaque session
- Documenter : ce qui a été fait, fichiers créés/modifiés, décisions prises, tâches ROADMAP cochées, prochaine session
- Ne jamais modifier une entrée passée — créer une nouvelle entrée si correction nécessaire

## Stack (versions verrouillées)

| Couche | Technologie | Note critique |
|--------|------------|---------------|
| Runtime desktop | **Tauri v2** (NOT v1) | API, imports et capabilities entièrement différents |
| Backend | **Rust** stable, tokio, sqlx 0.8 (sqlite), serde, thiserror | Async partout |
| Frontend | **React 19**, TypeScript strict (`"strict": true`) | Pas de `any` implicite |
| UI components | **shadcn/ui** (owned in `src/components/ui/`) | Jamais npm-installés |
| Data grid | **TanStack Table v8** + TanStack Virtual | Headless uniquement |
| État global | **Zustand** (slices typées) | Pas Redux, pas Context |
| Styling | **Tailwind CSS v4** | Via shadcn |
| Build / bundler | **Vite** (défaut Tauri) | Ne pas changer |
| Package manager | **pnpm UNIQUEMENT** | Jamais `npm` ni `yarn` |
| OS dev | CachyOS (Arch Linux) | webkit2gtk-4.1 requis |

---

## Architecture — 5 couches

```
┌─────────────────────────────────────────────┐
│  CAT UI (React)   src/                      │
│  Zustand stores · TanStack Table · shadcn   │
├─────────────────────────────────────────────┤
│  LLM Layer        src-tauri/src/llm/        │
│  Tokenizer · Provider router · Prompt chain │
├─────────────────────────────────────────────┤
│  Core Layer       src-tauri/src/core/       │
│  SQLite TM · Glossaire · QA engine · Diff   │
├─────────────────────────────────────────────┤
│  Engine Layer     src-tauri/src/engines/    │
│  mv_mz/ · vx_ace/ · wolf/ · bakin/         │
├─────────────────────────────────────────────┤
│  Export Layer     src-tauri/src/export/     │
│  Re-injector · Patch diff · Format .h2s     │
└─────────────────────────────────────────────┘
```

### Structure dossiers

```
hoshi2star/
├── CONTEXT.md                  ← ce fichier
├── docs/
│   ├── adr/                    ← Architecture Decision Records
│   ├── engines.md              ← formats par moteur RPG
│   └── llm-pipeline.md        ← détail des passes LLM
├── src/                        ← Frontend React
│   ├── components/ui/          ← shadcn (NE PAS ÉDITER MANUELLEMENT)
│   ├── components/editor/      ← composants CAT (SegmentGrid, TMPanel, etc.)
│   ├── features/               ← features verticales (project/, translate/, qa/)
│   ├── stores/                 ← Zustand slices (editor.ts, project.ts, llm.ts)
│   └── lib/                    ← utils, types partagés
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs              ← entrée app (PAS main.rs — requis mobile)
│   │   ├── commands/           ← tous les #[tauri::command]
│   │   ├── engines/            ← parsers par moteur
│   │   ├── core/               ← tm.rs, glossary.rs, qa.rs, diff.rs
│   │   ├── llm/                ← tokenizer.rs, provider.rs, pipeline.rs
│   │   ├── export/             ← reinjector.rs, patch.rs
│   │   ├── db/                 ← pool.rs, migrations/, queries/
│   │   └── state.rs            ← AppState (SqlitePool + config)
│   ├── capabilities/           ← ACL JSON (PAS allowlist dans tauri.conf.json)
│   ├── migrations/             ← sqlx migrate! files
│   └── Cargo.toml
├── .claude/
│   └── settings.json           ← hooks PreToolUse / PostToolUse
└── .mcp.json                   ← MCP servers (scope project)
```

---

## Frontière Rust ↔ TypeScript

**Règle absolue : tout accès données passe par `invoke()`. Jamais de DB direct depuis le JS.**

```typescript
// TS → Rust
import { invoke } from '@tauri-apps/api/core'          // ✅ v2
// import { invoke } from '@tauri-apps/api/tauri'       // ❌ v1 INTERDIT

const segments = await invoke<Segment[]>('get_segments', { projectId, fileId })
```

```rust
// Rust command — pattern obligatoire
#[tauri::command]
async fn get_segments(
    project_id: String,
    file_id: String,
    state: tauri::State<'_, AppState>,
) -> Result<Vec<Segment>, String> {          // Result<T, String> TOUJOURS
    todo!()
}
// Ajouter à generate_handler![get_segments, ...]  ← NE PAS OUBLIER
```

**Events Rust → TS** : `app.emit("h2s://qa-update", payload)` → `listen('h2s://qa-update', cb)`
Préfixer tous les events avec `h2s://` pour éviter les collisions.

---

## Domaine métier — vocabulaire Hoshi2Star

Utiliser ces termes exactement dans le code, les commentaires et les ADRs :

| Terme | Définition |
|-------|-----------|
| `Project` | Un jeu en cours de traduction (1 moteur, N fichiers) |
| `SourceFile` | Fichier extrait du jeu (Map001.json, Data.wolf, etc.) |
| `Segment` | Une unité de texte traduisible (source + target + status) |
| `TM` | Translation Memory — base SQLite des segments déjà traduits |
| `Glossary` | Paires terme source / terme cible par domaine |
| `Engine` | Moteur de jeu supporté (mv_mz, vx_ace, wolf, bakin) |
| `Placeholder` | Code d'échappement à préserver (`\V[n]`, `\C[n]`, `\N[n]`) |
| `Patch` | Fichier de diff exportable pour distribuer la traduction |
| `Pipeline` | Séquence de passes LLM (translate → review → tone → qa) |
| `Provider` | Backend LLM (Ollama, OpenAI, DeepSeek, DeepL) |

---

## Commandes de développement

```bash
pnpm tauri dev                                          # dev (Vite + Tauri)
pnpm tauri build                                        # release bundle
pnpm typecheck                                          # tsc --noEmit
pnpm test                                               # Vitest
pnpm lint                                               # ESLint + prettier check
cargo test --manifest-path src-tauri/Cargo.toml         # tests Rust
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml
```

**Avant tout commit** : `cargo fmt && cargo clippy -D warnings && pnpm typecheck`
(enforced par hooks PostToolUse)

---

## Conventions code

### Rust
- Erreurs : enum `H2sError` dérivant `thiserror::Error + serde::Serialize` — pas de `.unwrap()` hors tests
- State Tauri : `tauri::State<'_, AppState>` uniquement dans les commands, jamais dans les modules internes
- Async : `tokio::spawn` pour les tâches LLM longues (ne pas bloquer le thread IPC)
- Tests : unitaires dans le même fichier (`#[cfg(test)] mod tests`), intégration dans `tests/`

### TypeScript / React
- Composants : function components uniquement, pas de class components
- Zustand : `create<SliceState>()(...)` avec type explicite, sélecteurs exportés (`useActiveSegment`)
- TanStack Table : colonnes dans `features/<feature>/columns.tsx` séparé, jamais inline
- shadcn : ajouter via `pnpm dlx shadcn@latest add <component>` — ne jamais éditer `src/components/ui/*` manuellement sauf pour étendre
- Imports : paths aliases (`@/components`, `@/stores`, `@/lib`) — jamais de `../../../`

### Nommage
- Commands Tauri : `snake_case` côté Rust → `camelCase` dans l'objet args TS
- Events : `h2s://<domaine>/<action>` (ex: `h2s://qa/segment-error`)
- Fichiers Rust : `snake_case.rs`, dossiers : `snake_case/`
- Fichiers TS : `PascalCase.tsx` pour composants, `camelCase.ts` pour le reste

---

## Erreurs fréquentes — NE PAS FAIRE

```
❌  import { invoke } from '@tauri-apps/api/tauri'
✅  import { invoke } from '@tauri-apps/api/core'

❌  "allowlist": { "fs": { "all": true } }  dans tauri.conf.json
✅  src-tauri/capabilities/default.json avec permissions explicites

❌  tauri = { version = "1" } dans Cargo.toml
✅  version "2", plugins séparés (tauri-plugin-fs, tauri-plugin-dialog, etc.)

❌  npm install <pkg>  /  yarn add <pkg>
✅  pnpm add <pkg>

❌  npx shadcn-ui@latest add ...   (ancien package déprécié)
✅  pnpm dlx shadcn@latest add ...

❌  fn my_cmd(s: &str) -> String  (async command avec &str)
✅  fn my_cmd(s: String) -> Result<String, String>

❌  tauri::generate_handler![cmd_a]  puis plus loin  generate_handler![cmd_b]
✅  Un seul generate_handler![cmd_a, cmd_b, cmd_c]  dans lib.rs

❌  Traduire les placeholders \V[12] avec le LLM
✅  Tokeniser avant envoi → ⟦ph_001⟧ → restaurer après réponse

❌  WEBKIT_DISABLE_DMABUF_RENDERER non défini sur GPU NVIDIA (fenêtre blanche)
✅  export WEBKIT_DISABLE_DMABUF_RENDERER=1  dans ~/.zshrc (X11)
   ou export __NV_DISABLE_EXPLICIT_SYNC=1  (Wayland)
```

---

## MCP servers configurés

Voir `.mcp.json` (scope project). Résumé :

| MCP | Usage |
|-----|-------|
| `context7` | Docs Tauri v2, React 19, shadcn, TanStack, Zustand — toujours à jour |
| `filesystem` | Lecture/écriture fichiers projet (scopé à la racine) |
| `github` | Issues, PRs, code search |
| `rust-docs` | docs.rs + crates.io |
| `tauri` | Screenshots webview, DOM snapshot en dev (debug_assertions only) |

## Skills installés

```bash
# Lifecycle complet shadcn/ui avec détection pnpm automatique
npx skills add https://github.com/shadcn/ui --skill shadcn

# TDD red-green-refactor vertical slice (Rust + Vitest)
npx skills add https://github.com/mattpocock/skills --skill tdd

# Audit architecture "deep modules" (utiliser à partir du mois 5)
npx skills add https://github.com/mattpocock/skills --skill improve-codebase-architecture

# 69 règles React 19 : waterfalls, re-renders, bundle
npx skills add https://github.com/vercel-labs/agent-skills --skill vercel-react-best-practices

# Patterns Vitest + Testing Library
npx skills add https://github.com/anthropics/skills --skill webapp-testing
```

---

## Décisions d'architecture (ADRs)

Voir `docs/adr/` pour le détail. Résumé :

- **ADR-001** : SQLite via sqlx async (pas rusqlite sync, pas tauri-plugin-sql) — isolation DB côté Rust, pas d'accès JS direct
- **ADR-002** : Placeholder tokenisation Rust-side avant tout envoi LLM — UUID opaque → restauration post-réponse avec validation
- **ADR-003** : TM globale à l'installation (pas par projet) — fuzzy matching cross-projet = différenciateur clé vs Translator++
- **ADR-004** : MVP limité à RPG Maker MV/MZ uniquement — JSON natif, marché le plus large, complexité minimale
- **ADR-005** : `lib.rs` comme entrée (pas `main.rs`) — requis pour les builds mobiles futurs Tauri

---

## Progression du développement

Voir `ROADMAP.md` pour l'état actuel du projet.
Avant de coder une feature, vérifier dans la roadmap :
- La phase en cours (statut `[~]`)
- Les dépendances techniques (ex : pas de Wolf avant que F2 soit `[x]`)
- Le critère de sortie de la phase active

---

## Contexte produit (ne pas oublier lors des décisions techniques)

- **Solo dev**, ressources limitées — privilégier simplicité et maintenabilité sur exhaustivité
- **Cible** : traducteurs fans JP→EN/FR/ES/DE, communauté F95zone / Discord fan-trad
- **Concurrent principal** : Translator++ (Dreamsavior) — tableur NW.js, ~3K patrons Patreon
- **Différenciateur** : vrai UX CAT (TM fuzzy cross-projet, glossaire actif, QA live, diff-aware merge)
- **Monétisation** : one-shot 29 $ + 9 $/6 mois updates — pas d'abonnement mensuel (allergie fan-trad)
- **Langue source principale** : Japonais (90 %+ du marché RPG Maker/Wolf). Korean et Chinese source en add-ons optionnels (9 $ chacun) — voir F5 dans ROADMAP.md
- **MVP vendable** : mois 4 — MV/MZ + LLM pre-translation + TM exact match + QA placeholders