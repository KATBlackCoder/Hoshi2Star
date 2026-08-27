# Semantic QA Retry Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Retenter automatiquement, de façon bornée et isolée, une sortie LLM rejetée par le QA sémantique afin de réduire les expansions, fuites de contexte et reliquats de langue source sans affaiblir l'audit.

**Architecture:** Le pipeline conserve son premier appel contextuel. Un petit module `semantic_retry` reçoit uniquement les sorties fournisseur rejetées, les rejoue une par une avec les voisins retirés, puis sans métadonnées si nécessaire; chaque candidate repasse le QA complet avant d'être acceptée. Les traductions TM et les sorties déjà valides ne consomment aucun appel supplémentaire.

**Tech Stack:** Rust, Tokio, sqlx/SQLite, pipeline LLM générique, Tauri events, TypeScript, Ollama OpenAI-compatible.

---

### Task 1: Contrat de reprise sémantique

**Files:**
- Create: `src-tauri/src/llm/semantic_retry.rs`
- Modify: `src-tauri/src/llm/mod.rs`
- Modify: `src-tauri/src/llm/split.rs`
- Modify: `src/lib/types.ts`
- Test: `src/stores/llm.test.ts`

- [x] **Step 1: Étendre les métriques**

Ajouter `semantic_retries` et `semantic_recoveries` à `PipelineBatchMetrics` côté Rust et TypeScript, puis vérifier leur accumulation dans le store.

- [x] **Step 2: Écrire le module isolé**

Créer une fonction asynchrone qui reçoit source, sortie rejetée, tokenisation, contexte moteur, règles terminologiques et fournisseur. Elle tente au maximum deux nouvelles générations singleton : contexte moteur sans `previous/following`, puis contexte totalement absent.

- [x] **Step 3: Ne jamais contourner le QA**

Chaque candidate restaurée doit repasser `qa::check_with_context` avec les vrais voisins du segment. Retourner une nouvelle cible uniquement si aucune erreur critique ne subsiste; sinon conserver la première sortie et `needs_review`.

### Task 2: Intégration au pipeline avec TDD

**Files:**
- Modify: `src-tauri/src/llm/pipeline.rs`
- Test: `src-tauri/src/llm/pipeline.rs`

- [x] **Step 1: Écrire les tests rouges**

Ajouter un test où le premier appel copie un voisin et le second, singleton sans voisins, réussit. Ajouter un test où les deux reprises restent critiques et le segment reste `needs_review`. Vérifier le nombre d'appels et le contexte reçu par le mock.

- [x] **Step 2: Brancher seulement les sorties fournisseur rejetées**

Après restauration du premier résultat et avant persistance, exécuter la reprise uniquement pour `from_tm == false`, `provider_needs_review == false` et un QA critique. Les sorties valides conservent exactement le chemin actuel.

- [x] **Step 3: Vérifier la compatibilité**

Run: `cargo test --manifest-path src-tauri/Cargo.toml llm::pipeline`

Expected: les tests de contexte, placeholder, split, TM et reprise sémantique passent.

### Task 3: Reprise réelle des critiques StandGirl

**Files:**
- Modify: `src-tauri/tests/standgirl_full_pilot.rs`
- Modify: `src-tauri/tests/support/full_pilot/mod.rs`
- Modify: `src-tauri/tests/support/full_pilot/translation.rs`
- Modify: `src-tauri/tests/support/full_pilot/report.rs`
- Modify: `docs/validation/terminology-2026-08-27.md`

- [x] **Step 1: Ajouter un test privé dédié**

Le test ignoré reprend uniquement les segments `needs_review` de la variante terminologique existante, avec le même modèle et la même DB persistante. Il n'exécute ni nouveau scan ni nouvelle baseline.

- [x] **Step 2: Produire un rapport après reprise**

Écrire `terminology-retry.json` et `qa-details-after-retry.json` hors Git avec QA, tokens, appels, reprises/récupérations et statut d'export.

- [x] **Step 3: Rejouer avec Ollama**

Run: `H2S_STANDGIRL_PATH=... H2S_FULL_PILOT_ROOT=/tmp/hoshi2star-standgirl-full-pilot-20260827 H2S_OLLAMA_URL=http://127.0.0.1:11434/v1 H2S_OLLAMA_MODEL=gemma4:e4b cargo test --manifest-path src-tauri/Cargo.toml --test standgirl_full_pilot repair_standgirl_terminology_critical_segments -- --ignored --nocapture`

Expected: seuls les 15 segments critiques sont soumis au pipeline; tout reliquat critique est conservé en `needs_review`, jamais exporté silencieusement.

### Task 4: Validation globale et livraison

**Files:**
- Modify: `docs/superpowers/plans/2026-08-27-semantic-qa-retry.md`
- Modify: `docs/validation/terminology-2026-08-27.md`

- [x] **Step 1: Documenter les résultats sans causalité excessive**

Comparer avant/après reprise, noter les appels et tokens supplémentaires, les critiques restantes, l'export et l'empreinte intacte du jeu original.

- [x] **Step 2: Exécuter les gates**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Run: `pnpm typecheck && pnpm test`

Expected: toutes les suites passent; les pilotes privés restent ignorés par défaut.

- [x] **Step 3: Contrôler et livrer**

Vérifier `git diff --check`, l'absence de chemins privés/DB/ZIP/textes extraits, puis créer un commit conventionnel et pousser la branche sans force.
