# Journal — 2026-06-13 — Thème Tenmon 天文 phase 2 + fix regex placeholder Wolf

**Phase** : F3 (Polissage UI)
**Durée estimée** : ~3h
**Statut** : ✅ Complété

---

## Ce qui a été fait

- Validation manuelle du thème Tenmon (phase 1, déjà implémenté) en conditions
  réelles via `pnpm tauri dev` + tauri-mcp-server, sur le projet 月咲流ホノカ
  (Wolf, CommonEvent.dat 2195 segments / DataBase.dat 349 segments).
- Scan exhaustif du DOM (via `webview_execute_js`) : 0/2544 segments ne
  matchaient `PH_RE_SOURCE` — confirmation d'un gap déjà loggé (codes Wolf
  minuscules `\r[Base,Ruby]`, `\E`, `\c[2]`... jamais surlignés en chip cyan).
- **Fix regex placeholder par moteur** : ajout de `PH_RE_WOLF` (miroir JS de
  `RE_WOLF` côté Rust, `src-tauri/src/engines/wolf/placeholders.rs`) +
  `getPlaceholderRegex(engine)` dans `lib/constants.ts`, branché dans
  `SourceCell` via `useActiveProject().engine`. Validé sur CommonEvent.dat
  segment 5 (`\r[甘,あま]\r[酸,ず]` → chips cyan).
- **Phase 2 visuelle Tenmon** (items client-side de `demo-1-tenmon.html`) :
  - `QAScoreRing` dans `QAPanel` — anneau SVG doré (r=22) remplaçant le badge
    plat `ScoreBadge`, coloré selon le score (or=100, jaune≥75, rouge sinon).
  - Footer `SegmentGrid` étendu avec un récap par statut (pastilles + compteurs,
    `STATUS_STYLES` exporté de `columns.tsx`), dérivé en `useMemo` depuis
    `segments` (pas d'appel backend).
  - `ConstellationProgress` dans `AppToolbar` — barre de progression remplacée
    par une piste gradient violet→or avec nœuds-losanges fixes (8/26/45/78/94%)
    et comète ★ pulsante à la position courante.
  - Anneaux de progression par fichier (FileTree) **différés** — nécessitent
    une extension backend de `get_source_files` (compteurs traduit/total par
    fichier) ; tâche dormante ajoutée dans `tasks/todo.md` avec déclencheur
    explicite.
- Gate complet : `pnpm typecheck` ✅, `cargo clippy -D warnings` ✅,
  `cargo test` (304 passed / 0 failed / 4 ignored) ✅.
- Mise à jour `CHANGELOG.md` (entrées Tenmon phase 1+2 + fix regex Wolf),
  `tasks/todo.md` (item regex coché, section phase 2 + tâche dormante
  anneaux FileTree), `ROADMAP.md` (F3 Polissage UI), mémoire projet
  (`project_design_tenmon.md`).
- 2 commits séparés sur `main` (pas de push) :
  - `a8642e2` — thème Tenmon phase 1 + 2 (UI/design)
  - `34d9ac8` — fix regex placeholder Wolf (`SourceCell`)
  - `888b829` — doc ROADMAP

## Fichiers créés

- (aucun fichier source — démos `docs/design/` et screenshots créés lors de
  la session précédente, phase 1)

## Fichiers modifiés

- `src/lib/constants.ts` — ajout `PH_RE_WOLF`, `clonePH_RE_WOLF()`,
  `getPlaceholderRegex(engine)`
- `src/features/editor/columns.tsx` — `SourceCell` utilise
  `getPlaceholderRegex(activeProject?.engine)` via `useActiveProject()`
- `src/components/editor/QAPanel.tsx` — `QAScoreRing` (anneau SVG) remplace
  `ScoreBadge`
- `src/components/editor/SegmentGrid.tsx` — footer étendu avec récap de
  statuts (`statusCounts` via `useMemo`)
- `src/components/AppToolbar.tsx` — `ConstellationProgress` remplace la barre
  de progression simple
- `CHANGELOG.md`, `tasks/todo.md`, `ROADMAP.md` — entrées Tenmon phase 1+2 +
  fix regex Wolf

## Fichiers supprimés

- (aucun)

## Dépendances ajoutées

- (aucune — `@fontsource-variable/noto-sans-jp` ajouté lors de la session
  phase 1)

## Décisions prises

- Regex placeholder sélectionnée par moteur (`getPlaceholderRegex`), miroir
  du pattern `Engine::from_project_engine()` côté Rust (`llm/tokenizer.rs`).
- Anneaux de progression par fichier (FileTree) différés à une session future
  déclenchée par une extension de `get_source_files` (COUNT par statut groupé
  par `source_file_id`).
- Commits regroupés en 2 (et non 3 comme prévu initialement) : phase 1+2
  Tenmon dans un seul commit design (`a8642e2`), fix regex Wolf isolé
  (`34d9ac8`) — la séparation phase1/phase2 aurait nécessité un découpage
  fragile par hunks dans 3 fichiers partagés (`columns.tsx`, `AppToolbar.tsx`,
  `SegmentGrid.tsx`).

## Problèmes rencontrés

- `TaskCreate` mal utilisé en début de session (mauvais schéma) — abandonné,
  suivi via le fichier de plan approuvé à la place.
- `pnpm lint` reste cassé (ESLint 10 sans `eslint.config.js`) — problème
  préexistant, déjà loggé comme tâche dormante, non corrigé (hors scope).

## Tâches ROADMAP cochées

- [x] F3 Polissage UI — Thème Tenmon 天文 (phase 1 + 2), fix regex placeholder
      Wolf dans `SourceCell`

## Prochaine session

- Si déclenché : étendre `get_source_files` pour exposer des compteurs
  traduit/total par fichier, puis ajouter les anneaux de progression dans
  `FileTree.tsx` (miroir `demo-1-tenmon.html:165-173`).
- Sinon : reprendre F5 (Wolf RPG v3.x) — voir `project_wolf_v3_status.md`.

---
*Généré par Claude Code — Hoshi2Star*
