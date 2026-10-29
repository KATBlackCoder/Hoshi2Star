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

## Suite (même session) — mesure dans l'app réelle

- Le banc cloud (branche `perf/bench-grid`) et le correctif G1 (branche `perf/grid-scroll`) ont été produits par une session cloud, puis relus en local.
- Mesure dans la vraie app de dev, pilotée par le pont MCP, sur 12 423 segments réels. Détail : `docs/benchmarks/2026-10-29-grid-app-reelle.md`.
  - G1 : +12 à +31 % seulement, mais des **zones blanches** (jusqu'à 12 frames sur 60, 553 px) que `main` n'a pas. À ne pas fusionner tel quel.
  - Grille à environ 10 images par seconde, que ce soit avec G1 ou sur `main`. Les variantes CSS (sans `text-wrap: pretty`, zones de saisie masquées, lignes invisibles) n'y changent rien. Un bloc de test sans React est aussi lent (78 ms par frame).
  - Sans `WEBKIT_DISABLE_DMABUF_RENDERER=1` (imposé par `pnpm tauri:linux`), la grille passe de 95 ms à **19 ms par frame**, et le bloc de test de 78 ms à 17 ms.
- Leçon ajoutée dans `tasks/lessons.md` (2026-10-29, mesurer d'abord dans l'app réelle).
- Rien n'a été écrit en base ; aucun code applicatif n'a été modifié. L'app de dev a été relancée sans la variable, sur `main`.
- Non vérifié : l'affichage réel à l'écran sans la variable (à confirmer par l'utilisateur) ; G1 avec le rendu accéléré.
- **Correctif du script de dev** (branche `fix/tauri-linux-wayland`) : l'utilisateur a confirmé que l'app lancée sans `WEBKIT_DISABLE_DMABUF_RENDERER=1` est plus rapide.
  - `tauri:linux` ne garde que `__NV_DISABLE_EXPLICIT_SYNC=1`, et le nouveau script `tauri:linux:x11` reprend l'ancien réglage ;
  - `CLAUDE.md` et `CONTEXT.md` sont mis à jour ;
  - vérifié : `package.json` lisible par Node, diff relu. Le script lui-même n'a pas été relancé : l'utilisateur a tapé la même commande à la main. Gate applicatif non exécuté (aucun code de l'app modifié).

---
*Journal partagé Codex / Claude — Hoshi2Star*
