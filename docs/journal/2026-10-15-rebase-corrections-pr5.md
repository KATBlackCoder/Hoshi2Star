# Journal — 2026-10-15 — PR #5 : rebase sur main et corrections de revue

**Phase** : F4 / F5
**Durée estimée** : 4h
**Statut** : ✅ Complété
**Outil** : Claude (contribution ArcoFrio)
**Branche et commit de départ** : `feat/mv-mz-plugin-text` @ `e4e0a29`, rebasée sur `main` @ `b2089af`
**Objectif / plan / story** : demande de rebase de la PR #5 et constats de
`docs/journal/2026-10-15-revue-pr-arcofrio.md` (bloquant, importants, mineurs).

---

## Ce qui a été fait

- Rebase des 7 commits sur `main` @ `b2089af` (refonte UI, terminologie, migration 0009).
  Conflits résolus :
  - `core/qa.rs` (tests) : noms et `rich_rule` de main, plus le 5ᵉ argument `TokEngine`
    de `check_with_rules`.
  - `db/pool.rs` : tests sur `embedded_versions()`.
  - `tests/terminology_migration.rs` : versions `1..=10`.
  - `tasks/todo.md` : version de main gardée.
- Retrait de toutes les modifications de `tasks/`, `ROADMAP.md` et `tasks/lessons.md`
  des commits de la PR (constat mineur de la revue).
- Constat bloquant : `snapshot_root` refuse tout `project_id` qui n'est pas un UUID
  (36 caractères). `remove_snapshot` ne supprime alors rien. `open_project` ignore un id de
  manifeste malformé et génère un UUID.
- Snapshot jamais rafraîchi : chaque entrée garde `knownDisk`, les SHA-256 du disque à la
  capture et de chaque fichier exporté (zip ou injection d'un fichier), au plus 16.
  `ensure_fresh_mv_mz_snapshot` (export, preflight, couverture, injection) compare chaque
  fichier du jeu au snapshot. Un hash inconnu est reconstruit puis comparé (JSON parsé ; un
  `locale` égal à la cible d'export est annulé). S'il est identique, c'était un ancien export
  et son hash est retenu. Sinon le jeu a été mis à jour et le snapshot prend la nouvelle version.
- Catégories « verified » auto-activées : plus jamais. Après une extraction neuve, le scan
  propose une notification « Examiner » s'il existe une catégorie activable. Elles restent
  pré-cochées dans le dialogue, mais rien n'est appliqué sans clic.
- Snapshot complet à l'ouverture d'un projet restauré : plus de scan à l'ouverture. Nouvelle
  commande `get_text_coverage_status` (sans scan) pour proposer la notification si aucune
  décision n'existe.
- `%n` perdu : nouvelle erreur `MissingFormatSpecifier`, avertissement (−15) qui ne bloque
  pas l'export. Les codes d'échappement perdus restent `MissingPlaceholder`, critique.
- Mineurs :
  - limite de profondeur 128 dans le parseur de `json_span.rs` ;
  - `plugin_script_path` rejette `..`, `\`, `:` et les segments vides dans les noms de
    plugins (et ajoute `.js` au lieu de remplacer une extension) ;
  - `nuun_enemy_book` épinglé par SHA-256, et un test exige un hash pour chaque profil livré.

## Fichiers créés

- `docs/journal/2026-10-15-rebase-corrections-pr5.md`

## Fichiers modifiés

- `src-tauri/src/core/originals.rs` : `is_project_id`, `snapshot_root` faillible,
  `ensure_fresh_mv_mz_snapshot`, reconstruction factorisée (`Recorded`, `rebuild_original`,
  `update_snapshot`), `remember_written`, tests.
- `src-tauri/src/engines/mv_mz/originals.rs` : `known_disk`, `is_known_disk`,
  `remember_disk`, `revert_locale_switch`, `same_content`, tests.
- `src-tauri/src/commands/export.rs` : snapshot frais, hash des fichiers écrits retenus.
- `src-tauri/src/commands/coverage.rs` : `get_text_coverage_status`, `plugin_script_path`,
  scan sur snapshot frais.
- `src-tauri/src/commands/project.rs` : id de manifeste validé.
- `src-tauri/src/core/qa.rs`, `src-tauri/src/core/report.rs` : `MissingFormatSpecifier`.
- `src-tauri/src/engines/mv_mz/json_span.rs`, `src-tauri/src/engines/mv_mz/profiles.rs`,
  `src-tauri/profiles/mv_mz/nuun_enemy_book.toml`, `src-tauri/src/lib.rs`.
- `src-tauri/tests/sqlite_migration.rs` : id de projet UUID (l'export exige un UUID).
- `src/stores/coverage.ts`, `src/stores/project.ts`, `src/stores/project.test.ts`,
  `src/components/editor/QAPanel.tsx`, `src/lib/types.ts`, `src/locales/{en,fr}.json`.
- `docs/engines/mv-mz-optional-text.md`, `CHANGELOG.md`.

## Fichiers supprimés

- aucun

## Dépendances ajoutées

- aucune

## Décisions prises

- Avec l'accord de l'utilisateur (ArcoFrio) : opt-in partout, `%n` perdu en avertissement,
  et toutes les corrections dans cette PR.
- La migration `0010_text_categories.sql` n'est pas modifiée. Son commentaire sur 0009 reste,
  car changer le fichier changerait le checksum SQLx d'une migration déjà appliquée chez les
  testeurs.

## Problèmes rencontrés

- Conflits du rebase résolus par script : les fichiers ont des fins de ligne CRLF sur Windows,
  et un premier passage a laissé des marqueurs de conflit dans un commit. Corrigé par
  `commit --amend` pendant le rebase. Vérification : `git grep '^<<<<<<< '` vide.

## Corrections et erreurs à ne plus répéter

- Résoudre des conflits par script → regex insensible aux CRLF, puis rechercher les marqueurs
  avant `rebase --continue`.

## Vérifications et limites

- 2026-10-15, Windows 11. `cargo fmt --check`, `clippy --all-targets -D warnings` : OK.
  `pnpm typecheck` : OK. `pnpm test` : 112/112. `cargo test` : 629 OK, 2 en échec, déjà en
  échec avant le rebase sur cette machine et propres à Windows :
  `upgrades_pre_translation_secs_database` (checksum de migration changé par autocrlf) et
  `test_find_wolf_data_dir_lowercase_fallback` (système de fichiers insensible à la casse).
  La revue Linux les donne verts.
- Non exécuté : test manuel dans l'application, tests `#[ignore]` sur le vrai jeu après ces
  corrections, Linux.

## Tâches ROADMAP cochées

- [ ] aucune

## Prochaine session

- Nouvelle revue de la PR #5 par le mainteneur, puis la PR #6 (rebasée sur celle-ci, journal
  `2026-10-15-rebase-corrections-pr6.md`).

---
*Journal partagé Codex / Claude — Hoshi2Star*
