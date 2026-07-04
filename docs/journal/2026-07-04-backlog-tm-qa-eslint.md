# Journal — 2026-07-04 — Backlog trio : TM doublons · QA live glossaire · ESLint 10

**Phase** : Post-v0.4.5 (backlog audit)
**Durée estimée** : 2h
**Statut** : ✅ Complété

---

## Ce qui a été fait

- **FIX 1 — Doublons TM** : `idx_tm_hash_lang` était non-UNIQUE et `tm::insert` générait
  un UUID neuf à chaque appel → son `INSERT OR REPLACE` ne conflictait jamais, chaque
  sauvegarde de segment créait une ligne. Migration `0005` (dédup en gardant
  `MAX(rowid)` par `(source_hash, lang_pair)`, puis index UNIQUE) + upsert
  `ON CONFLICT DO UPDATE` (id de ligne préservé). 4 tests de régression.
- **FIX 3 — QA live sans glossaire ni engine** : `qa_check_segment` passait `&[]` au
  moteur QA (GlossaryMismatch −15 jamais détecté en frappe) et défaultait sur `mv_mz`
  même en projet Wolf (seuil 720 px au lieu de 520 px). Commande désormais async +
  `State` + `project_id: Option<String>` : glossaire via `glossary::relevant_terms`
  (même helper que le pipeline batch), engine résolu depuis `projects.engine`
  (précédence param > DB > `mv_mz`). QAPanel envoie `projectId` + queryKey étendue.
  4 tests via le helper testable `check_segment_live`.
- **FIX 2 — ESLint 10** : aucune config n'existait (ni `.eslintrc.*` ni flat) et le
  script utilisait `--ext` (supprimé) → ESLint 10 refusait de démarrer. Nouveau
  `eslint.config.js` flat + script `eslint src`. Triage : 3 fixes mécaniques
  (`no-useless-escape` constants.ts, `no-useless-assignment` updater.ts,
  eslint-disable périmé `react-compiler` columns.tsx), 2 règles downgradées en `warn`
  (`set-state-in-effect`, `only-export-components`) — refactors loggés dans todo.md.
- Gate complet vert : typecheck + 34 Vitest + clippy `-D warnings` + **375 tests lib
  (+8) + 3 intégration**.

## Fichiers créés

- src-tauri/migrations/0005_tm_unique.sql
- eslint.config.js
- docs/journal/2026-07-04-backlog-tm-qa-eslint.md

## Fichiers modifiés

- src-tauri/src/core/tm.rs — `insert` en upsert `ON CONFLICT`, doc comment, 4 tests
- src-tauri/src/commands/qa.rs — `qa_check_segment` async + State + project_id,
  helper `check_segment_live`, module tests (4)
- src/components/editor/QAPanel.tsx — `projectId` dans l'invoke + queryKey
- src/lib/constants.ts — échappement inutile `\-` dans PH_RE_SOURCE
- src/stores/updater.ts — init `null` inutile supprimée (no-useless-assignment)
- src/features/editor/columns.tsx — eslint-disable `react-compiler` périmé retiré
- package.json — script lint + 3 devDeps ; pnpm-lock.yaml
- CHANGELOG.md, tasks/todo.md

## Dépendances ajoutées

- pnpm add -D typescript-eslint@8.62.1 @eslint/js@10.0.1 globals@17.7.0

## Décisions prises

- Upsert `ON CONFLICT DO UPDATE` plutôt que garder `INSERT OR REPLACE` : REPLACE =
  delete+reinsert → l'id changerait à chaque sauvegarde ; DO UPDATE préserve
  l'identité de ligne (FK futures).
- Unicité TM scopée `(source_hash, lang_pair)` — pas l'engine (TM globale, ADR-003).
- Engine du QA live résolu depuis la DB (source de vérité, leçon 2026-06-11), le
  front n'envoie pas d'engine ; param explicite garde la précédence.
- `src/components/ui/` (shadcn) exclu du lint : interdits d'édition manuelle → les
  linter = bruit permanent.
- Chemin dédup de la migration 0005 non testé automatiquement : les DB de test
  naissent post-0005 ; vérif manuelle sur la DB dev.

## Problèmes rencontrés

- `TmSuggestion` expose `entry.target_text` (pas `target_text` direct) — corrigé au
  premier run de tests.
- `reactHooks.configs['recommended-latest']` est au format legacy dans react-hooks
  v7.1 → utiliser `configs.flat['recommended-latest']`.
- Ligne de test « 49 unités » comptée 50 de tête — l'assert `len()` a attrapé
  l'erreur, seuils wolf 40 < 49 < 55 mv_mz toujours valides.

## Tâches ROADMAP cochées

- (aucune — items backlog hors phases ROADMAP)

## Prochaine session

- Vérif manuelle FIX 1 sur la DB dev (lancer l'app, requête doublons → 0 ligne)
- Merge `fix/backlog-tm-qa-lint` → `main`
- Reste au backlog : Ph9 (perf, sur signal), refactors `set-state-in-effect`,
  invalidation cache `["qa-check"]` sur édition glossaire, lints clippy
  `--all-targets` du code de test

---
*Généré par Claude Code — Hoshi2Star*
