# Journal — 2026-10-25 — Analyse complète, session 2 (parcours réels MV/MZ)

**Phase** : F5 (analyse avant release)
**Durée estimée** : 2 h 30
**Statut** : ✅ Complété (MV/MZ ; Wolf reporté)
**Outil** : Claude
**Branche et commit de départ** : `main` @ `6edd29b`. Modifications locales préexistantes conservées : `BACKLOG.md`, suppression de `docs/plans/2026-10-25-sessions-suivantes.md`.
**Objectif / plan / story** : session 2 de `docs/plans/2026-10-25-analyse-complete-app.md`, MV/MZ seulement (Wolf reporté après la session REA). Aucun code applicatif modifié.

---

## Ce qui a été fait

- Mise en place hors dépôt suivi :
  - copies reflink des 3 jeux et copie de `~/.local/share/com.blackat.hoshi2star/` dans `test/_session2/` ;
  - app lancée avec `XDG_DATA_HOME` pointant sur la copie ;
  - preuves : `/proc/<pid>/fd` et sha256 de la base originale inchangé.
- Parcours sur MV (性処理係のある学校, neuf), MZ StandGirl (ancien) et MZ 借金JK (neuf) : ouverture et extraction, export sans traduction (zip vide), aller-retour complet par cibles marqueurs, échantillon traduit avec gemma4:e4b, QA, corrections manuelles des erreurs critiques, export et contrôle par pointeur.
- Comparaison gemma4:e4b / gemma4:e2b sur l'échantillon MV (S-L1, schéma JSON).
- Reproductions dans l'app :
  - S-X1 sur MV et MZ ;
  - S-U1, S-U2 et S-U3 ;
  - S-C4 (manifeste copié).
- S-L2 : « Tout traduire » lancé, mais Ollama n'a pas été arrêté (utilisateur absent). Le run complet sert de mesure : 46 `context_leak` critiques sur 1 191 segments. Projet jetable remis à zéro.

## Fichiers créés

- `docs/audit/2026-10-analyse-complete.session2.preuves.txt` — preuves brutes.
- `docs/journal/2026-10-25-analyse-session-2.md` — ce journal.
- Hors Git : `test/_session2/` (jeux copiés, base copiée, `evidence/`, `tools/`, `zips/`).

## Fichiers modifiés

- `docs/audit/2026-10-analyse-complete.md` — section « Parcours réels » (P-1 à P-9), cases « À confirmer » mises à jour, tableau des sessions.
- `docs/plans/2026-10-25-analyse-complete-app.md` — session 2 cochée (MV/MZ, Wolf reporté).
- `tasks/todo.md` — correctif demandé : une seule notification en cas de panne du fournisseur.
- `tasks/handoff.md` — reprise.
- `tasks/lessons.md` — 2 règles (copies de jeux et Vite, copie d'un projet ancien).

## Mise à jour du même jour : S-L2 (15:23–15:31 UTC, utilisateur présent)

- Ollama arrêté pendant « Tout traduire » : 1 119/1 191 segments passent en `needs_review`, cible = source, en 10 s.
- `llm/completed` est émis, aucune erreur.
- Un toast par segment ; le processus WebKit monte à 8,7 Go et se tue lui-même (`app.log`).
- 0 segment repris par « Tout traduire ».
- Rapport P-6, preuves `sl2-05`.

## Réduction des extraits (demande utilisateur, même jour)

- `test/` ne doit jamais aller dans Git. Vérifié : `.gitignore:29`, aucun fichier suivi, rien dans l'historique.
- Fichier de preuves réécrit en version réduite (99 lignes) : comptes, pointeurs, erreurs, métriques. Plus d'échantillons source/cible.
- Rapport :
  - la liste détaillée des vérifications en jeu est déplacée dans `test/_session2/A-VERIFIER-EN-JEU.md` ;
  - les citations de dialogue sont remplacées par des pointeurs JSON.

## Fichiers supprimés

- Aucun.

## Dépendances ajoutées

- Aucune.

## Décisions prises

- Pour l'aller-retour, la méthode « cible = source » a été remplacée par des marqueurs déterministes : la QA bloque `UnchangedSource`. La preuve obtenue est plus forte, puisqu'on contrôle chaque pointeur.
- Exports partiels faits par la commande `export_project` : l'interface exige 100 % de segments traduits.
- Le zip de 借金JK reste en français (projet créé en `fr`) : c'est un essai utile des accents.

## Problèmes rencontrés

- `webview_execute_js` coupe vers 30 s, quelle que soit l'option `timeout`. Contournement : promesse stockée sur `window`, puis lecture séparée.
- Les copies de jeux dans le dépôt ont déclenché un rechargement Vite de la webview (voir leçon).

## Corrections et erreurs à ne plus répéter

- Copies de jeux sous le dépôt pendant `tauri dev` : rechargement Vite. Le lien avec la langue `fr` du projet suivant est supposé, non prouvé. Règle dans `tasks/lessons.md`.
- Copie d'un projet ancien : `game_path` original conservé (S-C4). Chemins réécrits dans la base copiée avant tout export. Règle dans `tasks/lessons.md`.

## Vérifications et limites

- Comptes (vérifiés par script) :
  - MV : 15 498/15 498 pointeurs justes, puis 38 = 38 = 38 ;
  - StandGirl : 1 191/1 191, puis 31 = 31 = 31 ;
  - 借金JK : 19 663/19 663, puis 34 = 34 = 34 ;
  - 0 valeur modifiée hors segments.
- Non exécutés :
  - tests en jeu des zips (utilisateur) ;
  - Wolf ;
  - édition de `plugins.js` (aucune catégorie) ;
  - S-U3 au défilement et à la fermeture.
- Aucun test applicatif (`pnpm test`, `cargo test`) : aucune modification de code.

## Tâches ROADMAP cochées

- [ ] Aucune.

## Prochaine session

- L'utilisateur teste les 4 zips en jeu (liste dans le rapport) et consigne les résultats dans `tasks/handoff.md`.
- Décision sur les extraits de jeux pour adultes avant commit.
- Session 3 du plan, qui intègre le correctif « une seule notification » (`tasks/todo.md`).

---
*Journal partagé Codex / Claude — Hoshi2Star*
