# En attente (tests manuels utilisateur)

- [ ] Test manuel "Tout traduire" sur un projet multi-fichiers : vérifier que
  le % ne touche 100% qu'à la toute fin (`h2s://llm/completed`), pas après
  chaque fichier. (commit `bf48f02`)
- [ ] Test manuel "Tout traduire" sur CommonEvent.dat (2000+ segments) :
  vérifier que les lignes passent à "Traduit" batch par batch en temps réel.
  (commit `bf48f02`)

# Tâches dormantes (ne pas démarrer sans déclencheur)

- [ ] **Restructuration wolf Phase 2+3** : renommer `v3_format/` → `format_v3/`
  et extraire la glue `wolfrpg_map_parser` (branches v2 inline
  d'`extractor.rs`/`injector.rs`) vers `format_v2/`.
  **Déclencheur** : un bug v2, une feature v2, ou l'extension du support v1.
  Décision 2026-06-12 : différé — code v2 vert (tests réels Honoka), refacto
  purement cosmétique sans travail v2 planifié. Phase 1 (`decrypt/`) faite.
- [ ] **ESLint 10 sans `eslint.config.js`** : `pnpm lint` cassé (préexistant,
  migration flat config à faire).
- [ ] **Anneaux de progression par fichier (FileTree)** : item phase 2 de
  demo-1-tenmon.html non implémenté — nécessite que `get_source_files`
  (src-tauri/src/commands/project.rs) renvoie des compteurs traduit/total
  par fichier (actuellement seulement `translationSecs`).
  **Déclencheur** : si on veut compléter la phase 2 visuelle Tenmon, étendre
  la requête SQL de `get_source_files` avec un COUNT par statut groupé par
  `source_file_id`, puis ajouter l'anneau SVG (miroir
  demo-1-tenmon.html:165-173) dans FileTree.tsx.

# Hors scope notés

- Edge case % > 100% si des segments changent de statut entre le COUNT initial
  et les SELECT par-fichier — risque jugé négligeable (desktop mono-utilisateur).

# Design UI — exploration (2026-06-13)

- [x] Analyser l'UI actuelle (thème, toolbar, grid, panels, screenshots)
- [x] Démos HTML 3 directions dans `docs/design/demo-{1-tenmon,2-washi,3-yoru}.html`
      (+ aperçus PNG dans `docs/screenshots/design-demo-*.png`)
- [x] Choix utilisateur : **Tenmon 天文** (observatoire nocturne — indigo/violet/or)

# Design UI — implémentation Tenmon (2026-06-13)

Scope phase 1 : thème + composants visibles. Pas de refonte structurelle du layout.

- [x] `pnpm add @fontsource-variable/noto-sans-jp` (rendu CJK correct)
- [x] `src/index.css` : palette Tenmon en `.dark` (fond indigo profond, primary
      violet, nouveau token `--star` or) + variante claire assortie (primary
      violet, accents or) + starfield CSS sur `.dark body` + chaîne de polices
      Geist → Noto Sans JP
- [x] `AppToolbar.tsx` : logo ★ or avec glow, bouton « Traduire » en primary
      (hiérarchie d'action), chip projet+engine en pill, barre de progression
      gradient violet→or
- [x] `columns.tsx` : statuts avec pastille colorée + libellé (cyan=traduit,
      or=relu, ambre=à revoir, muted=non traduit) au lieu de texte seul
- [x] `highlight-utils.tsx` : placeholders en chips cyan bordés, termes
      glossaire en surlignage or pointillé (lisible dans les 2 thèmes)
- [x] En-têtes de panneaux (App.tsx, TMPanel, QAPanel, GlossaryPanel,
      SegmentGrid) : style uppercase + tracking-widest unifié
- [x] Gate : typecheck OK + clippy OK + 304 tests OK + vérif visuelle
      (`docs/screenshots/tenmon-{light,dark}.png`, headless Firefox sur Vite)
- [x] Vérif manuelle dans la vraie app (`pnpm tauri dev` + tauri-mcp-server,
      projet 月咲流ホノカ Wolf, 3011 segments) : starfield + palette OK, logo ★
      + bouton Traduire primary OK, chip projet/WOLF OK, statuts à pastille
      (cyan "Traduit" / muted "Non traduit") OK, QA score 100 vert OK,
      sélection de ligne OK

# Découverte pendant la vérif Tenmon (2026-06-13) — hors scope design

- [x] `PH_RE_SOURCE` (`src/lib/constants.ts`) ne matchait que les codes MV/MZ
      (`\V[12]`, `\C[2]`...) — les codes Wolf en minuscule (`\E`, `\c[2]`,
      `\cself[n]`, `\r[Base,Ruby]`...) n'étaient jamais surlignés en chip
      cyan dans SourceCell/highlight-utils.
      **Corrigé** : ajout de `PH_RE_WOLF` (miroir JS de `RE_WOLF`
      src-tauri/src/engines/wolf/placeholders.rs) + `getPlaceholderRegex
      (engine)` dans constants.ts, branché dans `SourceCell` via
      `useActiveProject().engine`. Validé sur CommonEvent.dat segment 5
      (`\r[甘,あま]\r[酸,ず]` → chips cyan).

# Phase 2 Tenmon — éléments visuels (2026-06-13)

Items client-side de demo-1-tenmon.html (anneaux QA, barre récap, toolbar
"constellation"). Les anneaux de progression par fichier dans le FileTree
sont différés (voir tâche dormante ci-dessous).

- [x] `QAPanel.tsx` : `ScoreBadge` plat remplacé par un anneau SVG doré
      (`QAScoreRing`, r=22, circonférence ≈138.2) coloré selon le score
      (or=100, jaune>=75, rouge sinon). Validé : score 75 (anneau
      jaune/partiel + erreur placeholder) et score 100 (anneau or plein +
      "Segment OK").
- [x] `SegmentGrid.tsx` : footer étendu avec un récap par statut
      (pastilles colorées + compteurs, réutilise `STATUS_STYLES` exporté de
      columns.tsx), dérivé en `useMemo` depuis `segments` (pas d'appel
      backend). Validé : "2 195 segments • 2 195 Non traduit".
- [x] `AppToolbar.tsx` : barre de progression remplacée par une
      "constellation" (`ConstellationProgress`) — nœuds-losanges fixes
      (8/26/45/78/94%), gradient violet→or, comète ★ pulsante à la position
      courante. Validé visuellement (progress=62 temporaire, revert après
      capture).
