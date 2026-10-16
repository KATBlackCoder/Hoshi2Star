# Point de reprise — Hoshi2Star

**Mis à jour :** 2026-10-16 — Claude.
**Objectif utilisateur :** intégrer les PR externes #5/#6 d'ArcoFrio en sécurité.
**Statut :** terminé. #7, #8, puis #5 (texte plugins/système) et #6 (text fitting MZ)
fusionnées dans `main` (`3177567`). Migrations 0009, 0010, 0011 séquentielles.

## Dernier point atteint

- Journal de revue : `docs/journal/2026-10-15-revue-pr-arcofrio.md`. Corrections de l'auteur :
  `docs/journal/2026-10-15-rebase-corrections-pr5.md`, `docs/journal/2026-10-15-rebase-corrections-pr6.md`.
- Tous les constats de la revue ont été traités par ArcoFrio dans `a4e7f41` (#5) et `996adcc` (#6),
  y compris les petits points (limite WOFF, profondeur JSON, `..` dans les noms de plugins, masquage QA).
- Faille `project_id` vérifiée corrigée : `core/originals.rs` `is_project_id` (36 caractères + UUID).
- Sauvegarde base avant fusion : `~/.local/share/com.blackat.hoshi2star/backups/hoshi2star-before-pr-merge-2026-10-15-1234.db`.

## Vérifications

- 2026-10-16, tête de #6 (contient #5), Linux : typecheck, lint OK ; vitest 121/121 ;
  fmt, clippy `-D warnings` OK ; cargo test 691 passés, 0 échec.
- Non exécutés : tests sur d'autres jeux MV/MZ (promis à ArcoFrio), Windows, lancement de
  l'app après fusion de #5/#6 (migrations 0010/0011 sur la base locale), relecture
  détaillée de la logique de fraîcheur des snapshots (seulement survolée).

## Prochaine action exacte

1. Lancer l'app (`pnpm tauri:linux`) : vérifier que 0010/0011 s'appliquent sur la base locale.
2. Tester l'export sur 2–3 jeux MV/MZ (dont un projet existant) avant la prochaine release.
3. Ajouter ArcoFrio au CHANGELOG (promis) et attendre son avis sur les modèles 8/12/16 Go.
