# Journal — 2026-05-24 — F1 Engine Layer MV/MZ + Tokenizer

**Phase** : F1
**Durée estimée** : 3h
**Statut** : ✅ Complété (Engine Layer terminé, 46 tests verts)

---

## Ce qui a été fait

- Lecture complète de `docs/engines/mv-mz-placeholders.md` avant de toucher au code
- Décision : implémentation custom plutôt que `rvpacker-txt-rs` — plus de contrôle, pas de dépendance externe fragile
- Implémentation TDD (tests écrits avant le code) pour tous les modules
- Ajout de `regex = "1"` et `tempfile = "3"` (dev) dans `Cargo.toml`
- Déclaration des modules `engines` et `llm` dans `lib.rs`
- **extractor.rs** : extraction complète depuis tous les types de fichiers MV/MZ
  - Codes event 101 (speaker MZ), 401 (dialogue), 102 (choices)
  - Fichiers : Map, CommonEvents, Troops, Actors, Classes, Items, Weapons, Armors, Skills, Enemies, States, MapInfos, System
  - Clés JSON Pointer RFC 6901 pour round-trip exact avec l'injector
  - 12 tests unitaires
- **injector.rs** : réécriture JSON via `pointer_mut()` de serde_json
  - Round-trip `extract → inject → JSON identique` pour Map et Items
  - 6 tests unitaires dont 2 round-trip
- **decryptor.rs** : décryptage `.rpgmvp/.rpgmvo/.rpgmvm`
  - Accepte magic `RPGMV` et `RPGMZ` (byte 4 = `V` ou `Z`)
  - XOR bytes [16..32] avec la clé hex 16 bytes de `System.json`
  - 8 tests unitaires (round-trip MV + MZ, erreurs : magic, longueur, clé invalide)
- **detector.rs** : détection moteur depuis un dossier jeu
  - Cherche `data/` puis `www/data/` + `System.json` avec `gameTitle`
  - Fonction pure `is_mv_mz_system(&str)` testable sans I/O
  - 9 tests unitaires (dont 4 avec `tempfile`)
- **tokenizer.rs** (avance sur F2) : tokenisation placeholders selon `mv-mz-placeholders.md`
  - Deux variantes : `Engine::MvMz` (A+B+D[%n]) et `Engine::MzOnly` (C+A+B+D%n)
  - Groupe C avant Groupe A (évite `\P` qui capturerait `\PX`/`\PY`/`\FS`)
  - Tokens opaques `⟦ph_0⟧`, `⟦ph_1⟧`… avec map UUID→original
  - `tokenize` / `validate` / `restore` — `restore` appelle `validate` en interne
  - Les 10 tests obligatoires du spec tous verts
- **Correctif regex** : les escape sequences `\!`, `\>`, `\<`, `\^`, `\{`, `\}` ne sont pas valides dans les classes de caractères du crate `regex`. Correction : utiliser les caractères directs `[G\\$.|!><^{}]` au lieu de `[G\\\$\.\|\!\>\<\^\{\}]`.
- `cargo fmt` ✅ | `cargo clippy -D warnings` ✅ | `cargo test` ✅ (46/46 passés)
- ROADMAP.md mis à jour

## Fichiers créés

- `src-tauri/src/engines/mod.rs`
- `src-tauri/src/engines/mv_mz/mod.rs`
- `src-tauri/src/engines/mv_mz/extractor.rs` — 12 tests
- `src-tauri/src/engines/mv_mz/injector.rs` — 6 tests
- `src-tauri/src/engines/mv_mz/decryptor.rs` — 8 tests
- `src-tauri/src/engines/detector.rs` — 9 tests
- `src-tauri/src/llm/mod.rs`
- `src-tauri/src/llm/tokenizer.rs` — 10 tests (spec complet)

## Fichiers modifiés

- `src-tauri/Cargo.toml` — ajout `regex = "1"`, `[dev-dependencies] tempfile = "3"`
- `src-tauri/src/lib.rs` — déclaration `pub mod engines;` et `pub mod llm;`
- `ROADMAP.md` — items F1 Engine Layer cochés, tokenizer F2 marqué fait

## Dépendances ajoutées

**Rust (Cargo.toml):**
- `regex = "1"` — compilation des patterns placeholders
- `tempfile = "3"` (dev-only) — tests d'I/O du detector

## Décisions prises

- **Implémentation custom vs `rvpacker-txt-rs`** : custom retenu — la lib externe a une interface orientée fichiers texte flat, pas des `serde_json::Value`. Notre approche JSON Pointer est plus propre pour le round-trip. Item ROADMAP marqué `[-]`.
- **Clés JSON Pointer RFC 6901** : format choisi pour les clés de segments. `serde_json::Value::pointer()` et `pointer_mut()` sont natifs, pas de dépendance supplémentaire.
- **Regex Groupe B** : la spec `mv-mz-placeholders.md` utilise `\\[G\\\$\.\|\!\>\<\^\{\}]` qui n'est PAS valide dans le crate `regex` (escape sequences invalides dans une classe de caractères). La version correcte est `\\[G\\$.|!><^{}]`. Ce pattern est équivalent et accepté.
- **Tokenizer en F1** : le tokenizer était en F2 dans ROADMAP mais requis dans les instructions de session F1. Implémenté en avance, marqué `[x]` en F2.

## Problèmes rencontrés

- **Regex escape sequences invalides** : `\!`, `\>`, `\<`, `\^`, `\{`, `\}` dans une classe de caractères `[...]` sont refusées par le crate `regex` (clippy les détecte comme `clippy::invalid_regex`). Résolu en utilisant les caractères directs sans backslash inutile.

## Tâches ROADMAP cochées

- [x] `src-tauri/src/engines/mv_mz/extractor.rs`
- [x] `src-tauri/src/engines/mv_mz/injector.rs`
- [x] `src-tauri/src/engines/mv_mz/decryptor.rs`
- [x] `src-tauri/src/engines/detector.rs`
- [x] Tests unitaires Rust : extraction round-trip
- [x] `src-tauri/src/llm/tokenizer.rs` (anticipé depuis F2)
- [-] Intégration `rvpacker-txt-rs` (abandonné — implémentation custom)

## Prochaine session (F1 — Étape 2)

1. `src-tauri/migrations/0001_initial.sql` — tables `projects`, `source_files`, `segments`
2. `src-tauri/src/db/pool.rs` — init `SqlitePool`, run migrations au démarrage
3. `src-tauri/src/state.rs` — compléter `AppState { db: SqlitePool }`
4. `lib.rs` — `.manage(AppState { db })` + setup migrations
5. Commands Tauri : `open_project`, `get_source_files`, `get_segments`, `update_segment`, `export_project`
6. Layout React 3 colonnes (FileTree | SegmentGrid | SidePanel) avec `ResizablePanelGroup`
7. `src/components/editor/SegmentGrid.tsx` — TanStack Table + Virtual scroll
8. Zustand stores : `editor.ts`, `project.ts`

---
*Généré par Claude Code — Hoshi2Star*
