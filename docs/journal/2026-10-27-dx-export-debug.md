# Journal — 2026-10-27 — DX : export de debug pour tester en jeu

**Phase** : F4
**Durée estimée** : 3h
**Statut** : 🔄 En cours (code et tests écrits ; ni commit, ni gate cloud)
**Outil** : Claude
**Branche et commit de départ** : `feat/dx-debug-export`, depuis `main` @ `2d9ecd7` (RW fusionné, PR #13 ; suivi PR #14)
**Objectif / plan / story** : `tasks/todo.md`, « Plan — DX », validé le 2026-10-27 (DX-1 à DX-3 = défauts)

---

## Ce qui a été fait

- Plan EX (export d'essai pour l'utilisateur final) conçu puis validé, puis **abandonné** par l'utilisateur :
  l'export normal reste réservé aux projets entièrement traduits (règle produit). Leçon dans `tasks/lessons.md`.
- Plan DX écrit et validé : même moteur qu'EX, mais placé dans les outils de diagnostic, avec une commande séparée.
- Rust (`commands/export.rs`) :
  - `export_project` est factorisé en `write_export(state, project_id, ExportOptions)` ;
    `export_project` garde sa signature, son audit et son refus des erreurs critiques ;
  - `ExportScope::{Release, Debug}`, avec un seul fragment SQL de sélection des cibles par portée. Il est appliqué aux :
    - lignes MV/MZ ;
    - `plugin_rows` ;
    - `linked_skill_type_renames` ;
    - `fitted_lines` (une cible écartée est vue comme vide par la mise en page) ;
    - `collect_wolf_zip_entries` (argument `_project_id` inutilisé retiré, pour rester sous la limite Clippy `too_many_arguments`) ;
  - nouvelle commande `export_debug_project(project_id) -> DebugExport { zip_path, exported, untranslated, skipped_critical }` :
    - audit QA puis comptes ;
    - refus si rien n'est exportable ;
    - `hoshi2star-debug.zip` avec un commentaire de zip « DEBUG, not for distribution » ;
    - sans police ni langue ;
    - `remember_written` comme à l'export normal ;
  - `write_zip` accepte un commentaire facultatif.
  - `debug_inject_file` est inchangée (portée `Release`), comme décidé en DX-3.
- Frontend :
  - bouton « Export de debug » dans `AppToolbar.tsx`, si `developerTools` est coché et que le moteur est `mv_mz` ou `wolf` ;
  - boîte de confirmation dans `AppDialogs.tsx` (non-traduits laissés d'origine, copie du jeu, pas de distribution, note Wolf) ;
  - `handleDebugExportConfirm` dans `useAppHandlers.ts`, avec un `reportExportError` commun à l'export normal :
    une erreur d'intégrité ouvre `ExportIntegrityDialog` ;
  - type `DebugExport`, clés i18n `debugExport.*` et `settings.developer.debugExportExplanation` (fr/en).

## Fichiers créés

- `src-tauri/tests/export_debug.rs` (5 tests : MV mixte, export normal toujours refusé, refus sans traduction, garde R2, Wolf mixte)
- `docs/journal/2026-10-27-dx-export-debug.md`

## Fichiers modifiés

- `src-tauri/src/commands/export.rs`, `src-tauri/src/lib.rs` — portée d'export, commande `export_debug_project`
- `src/components/AppToolbar.tsx`, `src/components/AppDialogs.tsx`, `src/App.tsx`, `src/hooks/useAppHandlers.ts`,
  `src/components/settings/DeveloperSettings.tsx`, `src/lib/types.ts`, `src/locales/fr.json`, `src/locales/en.json`
- `src/components/AppToolbar.test.tsx`, `src/hooks/useAppHandlers.test.ts` — tests Vitest
- `src-tauri/tests/text_fit.rs` — test `debug_export_does_not_lay_out_a_message_with_a_left_out_line` (branche « cible écartée » de `fitted_lines`, sur la fixture `mz_fit`)
- `CHANGELOG.md`, `tasks/todo.md`, `tasks/handoff.md`, `tasks/lessons.md`

## Fichiers supprimés

- aucun

## Dépendances ajoutées

- aucune

## Décisions prises

- DX-1 : une traduction en erreur QA critique est écartée. Après l'audit, une cible non vide est `needs_review`
  si et seulement si elle a une erreur critique (`report.rs`, `next_status`) : d'où le filtre unique
  `target_text != '' AND status != 'needs_review'`.
- DX-2 : aucune option (police, langue).
- DX-3 : `debug_inject_file`, `export_debug_json` et les clés `debugInject*` restent en place (retrait éventuel : suivi séparé).
- Effet de bord accepté : l'audit écrit les statuts et scores QA, comme avant un export normal.

## Problèmes rencontrés

- `prettier --write` a reformaté une ligne de `src/App.tsx` hors périmètre (écart déjà présent sur `main`) :
  fichier remis d'origine, seule la ligne `onDebugExport` est gardée.

## Corrections et erreurs à ne plus répéter

- Plan EX abandonné : un besoin de test traité comme une fonction produit, en assouplissant la porte d'export.
  Règle ajoutée dans `tasks/lessons.md` (2026-10-27).

## Vérifications et limites

- Exécuté localement (léger, sans compilation) : rustfmt sur `export.rs`, `lib.rs`, `export_debug.rs` et `text_fit.rs` : OK ;
  `prettier` sur les fichiers frontend touchés.
- **Non exécutés** (le poste gèle) : `cargo build`/`clippy`/`test`, `pnpm typecheck`, `pnpm test`, lint.
  À faire dans le gate cloud après le push.
- Non vérifié : affichage réel du bouton et de la boîte dans l'app (impossible sans build Rust local), essai en jeu.
- Limites connues :
  - Wolf n'a pas de copie d'origine : décompressé sur le vrai jeu, un zip de debug devient la base des exports suivants.
    La boîte le dit.
  - Un message MZ partiellement exporté n'est pas mis en page.

## Tâches ROADMAP cochées

- [ ] aucune

## Prochaine session

- Accord de l'utilisateur pour commit et push de `feat/dx-debug-export`. Le hook pre-commit lance clippy :
  demander avant tout `--no-verify`.
- Puis gate cloud complet. Ensuite PR, et essai en jeu avec accord.

---
*Journal partagé Codex / Claude — Hoshi2Star*
