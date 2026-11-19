# Hoshi2Star — CONTEXT.md

> **Hoshi2Star** (星 → ★) — Éditeur CAT + orchestrateur LLM pour la traduction
> fan de jeux RPG japonais. Transforme la 星 (hoshi) en ★ (star) : texte JP → toutes langues.
> App desktop Tauri v2, solo dev, production-ready, longévité 2–3 ans minimum.

---

## Dev Journal

`docs/journal/` — historique des sessions de développement.
Format fichiers : `YYYY-MM-DD-<sujet>.md` (ex: `2026-05-24-f0-setup.md`).
Template : `docs/journal/TEMPLATE.md`.

**Règles communes à Codex et Claude Code — voir `AGENTS.md` :**
- Lire d'abord `tasks/handoff.md` et le journal qu'il référence au début de chaque session
- Vérifier l'état Git réel et lire `tasks/lessons.md` avant de poursuivre
- Actualiser `tasks/handoff.md` après chaque étape significative et avant de rendre la main
- Créer une nouvelle entrée à la fin de chaque session
- Documenter : outil utilisé, ce qui a été fait, fichiers créés/modifiés, décisions, corrections, preuves, limites et prochaine action
- Ne jamais modifier une entrée passée — créer une nouvelle entrée si correction nécessaire

## Stack (versions verrouillées)

| Couche | Technologie | Note critique |
|--------|------------|---------------|
| Runtime desktop | **Tauri v2** (NOT v1) | API, imports et capabilities entièrement différents |
| Backend | **Rust** stable, tokio, sqlx 0.8 (sqlite), serde, thiserror, marshal-rs 2.0.1 | Async partout |
| Frontend | **React 19**, TypeScript strict (`"strict": true`) | Pas de `any` implicite |
| UI components | **shadcn/ui** (owned in `src/components/ui/`) | Jamais npm-installés |
| Data grid | **TanStack Table v8** + TanStack Virtual | Headless uniquement |
| État global | **Zustand** (slices typées) | Pas Redux, pas Context |
| Styling | **Tailwind CSS v4** | Via shadcn |
| Build / bundler | **Vite** (défaut Tauri) | Ne pas changer |
| Package manager | **pnpm UNIQUEMENT** | Jamais `npm` ni `yarn` |
| OS dev | CachyOS (Arch Linux) | webkit2gtk-4.1 requis |

---

## Architecture

**Carte du code : [`docs/architecture.md`](docs/architecture.md)** — rôle de chaque dossier, parcours (ouvrir,
traduire, contrôler, exporter), « où changer quoi » et règles à ne pas casser. Elle n'est pas recopiée ici ; tout
lot qui déplace une frontière de module la met à jour dans la même PR.

En bref : interface React (`src/`) → IPC (`src-tauri/src/commands/`, un seul `generate_handler!` dans `lib.rs`) →
métier (`core/`) et LLM (`llm/`) → moteurs (`engines/`) → SQLite (`db/`). Il n'y a pas de dossier `export/` :
l'export est orchestré par `commands/export.rs`.

Repères qui changent la façon d'écrire du code :
- **Contexte projet** : `db/project_context.rs` (`ProjectContext::load`) lit une fois le projet ; le moteur est un
  `Engine` typé (`Engine::from_db_str`, `engines/detector.rs`), jamais une chaîne comparée à la main.
- **Mise en page du texte** : `core/text_fit/` (commun) + un adaptateur `TextBox` par moteur (ADR-009).
- **Wolf** : lecture et export par `WolfSources` (`engines/wolf/sources.rs`), fichiers libres de `Data/` seulement
  (ADR-008) ; sites de texte des commandes 101/102 dans `engines/wolf/sites.rs`.
- **unwolf** (ADR-010, UWF-D11) : la crate `unwolf` (déchiffrement des archives Wolf, clés et protection Pro) n'est
  importée **que** par `engines/wolf/unpack.rs` (et ses tests) ; le reste de l'app ne voit que `unpack_game`,
  `UnpackReport`, `UnpackProgress`, `UnpackError`. Dépendance crates.io `{ version = "0.2", default-features =
  false }`, jamais un chemin, Git ni une copie du code. Un manque d'unwolf se corrige dans unwolf, jamais par un
  contournement dans l'app. Mise à jour : `cargo update -p unwolf` (0.2.x) ou nouvelle version dans
  `src-tauri/Cargo.toml` (0.3…), puis adapter ce seul module, avis de licence
  `src-tauri/resources/licenses/unwolf-NOTICE.txt`, gate complet (procédure détaillée dans ADR-010).
- **Resync et couverture** : MV/MZ seulement (`engines/mv_mz/resync.rs`, `engines/mv_mz/coverage/`).
- **Terminologie** : `core/terminology/` ([architecture](docs/architecture/terminology.md)).
- **Front** : tout appel à Rust passe par `src/lib/api/<domaine>.ts` (ou `src/features/terminology/api.ts`) ; une
  différence entre moteurs passe par un prédicat de `src/lib/engine.ts` ; une erreur s'affiche avec
  `errorMessage()` (`src/lib/utils.ts`).

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
| `SourceFile` | Fichier extrait du jeu (Map001.json, Map001.mps, etc.) |
| `Segment` | Une unité de texte traduisible (source + target + status) |
| `TM` | Translation Memory — base SQLite des segments déjà traduits |
| `Glossary` | Paires terme source / terme cible par domaine |
| `Engine` | Moteur de jeu supporté (mv_mz, vx_ace, wolf, bakin) |
| `Placeholder` | Code d'échappement à préserver (`\V[n]`, `\C[n]`, `\N[n]`) |
| `Patch` | Fichier de diff exportable pour distribuer la traduction |
| `Pipeline` | Séquence de passes LLM (translate → review → tone → qa) |
| `Provider` | Backend LLM (Ollama, OpenAI, DeepSeek, DeepL) |
| `TextBox` | Adaptateur de mise en page d'un moteur (`MzTextBox`, `WolfTextBox`) pour l'échelle commune de `core/text_fit/` (ADR-009) |

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

## Workflow Git — branches

Toujours créer une branche avant de travailler :
```bash
git checkout -b <type>/<description-courte>
```

Types :
- `feat/`  → nouvelle feature
- `fix/`   → correction de bug
- `chore/` → maintenance, docs, config
- `plan/`  → création de plan uniquement

Merge uniquement quand :
- `cargo test` vert
- `cargo clippy -D warnings` vert
- `pnpm typecheck` vert
- Review visuelle si UI modifiée

```bash
# Commandes merge
git checkout main
git merge --no-ff <branche>
git push origin main
git branch -d <branche>
```

---

## Conventions code

### Rust
- Erreurs : `Result<T, String>` aux frontières IPC (commands, via `map_err(|e| e.to_string())`) ; enums `thiserror` **internes par module** (`ExtractorError`, `V3FormatError`, `DatParseError`, …) — pas d'enum global `H2sError` ; pas de `.unwrap()` hors tests
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

❌  struct Foo { pub my_field: String }  (serde serialize en snake_case)
✅  ajouter #[serde(rename_all = "camelCase")] sur chaque struct IPC
   (serde ne fait PAS de conversion automatique — Tauri v2 non plus)

❌  fn my_cmd(s: &str) -> String  (async command avec &str)
✅  fn my_cmd(s: String) -> Result<String, String>

❌  tauri::generate_handler![cmd_a]  puis plus loin  generate_handler![cmd_b]
✅  Un seul generate_handler![cmd_a, cmd_b, cmd_c]  dans lib.rs

❌  Traduire les placeholders \V[12] avec le LLM
✅  Tokeniser avant envoi → ⟦ph_001⟧ → restaurer après réponse

❌  WEBKIT_DISABLE_DMABUF_RENDERER non défini sur GPU NVIDIA (fenêtre blanche, X11)
✅  export __NV_DISABLE_EXPLICIT_SYNC=1  dans ~/.config/fish/config.fish (Wayland — env actuel)
   ou export WEBKIT_DISABLE_DMABUF_RENDERER=1  (X11 uniquement)
   → scripts : `pnpm tauri:linux` (Wayland) ; `pnpm tauri:linux:x11` (X11)

❌  WEBKIT_DISABLE_DMABUF_RENDERER=1 sous Wayland : rendu non accéléré, grille ~5× plus lente
   (95 ms contre 19 ms par frame, docs/benchmarks/2026-10-29-grid-app-reelle.md)

❌  cargo build sans .cargo/config.toml → marshal-rs échoue (gxhash exige AES+SSE2)
✅  .cargo/config.toml à la racine avec rustflags = ["-C", "target-cpu=native"]
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
- **ADR-004** : MVP limité à RPG Maker MV/MZ (JSON natif) — VX Ace ajouté en F3 via marshal-rs (Ruby Marshal binary)
- **ADR-005** : `lib.rs` comme entrée (pas `main.rs`) — requis pour les builds mobiles futurs Tauri
- **ADR-006** : dispatch moteur par `match` sur l'enum `Engine` (pas d'objets-traits) — le compilateur signale tout
  `match` oublié quand un moteur est ajouté
- **ADR-007** : mise à jour automatique par `tauri-plugin-updater` (GitHub Releases, signature obligatoire), limitée
  aux installations qui la supportent
- **ADR-008** : Wolf RPG lu et exporté depuis le `Data/` déchiffré par UberWolf, jamais depuis une archive `.wolf`.
  Détection : `Game.exe` / `GamePro.exe` / `Game.ini` + `Data/BasicData/` en clair ; encodage par la première base
  lisible (`SysDatabase.dat`, `DataBase.dat`, `CDataBase.dat`), sinon refus `wolf_no_database` ; `Data.wolf`,
  `Data/MapData.wolf` ou `Data/BasicData.wolf` présent ⇒ refus `wolf_archive_present` (`engines/wolf/sources.rs`,
  `WolfSources`). Procédure : `docs/engines/wolf-uberwolf.md`
- **ADR-010** : un jeu Wolf encore empaqueté est déchiffré dans l'app par la crate `unwolf` (seul importeur :
  `engines/wolf/unpack.rs`) : archives de texte seulement, protection Pro 3.5 levée (V1/V2 refusées), jeu contrôlé
  lisible, puis archives de texte **déplacées** (jamais supprimées) dans `<jeu>/Hoshi2Star-archives/`. Ouverture
  d'un jeu empaqueté ⇒ marqueur `wolf_packed` ; commandes `unpack_wolf_game`, `cancel_wolf_unpack` ; dialogue
  `WolfUnpackDialog`. Lecture et export inchangés (ADR-008)
- **ADR-009** : mise en page du texte commune dans `core/text_fit/` (réglages, police, jetons, découpe, échelle
  `fit()`, `MeasuredTextBox`, scan), un adaptateur `TextBox` par moteur (`MzTextBox`, `WolfTextBox`) à dispatch
  statique. Le moteur n'est lu qu'une fois par couche : `engine_fit()` (`core/text_fit/settings.rs`) et
  `FitEngine::of()` (`commands/text_fit.rs`). Ne rien importer de `engines/mv_mz/layout/` hors du moteur MZ, sauf
  la couche commande MZ (`commands/text_fit.rs`, `commands/qa.rs`). Preuve de non-régression : le corpus
  `src-tauri/tests/text_fit_corpus.rs`
- **Textes Wolf non traduisibles** (plan WF) : skippés à l'extraction, sans segment ; toutes les règles propres à
  Wolf sont dans `engines/wolf/filter.rs` (`extractor.rs` ne fait que les appeler). Règles et ajout d'une règle :
  `docs/engines/wolf-filter.md`

---

## Changelog

Fichier : `CHANGELOG.md` (Keep a Changelog format)
Skill : `.claude/skills/update-changelog/SKILL.md`

Règle : mettre à jour `CHANGELOG.md` à la fin de chaque session qui produit des changements.
Utiliser le skill `update-changelog`.

---

## Release process

### Checklist obligatoire avant de tagger

1. Bumper la version dans **les deux fichiers** — ils doivent toujours être identiques :
   - `src-tauri/Cargo.toml` → champ `version`
   - `src-tauri/tauri.conf.json` → champ `version`
2. Mettre à jour `CHANGELOG.md` : renommer `[Unreleased]` en `[X.Y.Z] — YYYY-MM-DD`
3. Committer + pusher sur `main`
4. Créer et pusher le tag : `git tag vX.Y.Z && git push origin vX.Y.Z`

### Ce que fait le workflow automatiquement

Le workflow `.github/workflows/release.yml` se déclenche sur `push: tags: v*` :
- Extrait la section `## [X.Y.Z]` du `CHANGELOG.md` comme notes de release
- Build Linux (AppImage, .deb, .rpm) + Windows (.msi, setup.exe)
- Crée une **release GitHub en DRAFT** avec les binaires attachés

### Après le workflow

- Aller sur GitHub → Releases → vérifier la draft
- Cliquer **Publish release** (ou `gh release edit vX.Y.Z --draft=false --latest`)

### Erreurs à ne pas répéter

| ❌ Erreur | ✅ Correct |
|-----------|-----------|
| Bumper seulement `Cargo.toml` | Bumper aussi `tauri.conf.json` — la `tauri-action` lit ce fichier pour `v__VERSION__` |
| Créer la release GitHub manuellement avant le workflow | Laisser le workflow créer la release — sinon les binaires ne s'attachent pas |
| Déclencher le workflow via `workflow_dispatch --ref main` sans tag | Le workflow doit se déclencher sur un tag pour que `tagName: v__VERSION__` pointe au bon endroit |

---

## RunPod — Setup Ollama

RunPod est utilisé comme provider LLM distant (alternative à Ollama local) via une URL HTTPS.

### Commande de démarrage (terminal RunPod)

```bash
bash -c "
  mkdir -p /workspace/ollama-models &&
  apt-get update && apt-get install -y zstd &&
  curl -fsSL https://ollama.com/install.sh | sh &&
  ollama serve &
  until ollama list > /dev/null 2>&1; do sleep 1; done &&
  ollama pull qwen3:4b-instruct-2507-q8_0 &&
  wait
"
```

**Pourquoi `until ollama list`** : `sleep 5` fixe n'est pas fiable — Ollama n'est pas toujours prêt en 5s au premier démarrage. Le poll toutes les secondes garantit que `ollama pull` s'exécute seulement quand le serveur répond.

**Si "ollama not found"** après l'install : ajouter `export PATH=$PATH:/usr/local/bin` juste après le `curl | sh` — le script d'install peut ne pas mettre à jour le PATH du shell courant.

### Connexion depuis Hoshi2Star

Dans Settings → URL Ollama, utiliser l'URL HTTPS publique du pod RunPod (ex: `https://xxxxx-11434.proxy.runpod.net`). Le fix rustls (v0.4.1) est requis — `native-tls` (OpenSSL) crashait en SIGABRT sur les URLs HTTPS.

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
- **Moteurs prioritaires** : MV/MZ ✅ → Wolf RPG F4 → autres post-stabilisation
- **VX Ace** : code disponible mais désactivé volontairement — réactiver uniquement quand Wolf RPG est stable
