# Journal — 2026-10-28 — Implémentation UW : Wolf lu depuis le `Data/` déchiffré par UberWolf

**Phase** : F5
**Durée estimée** : 3h
**Statut** : 🔄 En cours (code et gate faits ; essai en jeu UW-9 à faire par l'utilisateur)
**Outil** : Claude (session cloud)
**Branche et commit de départ** : `feat/wolf-uberwolf` depuis `origin/docs/wolf-uberwolf-plan` @ `9189bc1`
**Objectif / plan / story** : `tasks/todo.md`, « Plan — UW », étapes UW-1 à UW-8 (décisions UW-D1 à UW-D8
validées telles que recommandées, UW-D5 reporté, UW-D8 hors périmètre, UW-R lot séparé). Plan :
`docs/journal/2026-10-28-plan-wolf-uberwolf.md`.

---

## Ce qui a été fait

- UW-1 (`9bcc1c0`) : détection Wolf par `Game.exe` / `GamePro.exe` / `Game.ini` + `Data/BasicData/` (ou
  `BasicData/`) contenant un `.project` ou `CommonEvent.dat` ; encodage lu dans la première base lisible
  (`SysDatabase.dat`, `DataBase.dat`, `CDataBase.dat`, toute casse) ; sans base, erreur `wolf_no_database`
  au lieu du repli v2 ; lecture de version par archive retirée.
- UW-2 + UW-3 (`0ebda33`, un seul commit) : `WolfSources` lit `Data/MapData` et `Data/BasicData` seuls et
  refuse `Data.wolf`, `Data/MapData.wolf`, `Data/BasicData.wolf` (`wolf_archive_present`) ; `Origin`,
  `display_path`, `archive_paths`, `wolf_unreadable_archive` retirés ; export en fichiers libres seulement
  (`deliver`, `by_archive`, `rebuild_archive`, `InjectorError::Archive`, `verify_archive_rebuild` retirés).
- UW-4 (`c337f5d`) : filtre des chemins de ressources ligne par ligne (`is_resource_path`, `is_lone_path`).
- UW-5 (`72bdafe`) : interface (marqueurs, messages fr/en, note d'export sans archives).
- UW-6 (`d8f396a`) : test e2e `wolf_game_without_database_is_refused_at_open`, texte `engineNotFound` précisé.
- UW-7 : ADR-008, `docs/engines/wolf-uberwolf.md`, `CONTEXT.md`, `ROADMAP.md`, `BACKLOG.md`, `CHANGELOG.md`.

## Fichiers créés

- `docs/adr/ADR-008.md`
- `docs/engines/wolf-uberwolf.md`
- `docs/journal/2026-10-28-uw-implementation.md`

## Fichiers modifiés

- `src-tauri/src/engines/detector.rs` — détection et encodage (UW-1), tests réécrits.
- `src-tauri/src/engines/wolf/sources.rs` — réécrit : `Data/` seul, refus des archives de textes.
- `src-tauri/src/engines/wolf/injector.rs` — livraison en fichiers libres seulement.
- `src-tauri/src/engines/wolf/integrity.rs` — contrôle d'archive reconstruite retiré.
- `src-tauri/src/engines/wolf/extractor.rs` — filtre des chemins ; test réel Honoka `(.wolf)` → refus.
- `src-tauri/src/commands/project.rs`, `src-tauri/src/commands/export.rs` — erreur d'encodage propagée ;
  `WolfExportPreflight.archives` retiré ; plus de zip « stored » pour les `.wolf`.
- `src-tauri/tests/e2e_project_flow.rs`, `src-tauri/tests/export_debug.rs` — fixture `BasicData` déchiffré,
  tests d'archive remplacés.
- `src/lib/projectError.ts` (+ test), `src/components/ExportWolfNotes.tsx` (+ test), `src/lib/types.ts`,
  `src/locales/fr.json`, `src/locales/en.json`.
- `CONTEXT.md`, `ROADMAP.md`, `BACKLOG.md`, `CHANGELOG.md`, `tasks/todo.md`, `tasks/handoff.md`.

## Fichiers supprimés

- Aucun (le module `engines/wolf/decrypt/` reste jusqu'au lot UW-R).

## Dépendances ajoutées

- Aucune.

## Décisions prises

- UW-2 et UW-3 dans un même commit : retirer `Origin::Archive` des sources sans retirer la livraison en
  archive laisserait une variante jamais construite (Clippy `-D warnings`).
- Marqueur supplémentaire `wolf_no_database` (`detector::WOLF_NO_DATABASE`) : UW-D4 demande « refuser avec un
  message » ; sans marqueur, l'interface n'affichait que « Impossible d'ouvrir le projet ».
- UW-4 : le plan exige « sans espace » et `test_is_resource_path` inchangé (qui contient `MS Gothic.ttf`).
  Retenu : espace ASCII admis seulement dans une ligne entièrement ASCII ; jamais d'espace pleine chasse.
  **À valider par l'utilisateur.**

## Problèmes rencontrés

- Gate UW-1 rouge une fois : `tests/export_debug.rs::wolf_debug_export_delivers_only_the_ready_translations`
  fabriquait un jeu Wolf sans `BasicData/` (non détecté par la nouvelle règle). Corrigé avec la même fixture
  que `e2e_project_flow.rs` ; ma recherche initiale ne couvrait que `e2e_project_flow.rs`.
- Un `git push` bloqué par le réseau ; relancé avec succès.

## Corrections et erreurs à ne plus répéter

- Changement de détection → chercher tous les tests qui fabriquent un jeu du moteur (`Game.ini`, `Game.exe`)
  dans **tous** les fichiers de `src-tauri/tests/`, pas seulement le fichier e2e principal. Règle ajoutée à
  `tasks/lessons.md` (complément de la leçon RW-5 du 2026-10-26).

## Vérifications et limites

- Gate cloud à chaque étape (fmt `--check`, `pnpm typecheck`, `pnpm test`, `pnpm lint`, clippy
  `--all-targets --locked -D warnings`, `cargo test --locked --no-fail-fast`) : vert de UW-1 à UW-6 ; chiffres
  par étape dans `tasks/todo.md`. Gate final : voir UW-8 dans `tasks/todo.md`.
- **Non exécuté** : tests réels `#[ignore]` (`H2S_WOLF_GAMES`, `H2S_WOLF_FIT_*`, `H2S_WOLF_ARCHIVES`) et tests à
  fixtures `test/` (absents du cloud, ils sortent sans rien vérifier) ; essai en jeu UW-9.

## Tâches ROADMAP cochées

- [x] Wolf lu depuis le `Data/` déchiffré par UberWolf (plan UW, ADR-008)

## Prochaine session

- Utilisateur, en local : UW-9 (とのかじ, ポンコツ, 3.658 déchiffré : ouvrir, traduire quelques segments,
  appliquer l'export sur une copie, vérifier en jeu ; Evtext du prologue de ポンコツ sur la copie marquée) ;
  tests réels `H2S_WOLF_GAMES`. Puis lot UW-R (retrait de `decrypt/`).
