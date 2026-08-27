# StandGirl Full Terminology Pilot Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Comparer sur les 1 191 segments de StandGirl une traduction ja→en Ollama sans terminologie et la même traduction avec une bibliothèque issue du scan MV/MZ.

**Architecture:** Un test d'intégration ignoré par défaut orchestre deux workspaces persistants et reprenables, chacun avec sa copie du jeu et sa DB SQLite. Les responsabilités de préparation, terminologie, traduction et rapport sont séparées en petits modules sous `tests/support/full_pilot/`; aucune donnée privée ou sortie de jeu n'entre dans Git.

**Tech Stack:** Rust, Tokio, sqlx/SQLite, Tauri mock runtime, pipeline LLM Hoshi2Star, Ollama OpenAI-compatible, serde JSON.

---

### Task 1: Workspace isolé et reprenable

**Files:**
- Create: `src-tauri/tests/standgirl_full_pilot.rs`
- Create: `src-tauri/tests/support/full_pilot/mod.rs`
- Create: `src-tauri/tests/support/full_pilot/workspace.rs`

- [x] **Step 1: Définir la configuration explicite**

Le test exige `H2S_STANDGIRL_PATH` et `H2S_FULL_PILOT_ROOT`; il accepte `H2S_OLLAMA_URL` et `H2S_OLLAMA_MODEL`. Aucun chemin privé n'est codé en dur.

- [x] **Step 2: Copier une fois chaque variante**

Créer `baseline/game`, `baseline/pilot.db`, `terminology/game`, `terminology/pilot.db`. Si la DB existe, rouvrir le projet par son manifest au lieu de réextraire.

- [x] **Step 3: Verrouiller les invariants**

Vérifier 1 191 segments dans chaque DB et calculer une empreinte stable `(file_name, json_key, source_text)` identique entre variantes.

- [x] **Step 4: Tester la préparation sans fournisseur**

Run: `cargo test --manifest-path src-tauri/Cargo.toml --test standgirl_full_pilot --no-run`

Expected: compilation réussie; le test reste `ignored` par défaut.

### Task 2: Bibliothèque proposée et curation minimale

**Files:**
- Create: `src-tauri/tests/support/full_pilot/terminology.rs`

- [x] **Step 1: Scanner toute la variante terminology**

Appeler le service long-vivant; accepter un scan froid ou un re-scan no-op.

- [x] **Step 2: Sélectionner les entités structurées**

Limiter aux types `character`, `speaker`, `item`, `weapon`, `armor`, `skill`, `enemy`, `state`, `class`, `place`, `title`. Traduire seulement les entrées sans cible anglaise, par groupes de trois pour limiter les échecs de protocole local.

- [x] **Step 3: Conserver toutes les sorties IA en proposed**

Le runner vérifie que le modèle n'a promu aucune valeur. Seuls `六花 → Rikka` et `凛 → Rin`, connus et vérifiés, deviennent `locked + required`; les autres restent des aides révisables.

- [x] **Step 4: Écrire un snapshot de révision**

Écrire hors Git `terminology-review.json` avec source, cible, type, nature, confiance et état.

### Task 3: Traductions complètes avec checkpoints

**Files:**
- Create: `src-tauri/tests/support/full_pilot/translation.rs`

- [x] **Step 1: Reprendre uniquement les segments non terminés**

Pour chaque fichier, charger les segments `untranslated` ou à cible vide, dans l'ordre stable. Appeler `pipeline::run_inner` avec contexte MV/MZ, batch 20, contexte moteur actif et délai nul.

- [x] **Step 2: Journaliser après chaque fichier**

Ajouter les `ProviderCallMetrics` à `provider-metrics.jsonl` et écrire `progress.json`. Une interruption ne doit pas retraduire les segments déjà persistés.

- [x] **Step 3: Exécuter baseline puis terminology**

La baseline ne lance jamais le scan. La variante terminology utilise le resolver normal; aucun hint n'est injecté manuellement dans le runner.

### Task 4: Rapport comparatif et export

**Files:**
- Create: `src-tauri/tests/support/full_pilot/report.rs`
- Create at runtime only: `$H2S_FULL_PILOT_ROOT/comparison.json`
- Modify: `docs/validation/terminology-2026-08-27.md`

- [x] **Step 1: Agréger les métriques**

Pour chaque variante: appels, tokens prompt/réponse, durée fournisseur, tentatives, hints, segments translated/needs_review, QA total/critique, erreurs par type, incohérences de sources répétées.

- [x] **Step 2: Comparer sans inventer une causalité**

Rapporter les écarts de cohérence, QA, tokens et durée. Le modèle n'étant pas déterministe, ne pas attribuer toute différence à la bibliothèque; signaler les variantes de noms observées.

- [x] **Step 3: Tester l'export**

Tenter un ZIP par variante. En cas de QA critique, enregistrer l'erreur de blocage au lieu de contourner l'audit.

- [x] **Step 4: Vérifier et committer uniquement le runner et la documentation**

Run: `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check`

Run: `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings`

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: toutes les suites publiques passent; aucun jeu, DB, ZIP, clé ou texte privé n'est indexé.
