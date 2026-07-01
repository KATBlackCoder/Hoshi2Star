# Journal — 2026-07-01 — Phase 3 remédiation : robustesse des promesses UI

**Phase** : F4
**Durée estimée** : 1h30
**Statut** : ✅ Complété

---

## Ce qui a été fait

Phase 3 du plan de remédiation issu de l'audit du 2026-07-01
(`docs/audit-remediation-plan-2026-07-01.md`) — 🟠 P1 erreurs silencieuses côté UI.
Décision de scope validée : **chargement par pages successives** pour le plafond
5000 (le backend renvoyait déjà le vrai `total`, ignoré par l'UI).

- **`ProjectList`** : `Promise.allSettled` — un `get_project_stats` en échec ne
  fait plus disparaître les stats de toutes les cartes.
- **`SegmentGrid.handleSave`** : try/catch + `toast.error`
  (clé i18n `segmentGrid.saveError` en/fr) — un échec de sauvegarde n'est plus
  un rejet silencieux.
- **Sélection** : `getRowId: (row) => row.id` — `rowSelection` keyée par id de
  segment au lieu de l'index dans `filteredSegments` → traduire les mauvais
  segments après filtrage devient structurellement impossible ; en complément,
  reset de la sélection sur changement de `qaFilter`/`searchQuery` (sélections
  invisibles).
- **Plafond 5000** : `loadSegments` boucle par pages de 2000 jusqu'à `total`,
  avec garde d'annulation `loadSeqRef` (un chargement périmé — fichier changé
  entre-temps — n'écrase ni `segments` ni `isLoading`). Footer et compteurs
  portent désormais sur le fichier entier.
- **`QAPanel`/`TMPanel`** : `await save(...)` (plugin dialog) gardé par try/catch.
- Doc-comment `get_segments` aligné (l'ancien « between 1 and 500 » était faux).

## Vérification

Gate ✅ (typecheck · clippy `-D warnings` · 363 tests Rust). Aucun test front
n'existant (Vitest = Phase 7), **vérification manuelle en live** : app dev lancée
(`pnpm tauri:linux`) et pilotée via le MCP Tauri sur le projet réel MV
(性処理係のある学校, 15 498 segments) :

- Stats des 2 cartes ProjectList affichées (allSettled).
- `Map119.json` (561 segments) chargé entier — footer « 561 segments »,
  recherche filtrée « 6 / 561 ».
- Boucle multi-pages prouvée sur le protocole réel : 6 pages × 100 → 561 ids
  uniques, ni doublon ni trou.
- 2 lignes cochées → bouton « Traduire 2 lignes » ; saisie d'une recherche →
  bouton disparu, 0 case cochée.
- Édition « Sandwich » → « Sandwich. » persistée (updatedAt bump), puis
  restaurée à l'identique.

## Fichiers créés

- `docs/journal/2026-07-01-ui-promise-robustness.md` — cette entrée

## Fichiers modifiés

- `src/components/editor/SegmentGrid.tsx` — pagination successive + garde,
  try/catch handleSave, getRowId, reset sélection
- `src/components/editor/ProjectList.tsx` — allSettled
- `src/components/editor/QAPanel.tsx`, `TMPanel.tsx` — garde `save()`
- `src/locales/en.json`, `fr.json` — clé `segmentGrid.saveError`
- `src-tauri/src/commands/project.rs` — doc-comment `get_segments`
- `CHANGELOG.md`, `tasks/todo.md`

## Décisions prises

- **Pages successives plutôt que pagination UI serveur** : la grille est
  virtualisée (mémoire OK) et les filtres/recherche/compteurs côté client
  restent corrects sur le fichier entier ; la pagination serveur aurait cassé
  la recherche pleine-file (UX CAT dégradée).
- **`getRowId` + reset** : deux couches — correctness (id stable) + UX (pas de
  sélection invisible).

## Problèmes rencontrés / découvertes

- 🐛 **Découverte hors scope (loggée au backlog)** : `tm.rs::insert` utilise
  `INSERT OR REPLACE` mais le PK est un UUID neuf et `idx_tm_hash_lang` n'est
  **pas UNIQUE** → le REPLACE ne remplace jamais, la TM accumule un doublon à
  chaque re-sauvegarde du même segment (observé en live pendant la vérif ;
  l'entrée de test a été nettoyée). Fix futur : index UNIQUE + dédup + upsert.
- 🐛 **`pnpm lint` cassé (pré-existant)** : ESLint 10 exige la flat config
  (`eslint.config.js`), le repo a `.eslintrc.*`. Vérifié identique sur l'arbre
  propre. Backlog.

## Tâches ROADMAP cochées

- [ ] (aucune — remédiation audit hors backlog planifié)

## Prochaine session

- Phase 4 — 🟠 Correctness du moteur QA : `is_fullwidth` (katakana demi-largeur
  U+FF61..FF9F = largeur 1), dénominateur du rapport, check glossaire mort.

---
*Généré par Claude Code — Hoshi2Star*
