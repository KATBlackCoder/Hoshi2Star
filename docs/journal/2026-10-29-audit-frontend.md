# Journal — 2026-10-29 — Audit du front-end (défilement, re-rendus, doublons)

**Phase** : F5
**Durée estimée** : 1h
**Statut** : ✅ Complété (analyse) — implémentation non commencée
**Outil** : Claude (session locale)
**Branche et commit de départ** : `main` @ `8e32c99`, avancée pendant la session à `b999c9b` par des pulls extérieurs. Aucun fichier de `src/` n'a changé entre les deux.
**Objectif / plan / story** : demande de l'utilisateur. Analyser le fonctionnement du front-end, ses doublons et ses lenteurs, en particulier le défilement lent des textes après l'extraction d'un jeu. Analyse seulement : l'implémentation attend la validation du plan.

---

## Ce qui a été fait

- Lecture directe de `SegmentGrid.tsx` et `features/editor/columns.tsx`, et du comportement de `@tanstack/react-virtual` 3.13.25 / `virtual-core` 3.15.0 dans `node_modules` : `useFlushSync` vaut `true` par défaut, et le défilement appelle `notify(isScrolling)` puis `flushSync`.
- Trois sous-agents en lecture seule : architecture et flux de données ; doublons et code mort ; lenteurs hors grille. Leurs constats principaux ont été revérifiés dans le code.
- Volumétrie relevée en lecture seule dans la base de l'app : 12 423 segments dans le plus gros fichier, 2,4 lignes par segment en moyenne, 0,77 Mo de texte.
- Rapport : `docs/audit/2026-10-29-audit-frontend.md`.

## Fichiers créés

- `docs/audit/2026-10-29-audit-frontend.md`
- `docs/journal/2026-10-29-audit-frontend.md`

## Fichiers modifiés

- `tasks/handoff.md` : ajout de la section « Audit (2026-10-29) ».

## Fichiers supprimés

- Aucun.

## Dépendances ajoutées

- Aucune.

## Décisions prises

- Aucune décision d'implémentation. Un plan par lots P1 à P6 est proposé (§11 du rapport) ; il est à valider par l'utilisateur.

## Problèmes rencontrés

- La section non commitée « Étude UberWolfCli sous Linux » de `tasks/handoff.md` a disparu pendant la session. Elle n'est ni dans le fichier ni dans un stash. Cause non établie : `main` a été avancée par `git pull --ff-only` pendant la session (reflog). Une copie a été sauvegardée hors dépôt depuis la sortie relevée en début de session, pour une restauration éventuelle avec l'accord de l'utilisateur.

## Corrections et erreurs à ne plus répéter

- Aucune correction de code.

## Vérifications et limites

- Analyse statique uniquement. Aucune mesure à l'exécution (profil React, timeline WebKit) : le poids réel de chaque cause reste à mesurer, selon le protocole du §3.4 du rapport.
- `pnpm typecheck`, `pnpm test`, Clippy et `cargo test` n'ont pas été exécutés : aucune modification de code.

## Tâches ROADMAP cochées

- Aucune.

## Prochaine session

- L'utilisateur valide (ou ajuste) les lots P1 à P6. P1 commence par une mesure en build release sur トロイメライ `CommonEvent.dat`, puis applique les étapes du §3.3 une par une, chacune mesurée.

---
*Journal partagé Codex / Claude — Hoshi2Star*
