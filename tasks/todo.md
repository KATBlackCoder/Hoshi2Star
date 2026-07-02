# Tasks — Hoshi2Star

> **Remédiation audit 2026-07-01 — avancement**
> Ordre : 1 → 2 → 3 → 4 → 5 → 7 → 6 → 8 → 9 (`docs/audit-remediation-plan-2026-07-01.md`).
> - Phase 1 : ✅ **terminée & mergée sur `main`** (2026-07-01).
> - Phase 2 : ✅ **terminée & mergée sur `main`** (2026-07-01).
> - Phase 3 : ✅ **terminée & mergée sur `main`** (2026-07-01).
> - Phase 4 : ✅ **terminée & mergée sur `main`** (2026-07-01).
> - Phase 5 : ✅ **terminée & mergée sur `main`** (2026-07-02).
> - Phase 7 : ✅ **terminée** (2026-07-02) — 23 tests front (Vitest) + 2 tests
>   d'intégration Rust ; `pnpm test` ajouté au gate. Branche `feat/test-coverage`.
> - Phase 6 : ✅ **terminée & mergée sur `main`** (`de0f61a`, 2026-07-02).
> - Phase 8 : ✅ **terminée & mergée sur `main`** (2026-07-02) — ADR-006.
> - 👉 **PROCHAINE : Phase 9 — performance** (dernière ; sur signal, pas prématuré).

## ✅ Phase 8 — Centraliser le dispatch moteur (OCP) — TERMINÉE

> Plan §Phase 8. **Sites vérifiés sur le code réel (2026-07-02)**. Refactor **majeur**,
> **iso-comportement** : le filet est la suite Phase 7 (367 Rust + 2 intégration + 23 front) ;
> chaque étape doit laisser le gate 100 % vert **sans modifier un test existant** (ajout de
> tests de caractérisation AUTORISÉ, cf. étape 0). Toute divergence de comportement = STOP +
> re-plan. Branche : `refactor/engine-dispatch-phase-8`.
>
> **Décision de design (tranchée) : enum + méthodes, PAS de trait-objects.** Le titre « OCP »
> tenterait une réécriture en `trait Engine`, mais (a) l'ensemble des moteurs est *fermé* et
> petit (3→4 avec Bakin), (b) l'audit lui-même dit « enum `FileType` » + « ajouter Bakin = 1
> arm ». Un trait-object = diff plus gros et plus risqué pour zéro gain d'extensibilité réel
> ici. → On donne des **méthodes à `Engine`** (`db_str`, `data_dir`, `game_title`) pour tuer
> les 3 `match` de métadonnées, et on centralise la **classification des `file_type`** pour
> qu'`export.rs` cesse ses `starts_with("wolf_")`/`("vx_")` ad hoc. Trait = alternative
> **rejetée** (documentée).
>
> **Découpage en 2 commits (comme Phase 6)** — gate vert entre les deux :
> - **Commit A — décomposition pure** de la god-fn `open_project`, **sans nouvelle
>   abstraction** : extraire les helpers d'insertion + un `extract_project` par moteur
>   renvoyant *exactement* les formes actuelles.
> - **Commit B — dispatch centralisé** : introduire les méthodes d'`Engine` + la
>   classification `file_type`, router les 4 commandes.

### Sites de dispatch relevés (source de vérité)
- `project.rs::open_project` (33-284, god-fn) : `match engine` ×4 — `engine_str` (80-84),
  `data_dir` (88-94), `game_title` (97-101), extraction+INSERT (128-249, 3 bras au boilerplate
  `source_files`+`segments` dupliqué).
- `project.rs::debug_dump_segments` (~575-680) : 3-bras parallèle qui **re-détecte + ré-extrait**
  (duplique l'extraction d'open_project) ; `kind` string spécifique moteur.
- `export.rs::export_project` (229, 258, 272) : branche `has_wolf` → `collect_wolf_zip_entries`
  sinon boucle générique + sous-dispatch `starts_with("vx_")`.
- `export.rs::debug_inject_file` (498-541) : dispatch 3 voies `wolf_`/`vx_`/json.
- `export.rs` helpers font (37-51) : `engine=="wolf"` → laissés (config, pas dispatch fichier).

### Étape 0 — Filet de caractérisation (AVANT tout refactor)
- [x] Test snapshot de `debug_dump_segments` sur la fixture MV réelle (JSON figé) →
      convertit ce chemin d'« inspection seule » en « prouvé par le gate ». Ethos Phase 7.
- [x] (VX Ace `open_project` reste non couvert par e2e — moteur désactivé, pas de fixture ;
      cf. Vérification. On NE crée pas de fixture VX ici.)

### Commit A — Décomposer `open_project` (aucune abstraction nouvelle)
- [x] Extraire `insert_source_file(&mut tx, …)` + `insert_segment(&mut tx, …)` pour tuer le
      boilerplate INSERT répété 3×. **Contrainte iso critique** : les helpers prennent
      `&mut Transaction` (PAS `&pool`) — ne PAS sortir les inserts de la transaction unique
      ni changer l'atomicité (le happy-path e2e ne détecterait pas un rollback cassé).
- [x] Extraire `extract_project(engine, game_dir, data_dir) -> Vec<(file_name, file_path,
      file_type, Vec<CommonSeg>)>` — forme commune consommée par `open_project` ET
      `debug_dump_segments`. **Contraintes iso** :
      - Wolf : `file_path` est *construit* (`Data/{MapData|BasicData}/name`) — cette logique
        migre dans le bras Wolf du normalizer, ne pas la perdre.
      - Champ source : `.source` (MV/VX) vs `.source_text` (Wolf) → mapper les deux.
      - `debug_dump` : le `kind` est spécifique moteur (Wolf `"map_message"`… ; MvMz
        `format!("{:?}")`) — l'extracteur unifié doit **reproduire chacun**, ne pas collapser.
      - VX : garder l'appel `vx_extractor::extract_from_bytes` intact, juste re-câbler sa
        sortie (chemin non couvert e2e → relecture).
- [x] Gate complet vert, aucun test existant modifié (test étape 0 inclus).

### Commit B — Dispatch centralisé (enum + méthodes)
- [x] `Engine::db_str(&self) -> &'static str` → remplace le `match` de `engine_str` (80-84) et
      la construction manuelle dans manifest/insert.
- [x] `Engine::data_dir(&self, game_dir) -> Result<PathBuf,String>` → remplace (88-94).
- [x] `Engine::game_title(&self, game_dir, data_dir) -> Option<String>` → remplace (97-101).
- [x] ~~`Engine::from_db_str(&str) -> Option<Engine>`~~ **RETIRÉ avant merge** : le
      consommateur export.rs prévu ne s'est pas matérialisé (font scopé hors périmètre) →
      code mort masqué au `clippy dead_code` par `pub`. Retiré par discipline de portée
      (relevé advisor).
- [x] Classification `file_type` centralisée (ex. `FileClass::{Wolf,VxAce,Json}` +
      `fn classify(file_type: &str) -> FileClass`) → `export.rs` (229/272/498) et
      `has_wolf`/font-prefix consomment ça au lieu de `starts_with`.
- [x] Router `open_project`, `debug_dump_segments`, `export_project`, `debug_inject_file` via
      ces méthodes/classification.
- [x] Gate complet vert, aucun test existant modifié.

### Vérification (⚠ trous de couverture explicites — advisor)
- [x] `pnpm typecheck && pnpm test && cargo clippy -- -D warnings && cargo test` — vert,
      aucun test existant modifié.
- [x] **Couvert par e2e** : `open_project`+`export_project` pour **MV et Wolf** (fixtures
      réelles). Le round-trip Wolf CE verrouille toujours Phase 1.
- [x] **NON couvert par e2e → prouvé autrement** :
      - `debug_dump_segments` → snapshot étape 0 (gate-proven).
      - `debug_inject_file` → relecture (pas de test e2e) ; vérifier les 3 voies inchangées.
      - `open_project` **bras VX Ace** → relecture seule (moteur désactivé, sans fixture) ;
        appel extracteur intact.
- [x] Diff `main` vs branche : confirmer que Bakin = « 1 module + 1 arm/impl » devient vrai.
- [x] **Hors périmètre** (ne PAS toucher) : `tokenizer.rs:95/135`, `qa.rs:225`
      (`engine=="wolf"` = config par moteur, pas dispatch de fichier ; hors liste de fichiers
      de la phase). Backlog inchangé (doublons TM, ESLint 10, QA live sans glossaire, 8 lints
      clippy --all-targets).

### Docs (fin de phase)
- [x] `docs/architecture.md` : documenter les méthodes d'`Engine` + la classification comme
      point de dispatch unique.
- [x] ADR si l'abstraction est structurante (enum-méthodes vs trait — décision + rejet trait).
- [x] CHANGELOG.md (Changed) + docs/journal/ + tasks/todo.md cochés.

## ✅ Phase 6 — DRY backend + UI — TERMINÉE

> Plan §Phase 6. **Sites vérifiés sur le code réel (2026-07-02)** — voir n° de
> ligne ci-dessous. Pas un bugfix → pas de « test rouge » ; le filet est la suite
> Phase 7 (367 Rust + 2 intégration + 23 front) : **chaque refacto doit laisser le
> gate 100 % vert sans changer un seul test** (refacto = iso-comportement). Toute
> divergence de comportement observée = ce n'était pas une simple duplication →
> STOP + re-plan. Branche : `refactor/dry-phase-6`.
>
> **Ordre** : backend d'abord (1→5, un commit), puis UI (6→9, un commit), merge
> unique. Gate complet entre les deux blocs.

### Volet backend

- [x] **1. Stats recompute (dupliqué 3×)** → `core/manifest.rs::refresh_stats`.
      Le bloc `SELECT (…4 COUNT…) FROM … WHERE …` + `manifest::update_stats(…)`
      est copié-collé à `project.rs:421-450` (clé `s.id = ?`),
      `translate.rs:148-174` (clé `s.id = ?`) et `translate.rs:368-394` (clé
      `project_id`). Extraire `pub async fn refresh_stats(pool, project_id)` (best-
      effort, ne renvoie pas d'erreur) ; `update_segment`/`translate.rs:148`
      résolvent `project_id` depuis le segment d'abord (une requête triviale, ou
      variante `refresh_stats_for_segment`). **Attention** : garder le caractère
      *best-effort* (les 3 sites ignorent l'erreur) — ne pas transformer en
      `?`/propagation.
- [x] **2. Filtre glossaire pertinent (dupliqué)** → `core/glossary.rs`.
      `translate.rs:105-128` (charge `list_for_project`, filtre
      `pairs.contains(t.source_text)`, fallback par longueur si vide) est répété
      pour la passe projet. Extraire `pub async fn relevant_terms(pool,
      project_id, lang_pair, sources: &[…]) -> Vec<(String,String)>` avec la même
      logique de fallback ; les 2 passes l'appellent.
- [x] **3. Test « fichier Map ? » (dupliqué 4×)** → helper partagé.
      `project.rs:814, 886, 916` + `vx_ace/extractor.rs:435` font tous
      `file_name.starts_with("Map")` + `trim_start_matches("Map")`. Extraire
      (`engines/mod.rs` ou `utils`) `fn map_id_from_name(file_name) ->
      Option<&str>` (ou `is_map_file` + extraction du numéro) ; les 4 sites
      l'utilisent. **Vérifier** : les 3 sites de project.rs partagent-ils
      exactement la même sémantique (extension `.json` implicite) — sinon garder
      le helper minimal (booléen + id) sans forcer une unification abusive.
- [x] **4. Table `QaError` pénalité + label (match répété)** →
      `QaError::kind()`/table unique. `qa.rs:262-264` (pénalités 10/15/15),
      `qa.rs:283-330` (`label_en`/`label_fr`), et `report.rs` (labels) répètent le
      `match` par variante. Introduire une seule source (ex. `impl QaError { fn
      penalty(&self)->i32; fn label(&self, lang)->String }` déjà partiellement là)
      et faire consommer `report.rs` par ces méthodes plutôt qu'un `match`
      parallèle. **Risque moyen** : le score QA est testé (`qa.rs` tests) — le gate
      doit rester vert à l'identique (mêmes pénalités, mêmes libellés fr/en).
- [x] **5. glossaire — `SELECT` 9-col + N+1 dédup** → `core/glossary.rs`.
      Le `SELECT id, source_text, target_text, lang_pair, domain, project_id, …`
      est répété à `:116, :127, :307` → une `const GLOSSARY_COLUMNS` (ou fonction
      de mapping `FromRow`). Corriger le **N+1** de dédup (`:230-253` : un `SELECT
      COUNT(*)` par terme dans la boucle `for term in …take(50)`) par un pré-fetch
      unique des `source_text` existants (un seul `SELECT source_text WHERE
      project_id = ?` → `HashSet`) puis test en mémoire. **Iso-comportement** :
      mêmes termes insérés/ignorés qu'avant.

### Volet UI

- [x] **6. Helper « ouvrir un jeu + mapper l'erreur moteur » (dupliqué 3×)**.
      `AppToolbar.tsx:60-66` + `ProjectList.tsx:60-66, 101-107` refont le même
      `catch` → `msg.includes("could not identify game engine") ? clé A : clé B`.
      Extraire un util (`lib/openProject.ts` ou méthode de `stores/project.ts`)
      qui invoque `open_project`, mappe l'erreur en clé i18n, et laisse le
      composant afficher le toast. Ne pas déplacer l'UI (toast) dans le store.
- [x] **7. Hook `useExportToFile(...)`** → factorise `QAPanel.tsx:127-140` et
      `TMPanel.tsx:75-87` (même séquence `save()` dialog → `invoke(cmd,…)` →
      `toast.success/error`). Hook générique `(cmd, args, {successKey, errorKey})`.
- [x] **8. Source unique statut→couleur** (scopé, validé advisor) — extrait
      `lib/statusSummary.ts` (`STATUS_SUMMARY` : convention chip ✓/◎/⚠/○ partagée
      FileTree + ProjectList, classes verbatim). **PAS** fusionné avec
      `STATUS_STYLES` (badge grille cyan/or) : concepts/teintes distincts →
      fusion = changement de comportement. — **scoper d'abord** : ce sont
      potentiellement 3 échelles *différentes* (statut de segment `STATUS_STYLES`
      / `SegmentStatsBar` ; icône par *type de fichier* `FileTree fileIcon` ;
      qualité de *match TM* `MatchBadge`). N'unifier que ce qui est réellement le
      même concept (statut de segment) dans un `lib/statusStyles.ts` ; **ne pas**
      fusionner de force des palettes de concepts distincts (anti-DRY abusif).
- [x] **9. Listener `h2s://glossary/extraction-done`** (scopé, validé advisor) —
      extrait uniquement le type de payload `GlossaryExtractionDonePayload`
      (types.ts), utilisé par les 3 listeners. **PAS** fusionné en un listener
      unique : les 3 mettent à jour des états/toasts distincts → fusion = refonte
      d'architecture d'état, hors iso-comportement. — original ci-dessous :
      `useAppHandlers.ts:44`, `SegmentGrid.tsx:240`, `GlossaryPanel.tsx:248`.
      Vérifier s'ils font la *même* chose (recharger le glossaire / rafraîchir la
      grille) ; si oui, centraliser en **un** listener (probablement dans
      `useAppHandlers` ou un hook dédié) qui met à jour le store, les autres
      consommant le store. **Attention** : ne pas casser le rafraîchissement de la
      grille après extraction (comportement observable Phase 3/5).

### Vérification
- [x] Backend (1-5) : gate complet vert **sans modifier aucun test** —
      `pnpm typecheck && pnpm test && cargo clippy -- -D warnings && cargo test`.
      → 367 Rust + 2 intégration + 23 front, clippy exit 0, typecheck clean.
- [x] UI (6-9) : idem ; les tests front existants (`ProjectList`, `SegmentGrid`,
      `FileTree`, `useAppHandlers`) restés verts inchangés → iso-comportement prouvé.
      Réserve : `useExportToFile` (item 7) prouvé iso **par inspection** (QAPanel/
      TMPanel sans test) — tests panneaux relèvent de Phase 7 (hors périmètre).
- [x] `docs/architecture.md` : `manifest::refresh_stats` noté point unique de
      recalcul de stats.
- [x] CHANGELOG.md (Changed) + docs/journal/2026-07-02-dry-phase-6.md + todo cochés.
- [ ] **Hors scope** (log backlog, ne pas traiter ici) : doublons TM (index non-
      UNIQUE), `pnpm lint` cassé (ESLint 10), QA live sans glossaire, 8 lints
      `clippy --all-targets` dans le code de test.

## ✅ Phase 7 — Couverture de tests front + intégration — TERMINÉE

> Plan §Phase 7. **Vérifié sur le code réel (2026-07-02)** : `package.json` n'a
> AUCUN script `test` (CLAUDE.md documente pourtant `pnpm test` — mensonge à
> corriger par l'implémentation) ; pas de `vitest.config.ts` ; `src-tauri/tests/`
> existe mais est VIDE ; `Cargo.toml` sans feature `test` de tauri ; tous les
> modules lib.rs sont `pub` (tests d'intégration possibles sans churn).
> Branche : `feat/test-coverage`. Pas de « test rouge » ici (pas un bugfix) —
> le critère est : chaque fix des Phases 3/5 verrouillé par un test qui échouerait
> si on le révertait.

### Volet A — Infra Vitest
- [ ] `pnpm add -D vitest jsdom @testing-library/react @testing-library/jest-dom
      @testing-library/user-event`
- [ ] `vitest.config.ts` (environnement jsdom, setupFiles) — séparé de
      vite.config.ts (qui porte les réglages Tauri)
- [ ] `src/test/setup.ts` : jest-dom + init i18n (locale en) + `mockIPC`
      (`@tauri-apps/api/mocks`) + `vi.mock` de `plugin-store`/`plugin-dialog`
- [ ] `package.json` : scripts `test` (= `vitest run`) et `test:watch`

### Volet B — Tests stores (Zustand)
- [ ] `settings.ts` : défauts = `lib/constants` (gemma4:e4b) ; `saveSettings`
      persiste (mock plugin-store) ET pousse `providerConfig` dans le store llm
- [ ] `llm.ts` : `setProviderConfig` (merge partiel), `reset`, défauts = constants
- [ ] `project.ts` : `openProject` (invoke mocké) → projet ajouté + actif +
      stats fetchées ; erreur → propagée au caller
- [ ] `editor.ts` : `setActiveSegment` / `setGlossaryTerms` + sélecteurs

### Volet C — Tests composants/hooks (verrouillent les fixes Phases 3/5)
- [ ] `ProjectList` : `get_project_stats` rejette pour p1, résout pour p2 →
      la barre de p2 s'affiche quand même (allSettled, Phase 3)
- [ ] `FileTree` : fichier `⚠ M > 0` → compteurs affichés, PAS de bouton inject ;
      fichier 100 % translated → `✓ N` + bouton (Option 2, Phase 5)
- [ ] `SegmentGrid` (mock `@tanstack/react-virtual` → tous les items rendus) :
      · chargement multi-pages : mock `get_segments` paginé → N appels jusqu'à
        `total`, footer = total réel (Phase 3)
      · sélection : cocher 2 lignes → « Traduire 2 lignes » ; taper une recherche
        → bouton disparu (reset, Phase 3)
      · `update_segment` rejette → toast d'erreur `saveError` (Phase 3)
- [ ] `useAppHandlers` (renderHook) : gate export — `untranslatedCount > 0` →
      dialog `blocked`, sinon `confirm`

### Volet D — Intégration Rust `src-tauri/tests/`
- [x] `Cargo.toml` `[dev-dependencies]` : `tauri = { version = "2",
      features = ["test"] }` + `zip` (les tests d'intégration n'héritent pas
      des `[dependencies]`)
- [x] `tests/e2e_project_flow.rs` : `mock_builder().manage(AppState).build(
      mock_context(noop_assets()))` ; **appel direct** des `pub async fn`
      (`open_project`/`update_segment`/`export_project`) avec un `State` via
      `Manager::state()` — teste la couche logique, pas la sérialisation IPC ;
      `mock_context(noop_assets())` évite la dépendance à `dist/` (gitignoré)
- [x] Scénario MV : `open_project` sur la fixture réelle
      (`test/性処理係のある学校`, copie de `www/data` seule) → `update_segment`
      d'1 segment → `export_project` → le ZIP contient la traduction
- [x] Scénario Wolf CE (**verrouille Phase 1**) : `open_project` sur Honoka
      (copie `Data/BasicData`+`MapData` + stub `Game.ini`) → traduire 1 segment
      `wolf_common_events` → `export_project` → `Data/BasicData/CommonEvent.dat`
      du ZIP contient la traduction (marqueur ASCII, identique en UTF-8 et
      Shift-JIS)
- [x] Portabilité (leçon Phase 1) : `if !fixture.exists() { return; }` — skip
      propre sur un checkout sans fixtures ; isolation via `tempfile::tempdir`
      (le `.hoshi2star.json` et le `hoshi2star.zip` sont écrits DANS le tempdir →
      cleanup gratuit au `Drop`, aucune pollution des fixtures)

### Vérification
- [x] Gate étendu : `pnpm typecheck && pnpm test && cargo clippy -- -D warnings
      && cargo test` — 23 tests front + 367 unitaires Rust + 2 intégration ✅
- [x] CLAUDE.md : la ligne `pnpm test # Vitest` devient vraie (script ajouté) ;
      gate `pnpm test` ajouté à la section vérification de CLAUDE.md
- [x] docs/architecture.md : nouvelle section « Tests » (pyramide : unitaires
      inline Rust · intégration tests/ · front Vitest)
- [x] CHANGELOG.md (Added) + docs/journal/ + tasks/todo.md cochés
- [ ] Backlog (hors scope) : 8 lints `clippy --all-targets` préexistants dans
      le code de test `#[cfg(test)]` (`non_snake_case`, `if_same_then_else`) —
      non couverts par le gate mandaté (sans `--all-targets`)

## ✅ Phase 5 — Cohérence #8 (Option 2) + alignements doc/valeurs — TERMINÉE

> Plan §Phase 5 + complément §6. Décision produit #8 **déjà tranchée : Option 2**.
> **Vérifié sur le code réel (2026-07-01)** : `get_source_files` compte
> `target_text != ''` (project.rs:295) ; `get_project_stats` ignore `reviewed`
> (:461-489) ; FileTree `isComplete` gate le bouton inject (:177-178, aucun
> compteur affiché aujourd'hui) ; `SegmentStatsBar` ne rend pas `reviewed`
> (ProjectList.tsx:238-272) ; modèle par défaut divergent 3× (provider.rs:83
> `q4_K_M` · settings.ts:23 `q8_0` · llm.ts:50 `qwen3:4b`) ; `H2sError` 0
> occurrence (CONTEXT.md:195, CLAUDE.md:175) ; « skeleton » CLAUDE.md:69.
> ⚠ **Types IPC modifiés** → docs/architecture.md À METTRE À JOUR.
> Branche : `fix/status-consistency`.

### Étape A — Backend : compteurs stricts + reviewed
- [x] Helpers `fetch_source_files` / `fetch_project_stats` (`&SqlitePool`,
      commands = façades) — testables sans `tauri::State`
- [x] `translated_count` strict (`status='translated'`) + `needs_review_count`
      par fichier ; `reviewed_count` projet (somme des 4 == total)
- [x] `domain/types.rs` : 2 nouveaux champs (`#[sqlx(default)]` couvre les
      requêtes à littéraux `0 as …`, ex. export.rs:447)
- [x] Test rouge prouvé : `translated_count` retournait **4** au lieu de 1
      (4 segments, 1 par statut, tous target non vide) →
      `test_counter_semantics_one_segment_per_status` vert, fige les 4 compteurs

### Étape B — Frontend : miroir types + FileTree + barre
- [x] `lib/types.ts` : `needsReviewCount` / `reviewedCount` (doc-comments)
- [x] `FileTree.tsx` : compteurs `✓ N · ⚠ M` (verts/ambre) ; inject seulement si
      100 % strictement `translated`
- [x] `SegmentStatsBar` : segment `reviewed` bleu (`◎ N`) — somme à 100 %
- [x] Consommateurs de `translatedCount` vérifiés (grep) : FileTree (voulu),
      AppToolbar % (déjà strict via ProjectStats), project.ts toast (idem)

### Étape C — Alignements annexes
- [x] Modèle par défaut unifié sur **`gemma4:e4b`** (choix utilisateur, tag
      vérifié sur son Ollama local — remplace q4_K_M/q8_0/qwen3:4b) :
      source TS unique `lib/constants.ts` (settings.ts importait déjà llm.ts →
      un import inverse aurait créé un cycle) + miroir Rust `DEFAULT_OLLAMA_MODEL`,
      commentaires croisés. Section RunPod de CONTEXT.md non touchée (exemple
      de workflow cloud, pas le défaut de l'app).
- [x] CONTEXT.md + CLAUDE.md : convention `Result<T, String>` + thiserror par
      module actée (pas de `H2sError` global) ; « skeleton » → état réel

### Vérification
- [x] Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ ·
      `cargo test` = **367 pass / 0 fail / 4 ignored** (+1)
- [x] **docs/architecture.md mis à jour** : section domain/types.rs (Option 2,
      helpers, invariant somme, UI ✓/⚠) + miroir constants.ts
- [x] CHANGELOG.md (Fixed + Changed) + docs/journal/ + tasks/todo.md cochés
- [x] Vérification visuelle en live (2 statuts basculés temporairement puis
      restaurés) : ProjectList `✓ 15496 · ◎ 1 · ⚠ 1 · ○ 0 — 15498` (somme
      exacte) ; FileTree `Map118 ✓ 44` + bouton inject vs `Map119 ✓ 559 · ⚠ 1`
      SANS bouton inject

## ✅ Phase 4 — Correctness du moteur QA (P2, faux résultats) — TERMINÉE

> Plan de `docs/audit-remediation-plan-2026-07-01.md` §Phase 4 + complément §1.2/§1.3.
> **Vérifié sur le code réel (2026-07-01)** : les 3 défauts confirmés —
> `qa.rs:134` (`'\u{FF00}'..='\u{FFEF}'` entier compté largeur 2),
> `report.rs:231-232` (`total_checked = details.len()`, le commentaire l'avoue),
> `report.rs:71` (`qa::check(..., &[], ...)`, doc de module :4 « Glossary terms are
> not applied »). `export_qa_report` vit dans `commands/export.rs:366` (pas qa.rs).
> Branche : `fix/qa-correctness`.

### Étape 1 — `qa.rs` : katakana demi-largeur comptés pleine largeur
- [x] Test rouge prouvé : `measure_line_units("ｱｲｳ")` retournait 6.0 (attendu 3.0)
- [x] Fix : `'\u{FF00}'..='\u{FFEF}'` → sous-plages EAW **F** réelles
      (`FF00–FF60` + `FFE0–FFE6`) ; gardes `｡･` == 2.0 et `Ａ￥` == 4.0

### Étape 2 — `report.rs` : dénominateur « X / Y vérifiés » toujours X == Y
- [x] `collect_qa_details` → `(total_checked, details)` (total = lignes avant
      filtrage `score < 100`)
- [x] `generate_qa_html(…, total_checked, lang)` — plus de `details.len()`
- [x] `export_qa_report` propage le tuple
- [x] Test : « 1 segments with errors / 10 checked » avec 1 détail + total 10

### Étape 3 — `report.rs:71` : check glossaire mort — Option A (câblé)
- [x] `collect_qa_details(pool, project_id, terms)` — termes injectés
- [x] `export_qa_report` charge `glossary::list_for_project(…, "ja-en")` →
      map `(source_text, target_text)`
- [x] Doc de module `report.rs:1-6` mise à jour
- [x] Test rouge intégration prouvé (pool SQLite via `db::pool::init` + tempfile,
      pattern déjà présent dans pool.rs — pas de nouvelle infra nécessaire) :
      2 segments traduits, s1 viole ハルカ→Haruka → AVANT fix : `got: []` ;
      APRÈS : `GlossaryMismatch` sur s1 + `total_checked == 2` + `details.len() == 1`

### Hors scope (loggé → Backlog)
- `qa_check_segment` (`commands/qa.rs:37`) et `update_segment`
  (`commands/project.rs:388`) passent toujours `&[]` → le QA live ne détecte pas
  les GlossaryMismatch (cohérent : le panneau live n'affiche pas de stat glossaire).

### Vérification
- [x] Tests rouges prouvés avant fix (kana 6.0≠3.0 · glossaire `got: []`)
- [x] Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ ·
      `cargo test` = **366 pass / 0 fail / 4 ignored** (+3)
- [x] docs/architecture.md : non touché (signatures internes core/, aucun type
      IPC ni commande modifiés)
- [x] CHANGELOG.md (Fixed) + docs/journal/ + tasks/todo.md cochés

**Phase 4 : livrée — largeur kana correcte, dénominateur réel, GlossaryMismatch
vivant dans le rapport (premier test d'intégration DB du module report).**

## ✅ Phase 3 — Robustesse des promesses UI (P1, silencieux) — TERMINÉE

> Plan de `docs/audit-remediation-plan-2026-07-01.md` §Phase 3 + complément §1.4/§1.5.
> **Vérifié sur le code réel (2026-07-01)** : les 4 défauts sont confirmés aux lignes
> de l'audit. Fait nouveau : `get_segments` retourne DÉJÀ `total` (vrai `COUNT`,
> `project.rs:332`) et `PaginatedSegments.total` existe côté TS (`types.ts:72`) —
> l'UI l'ignore. Branche : `fix/ui-promise-robustness`.
> **Contrainte** : aucun test front n'existe (Vitest = Phase 7) → vérification
> manuelle via `pnpm tauri dev` + gate typecheck/clippy/tests (le plan l'acte).

### Étape 1 — `ProjectList.tsx:44-51` : stats tout-ou-rien
- [x] `Promise.allSettled` : un `get_project_stats` en échec ne rejette plus le lot

### Étape 2 — `SegmentGrid.tsx:230-244` : `handleSave` rejet non géré
- [x] try/catch autour de `invoke("update_segment")` + `toast.error`
- [x] i18n : clé `segmentGrid.saveError` dans `en.json` + `fr.json`

### Étape 3 — `SegmentGrid.tsx` : sélection → mauvais segments après filtrage
- [x] `getRowId: (row) => row.id` → `rowSelection` keyée par id de segment ;
      `selectedIds` sans mapping par index — désigner un autre segment devient impossible
- [x] Reset de `rowSelection` sur changement de `qaFilter`/`searchQuery`

### Étape 4 — `SegmentGrid.tsx:66-77` : plafond 5000 lignes silencieux
> Décision validée : **chargement par pages successives** (pageSize 2000).
- [x] `loadSegments` : boucle jusqu'à `total` + garde d'annulation `loadSeqRef`
      (un load périmé n'écrase ni `segments` ni `isLoading` du fichier suivant)
- [x] Footer/`totalCount` : corrects une fois le fichier complet (`isLoading` couvre
      le chargement)
- [x] Doc-comment `get_segments` aligné (pages successives de 2000)

### Étape 5 — `QAPanel.tsx` / `TMPanel.tsx` : dialogues `save()` non gardés
- [x] `await save(...)` dans try/catch → toast `exportError` existante (les 2 panneaux)

### Vérification
- [x] Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ · `cargo test`
      = 363 pass (Rust intact hors doc-comment)
- [x] **Vérification manuelle via MCP Tauri** (`pnpm tauri:linux`, projet réel MV) :
      · ProjectList : stats des 2 cartes affichées (✓ 15498 / ✓ 22803) — allSettled OK
      · Map119.json (561 segments) chargé ENTIER, footer « 561 segments », filtré « 6 / 561 »
      · Boucle multi-pages prouvée sur le protocole réel : 6 pages × 100 → 561 ids uniques
      · Sélection : 2 lignes cochées → « Traduire 2 lignes » ; recherche modifiée →
        bouton disparu + 0 case cochée (reset OK)
      · handleSave : édition « Sandwich » → « Sandwich. » persistée (updatedAt bump),
        puis restaurée à l'identique (TM nettoyée, cf. découverte ci-dessous)
- [x] docs/architecture.md non touché (aucun changement d'architecture)
- [x] CHANGELOG.md (Fixed) + docs/journal/ + tasks/todo.md cochés

**Phase 3 : livrée — 5 fixes UI, vérifiés en live sur l'app dev pilotée via MCP.**

### Découvertes hors scope loggées (→ Backlog)
- 🐛 **TM : doublons à chaque re-sauvegarde** — `tm.rs::insert` utilise
  `INSERT OR REPLACE` mais le PK est un UUID neuf et `idx_tm_hash_lang` n'est PAS
  UNIQUE → le REPLACE ne remplace jamais, une ligne s'ajoute à chaque save du même
  segment (observé en live : 2 entrées pour le même `source_hash`). Fix : migration
  index UNIQUE (source_hash, lang_pair) + dédup des données existantes + upsert.
- 🐛 **`pnpm lint` cassé** (pré-existant, vérifié sur arbre propre) : ESLint 10
  exige `eslint.config.js` (flat config), le repo a encore `.eslintrc.*`.
- Phase 7 : tests front à écrire pour les 4 fixes (ProjectList allSettled,
  handleSave erreur, sélection/filtre, chargement complet multi-pages)

## ✅ Phase 2 — Éliminer les paniques des parsers binaires Wolf (P1, crash) — TERMINÉE

> Plan de `docs/audit-remediation-plan-2026-07-01.md` §Phase 2 + complément §2.
> **Vérifié sur le code réel (2026-07-01)** : tous les sites sont encore aux lignes de
> l'audit (le code n'a pas bougé). Tous les champs `DxFileEntry` sont `u64` attaquant
> (`legacy_xor.rs:62-66`) → les casts `as usize` + additions brutes peuvent réellement
> wrapper même en 64-bit. Branche : `fix/wolf-parser-panics`.
> **NE PAS toucher** : les `catch_unwind` d'`extractor.rs` (garde-fou voulu).
> Convention erreur : réutiliser `DecryptorError::HeaderTooShort` (déjà utilisé pour
> toutes les bornes voisines :555/582/1185/1196…) — pas de nouveau variant.

### Étape A — Racine : allocation Huffman non bornée (`legacy_xor.rs:731`)
- [x] Test rouge : `huffman_decode(&[0xFF;16])` (orig_bits=64, orig_size≈u64::MAX) →
      panique « capacity overflow » prouvée
- [x] Fix : const `MAX_DECODE_OUTPUT = 1 GiB` (partagée Huffman/LZ) → `return None`
      (les appelants mappent déjà `None` → `Err`)
- [x] Même classe : `lz_decode` — `dest_size` plafonné (rouge prouvé : retournait
      `Some(vec 4 GiB)`)

### Étape B — Slice TOC non bornée (`legacy_xor.rs:591`)
- [x] Test rouge : `make_v5_archive` + helper `patch_v5_toc` (XOR symétrique) avec
      `name_offset=0xFFFF` → panique OOB prouvée
- [x] Fix : `usize::try_from(name_offset)` + borne `ns ≤ toc_data.len()` → `Err(HeaderTooShort)`

### Étape C — Slices Huffman v8 (`legacy_xor.rs:1214/1216` + `:1267/1269`)
- [x] Tests rouges : appels directs `extract_v8_huffman_only`/`assemble_v8_lz_stream`
      avec blob `[0u8;16]` (décode vers tampon VIDE) + `unpacked/press_sz > 2*huff_kb`
      → panique « range end index 1024 out of range for slice of length 0 » prouvée ×2
- [x] Fix : `decoded.len() < huff_kb * 2` → `Err(HeaderTooShort)` dans les 2 fonctions

### Étape D — Tier arithmétique : additions d'offsets en `checked_add`
- [x] `:541` (`saturating_add`), `:554` (check u64 `index_offset+index_size` avant cast),
      `:580` (checked u64 + `try_from`), `:1387` (idem v8)
- [x] Sites frères mêmes fonctions : `:1185`, `:1204-1205`, `:1238`, `:1257-1258`,
      `:1396`, `:1419` + casts `u64→usize` non tronquants (`unp`, `pz`, `hz`) +
      `key_offset + huff_sz` (u64) en `checked_add`
- [x] **Ajouts vérifiés hors liste audit (même classe, même chemin de dispatch v8)** :
      `read_original_name` (`:1016/:1020` saturating), `build_per_file_key_str`
      (`:1049` saturating), `find_parent_dir` (`:992` `checked_mul`+`checked_add`,
      skip du dir forgé)
- [x] Tests rouges : v6 `data_offset=u64::MAX` via `patch_v6_toc` (panique add overflow
      prouvée) + `extract_v8_huffman_only(file_start≈usize::MAX)` (panique add prouvée).
      `:554` non testable en rouge (champs v5/v6 u32 → pas de wrap 64-bit) → défensif.

### Étape E — `v3_format/map.rs:355/369` : multiplication tiles non vérifiée
- [x] Test rouge : dump d'une map valide + patch `width=0x4000_0000, height=4` →
      « attempt to multiply with overflow » prouvé (map.rs:369)
- [x] Fix : `tile_data_len(width,height,layer_cnt)` en `checked_mul` u64 → nouveau
      variant `V3FormatError::TileSizeOverflow` ; branche utf8 : `read_bytes` (borné,
      sans alloc) AVANT `Vec::with_capacity(tile_len)`

### Étape F — `dat_parser.rs` : allocations pilotées par l'entrée
- [x] `read_bytes` : refuse `n > bytes restants` AVANT `vec![0u8; n]` (couvre aussi
      `n*4`/`cnt*4`, passés en `saturating_mul`)
- [x] Helper `bounded_cap(cursor, count)` (`count.min(remaining/4)`) appliqué aux 5
      `with_capacity` attaquants : `parse_project` ×3 + `parse_dat_types` `fields_size`
      + `data_count`
- [x] Tests rouges : `read_bytes(n=usize::MAX)` → « capacity overflow » prouvé ;
      `type_count=u32::MAX` → **SIGABRT alloc 309 Go** prouvé (tuait tout le process !) ;
      `fields_size=u32::MAX` → **SIGABRT alloc 137 Go** prouvé

### Vérification (gate obligatoire)
- [x] 11 tests rouges prouvés AVANT fix (9 paniques + 2 SIGABRT), 11/11 verts après
- [x] Non-régression fixtures réelles : `test_real_honoka_*` (v2 + archive v8 DXA) +
      `test_real_inko_*` (v3.5) tous verts
- [x] Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ (seul warning :
      toolchain `pclmul` pré-existant, hors code) · `cargo test` = **363 pass / 0 fail / 4 ignored**
- [x] CHANGELOG.md (Fixed)
- [x] docs/architecture.md : PAS touché — aucun changement d'architecture (fns internes
      durcies ; seul ajout : variant interne `V3FormatError::TileSizeOverflow`, pas un type IPC)
- [x] docs/journal/ : entrée de session
- [x] tasks/todo.md : coché

**Phase 2 : livrée — 11 sites de panique/abort éliminés (legacy_xor.rs, v3_format/map.rs,
dat_parser.rs), `catch_unwind` d'extractor.rs intacts, prouvé rouge→vert site par site.**

## ✅ Phase 1 — Réinjecter les Common Events Wolf à l'export (P0, perte de données) — TERMINÉE

> Plan de `docs/audit-remediation-plan-2026-07-01.md` §Phase 1. Vérifié sur le code réel
> (2026-07-01). Le bug est confirmé : `inject_all_to_memory`/`inject_all` (injector.rs:483/521)
> ne matchent que `MapData`/`Database`, `_ => {}` (514/568). Aucun `inject_common_events`.

### Étape 1 — Verify read-only + test rouge (priorité absolue)
- [x] Pas de fixture Wolf réelle dans ce checkout (`test/性処理係のある学校` = MV/MZ) →
      reproduction via CE v3 synthétique (`CommonEventsV3::synthetic_messages`), pas de fixture chiffrée
- [x] Test **rouge** via le **regroupement** (extract→`injection_bucket`→`inject_all_to_memory`)
      avec 2 event_names → prouvé rouge (compile-fail sur `injection_bucket` absent, puis
      assert perte). `test_common_events_translations_survive_export`

### Contraintes de correction (silencieuses si ratées — cf. advisor)
- [x] **C1 — Bucket unique** : `injection_bucket()` collapse tous les `CommonEvents/*` en
      `COMMON_EVENTS_BUCKET = "CommonEvents/CommonEvent"`. Les 2 sites de bucketing d'`export.rs`
      (`collect_wolf_zip_entries` + `debug_inject_file`) l'utilisent. Clé complète conservée.
- [x] **C2 — Parité de clés** : injecteur régénère `CommonEvents/{event_name}/{event_idx}/{cmd_idx}`
      (+`/choices/{choice_idx}`), pas de `pages`, `event_name = event.event_name()` (v2) / `event.name` (v3).
- [x] **C3** : test rouge passe par `injection_bucket` + `inject_all_to_memory` (2 event_names).

### Implémentation
- [x] `inject_common_events(bytes, translations, version)` : v2 → `common_events_parser::parse_bytes`
      (catch_unwind) + `patch_common_events_strings` (remplacements ordonnés, dst=src si non traduit) +
      `splice_wolf_strings` (splice partagé extrait de `patch_mps_strings`). v3 → `inject_common_events_v3`
      (`is_lz4_v3` → `decompress` → `CommonEventsV3::parse` → mute `string_args` → `dump` → `recompress`).
- [x] Arm `"CommonEvents"` dans **`inject_all_to_memory` ET `inject_all`** → `load_common_event_bytes`,
      écrit/emet `Data/BasicData/CommonEvent.dat`.
- [x] `export.rs` : les 2 sites routent via `injection_bucket`.

### Vérification
- [x] **v3 (Inko)** vert : `test_common_events_translations_survive_export` (2 CE, both survive) via `CommonEventsV3::synthetic_messages`
- [x] **v2 (Honoka)** vert : fixture v2 synthétique construite à la main (`make_v2_common_event_dat`) →
      `test_v2_common_event_fixture_extracts` + `test_v2_common_events_translations_survive_export` (2 CE) +
      `test_v2_common_events_identity` (byte-exact, doublons → pas de dé-alignement du splice)
- [x] `test_injection_bucket_collapses_common_events`
- [x] **Vérif fichiers réels** (fixtures fournies par l'utilisateur, renommées en `Densyanai_Inko_ver2.0` /
      `月咲流ホノカver1.03` pour matcher les 13 refs de tests existantes) :
      `test_real_honoka_common_events_inject` (v2 : identity byte-exact + trad round-trip) +
      `test_real_inko_common_events_inject` (v3.5 : identity payload décompressé + trad round-trip)
- [x] Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ · `cargo test` = **352 pass / 0 fail / 4 ignored**
- [x] `docs/architecture.md` : section `wolf/injector.rs` + date 2026-07-01
- [x] CHANGELOG.md (Fixed)

**Phase 1 : livrée — v2 (Honoka) ET v3 (Inko) vérifiés sur fixtures synthétiques ET sur les jeux réels. Commitée + mergée sur `main` (2 commits + merge --no-ff).**

### Résolu par les fixtures fournies
- [x] Les 2 tests `test_real_inko_*_round_trip` qui paniquaient (fixture Inko absente) passent maintenant
      (fixtures présentes). Note portabilité : ils paniquent toujours (au lieu de skip) sur un checkout sans
      fixtures — aligner sur `if !path.exists() { return; }` reste un petit nettoyage optionnel.

---

## Complétées (session 2026-06-17/18)

- [x] Wolf extractor skip filters : `X[`/`zz` events, `自動ｼｽﾃﾑ初期化` DB, `@N\n` tokenizer (317 tests)
- [x] `extract_wolf_speakers` Tauri command + bouton "Speakers" dans GlossaryPanel
- [x] `SourceFile.translated_count` / `total_count` + requête SQL `get_source_files`
- [x] `debug_inject_file` Tauri command (Wolf, complétude enforced)
- [x] `scan_font_status` Tauri command + `FontSizeDialog` + `\f[N]` prefix management
- [x] `export_project` étendu avec `fontSize` / `replaceExisting`
- [x] Bouton Debug Inject par fichier dans `FileTree.tsx` (hover, `FlaskConical`)
- [x] Docs CHANGELOG + ROADMAP + architecture.md mis à jour

## Complété — MV/MZ placeholder codes custom + GameTitle (2026-06-19)

- [x] 1. `tokenizer.rs` — Groupe F : `\FF[...]`, `\F[...]`, `\AA[...]` dans RE_MVMZ + RE_MZONLY
- [x] 2. `mv_mz/extractor.rs` — GameTitle + " by Hoshi2Star"
- [x] 3. 6 tests Groupe F + test GameTitle mis à jour
- [x] 4. 323 tests ✓ · clippy ✓

## Complété — Debug Extraction universelle (2026-06-19)

Objectif : rendre le bouton Bug (debug dump JSON) disponible pour tous les moteurs,
pas uniquement Wolf. Le JSON doit avoir un format unifié pour que Claude puisse
l'analyser et identifier ce qui mérite traduction vs ce qui peut être skippé.

- [x] 1. `commands/project.rs` — ajouter `debug_dump_segments` générique (dispatch par moteur)
- [x] 2. `commands/project.rs` — supprimer `debug_dump_wolf_segments` (remplacé)
- [x] 3. `lib.rs` — remplacer `debug_dump_wolf_segments` par `debug_dump_segments`
- [x] 4. `AppToolbar.tsx` — retirer la condition `engine === "wolf"`, appeler `debug_dump_segments`
- [x] 5. Vérification : cargo clippy ✓ · pnpm typecheck ✓

## En cours — Stats de segments (2026-06-26)

Trois emplacements : toast post-extraction · barre dans ProjectList · % dans toolbar pill.

- [x] 1. Rust `domain/types.rs` — ajouter `translated_count` + `needs_review_count` à `ProjectStats`
- [x] 2. Rust `commands/project.rs` — étendre la query SQL de `get_project_stats` (5 sous-requêtes)
- [x] 3. TS `lib/types.ts` — ajouter interface `ProjectStats` partagée
- [x] 4. TS `stores/project.ts` — ajouter `activeProjectStats`, fetch après open, toast si `!wasRestored`
- [x] 5. TS `useAppHandlers.ts` — supprimer interface locale, importer depuis `lib/types`
- [x] 6. TS `AppToolbar.tsx` — afficher `37%` dans la pill projet
- [x] 7. TS `ProjectList.tsx` — fetch stats par carte, afficher barre + compteurs
- [x] 8. i18n `en.json` + `fr.json` — ajouter clé `project.extracted`
- [x] 9. Vérification : `cargo clippy` ✓ · `pnpm typecheck` ✓

## En cours — Tokenizer Groupe G + \# (2026-06-26)

- [x] 1. `tokenizer.rs` — ajouter Groupe G (`\\n<[^>]+>`) dans RE_MVMZ et RE_MZONLY
- [x] 2. `tokenizer.rs` — ajouter `\#` (échappé) dans Groupe B des deux regex
- [x] 3. `tokenizer.rs` — 4 tests : tokenize `\n<Name>`, tokenize `\#`, round-trip, pas de conflit avec `\n[N]`
- [x] 4. Vérification : 27 tests ✓ · clippy ✓

## Complété — Export ZIP (2026-06-26)

- [x] 1. `Cargo.toml` — ajouter crate `zip = "2"`
- [x] 2. `engines/mv_mz/injector.rs` — ajouter `inject_to_bytes(raw_json, pairs) -> Vec<u8>`
- [x] 3. `engines/wolf/injector.rs` — ajouter `inject_all_to_memory(...) -> Vec<(String, Vec<u8>)>`
- [x] 4. `commands/export.rs` — refactorer `export_project` → zip `hoshi2star.zip` + `collect_wolf_zip_entries` + `write_zip`
- [x] 5. `commands/export.rs` — return type `Result<String, String>` (retourne le chemin zip)
- [x] 6. `useAppHandlers.ts` — `invoke<string>`, toast description = chemin zip
- [x] 7. i18n `en.json` + `fr.json` — messages mis à jour (ZIP / hoshi2star.zip)
- [x] 8. Vérification : clippy ✓ · typecheck ✓ · 336 tests ✓ (2 échecs pré-existants Inko)

## Complété — Font size MV/MZ (\\FS[N]) (2026-06-27)

`\FS[N]` MZ-natif (Groupe C). `\f[N]` Wolf-only. Dialog gate wolf/mv_mz.
Hint affiché pour mv_mz : nécessite MZ ou plugin messages pour MV.
clippy ✓ · typecheck ✓ · 336 tests ✓ (2 Inko pré-existants)

- [x] 1. `export.rs` — `RE_FONT_PREFIX_MVMZ` + `engine: String` dans `FontScanResult`
- [x] 2. `export.rs` — `apply_font_prefix(text, n, replace, engine)` engine-aware
- [x] 3. `export.rs` — `scan_font_status` fetch engine depuis DB, retourne engine
- [x] 4. `export.rs` — `persist_font_size(…, engine)` passe engine à apply
- [x] 5. `export.rs` — `export_project` fetch engine avec game_path
- [x] 6. `export.rs` — `debug_inject_file` utilise engine (plus `_engine`)
- [x] 7. `lib/types.ts` — `engine: string` dans `FontScanResult`
- [x] 8. `useAppHandlers.ts` — gate wolf/mv_mz uniquement
- [x] 9. `FontSizeDialog.tsx` — `code` calculé, hint mv_mz
- [x] 10. `en.json` + `fr.json` — `{{code}}` interpolé + clé `hintMvMz`
- [x] 11. Vérification ✓

## En cours — Externalisation des prompts LLM vers TOML

Objectif : sortir les prompts hardcodés de `provider.rs` et `glossary.rs` vers des
fichiers `.toml` embarqués à la compilation via `include_str!()`. Structure dossier
dès maintenant pour accueillir les langues cibles futures sans refactor.

**Architecture :**
```
src-tauri/prompts/
  translate/
    default.toml    ← fallback générique (ja→en aujourd'hui)
  glossary/
    default.toml    ← fallback générique
```
Quand on ajoutera FR : créer `translate/fr.toml` + bras `"fr"` dans le `match`.

**Variables dans les templates :**
- `translate/default.toml` : `{{source_lang}}` / `{{target_lang}}` / `{{glossary}}` / `{{segments}}`
- `glossary/default.toml` : `{{target_lang}}` / `{{source_list}}`
- Les valeurs sont des noms complets (`"Japanese"`, `"English"`) — pas des codes courts (`"ja"`, `"en"`)
  → `translate.rs` conserve `"ja"`/`"en"` en interne ; `prompts.rs` expose `lang_code_to_name()`

**Fichiers créés :**
- `src-tauri/prompts/translate/default.toml`
- `src-tauri/prompts/glossary/default.toml`
- `src-tauri/src/llm/prompts.rs`

**Fichiers modifiés :**
- `src-tauri/Cargo.toml` — ajouter `toml = "0.8"`
- `src-tauri/src/llm/mod.rs` — `pub mod prompts`
- `src-tauri/src/llm/provider.rs` — remplacer `format!()` hardcodé
- `src-tauri/src/core/glossary.rs` — remplacer strings hardcodées

- [x] 1. `Cargo.toml` — ajouter dépendance `toml = { version = "0.8", features = ["parse"] }`
- [x] 2. Créer `src-tauri/prompts/translate/default.toml` — system + user avec variables ci-dessus
- [x] 3. Créer `src-tauri/prompts/glossary/default.toml` — system + user avec variables ci-dessus
- [x] 4. Créer `src-tauri/src/llm/prompts.rs` :
         · `PromptTemplate { system, user }` + `render(part, vars)`
         · `fn lang_code_to_name(code: &str) -> &str` (`"ja"` → `"Japanese"`, `"en"` → `"English"`, …)
         · `LazyLock` `TRANSLATE_DEFAULT` + `GLOSSARY_DEFAULT`
         · `fn translate_for(target_lang: &str) -> &'static PromptTemplate` (fallback default)
         · `fn glossary_for(target_lang: &str) -> &'static PromptTemplate` (idem)
- [x] 5. `src-tauri/src/llm/mod.rs` — ajouter `pub mod prompts`
- [x] 6. `src-tauri/src/llm/provider.rs` — appeler `prompts::translate_for(&context.target_lang)`,
         passer `lang_code_to_name(source_lang)` et `lang_code_to_name(target_lang)` au `render()`
- [x] 7. `src-tauri/src/core/glossary.rs` — appeler `prompts::glossary_for(lang_target)`,
         passer `lang_code_to_name(lang_target)` au `render()`
- [x] 8. Vérification : clippy ✓ · 343/349 tests ✓ (2 échecs Inko pré-existants, 4 ignored)

## Backlog

- [ ] 🐛 TM : doublons à chaque re-sauvegarde (`tm.rs::insert` REPLACE inopérant,
      index non-UNIQUE) — découvert Phase 3, cf. section Phase 3
- [ ] 🐛 `pnpm lint` cassé : migrer `.eslintrc.*` → `eslint.config.js` (ESLint 10)
- [ ] QA live : câbler les termes glossaire dans `qa_check_segment` +
      `update_segment` (aujourd'hui `&[]` — le rapport les applique depuis Phase 4)
- [ ] Anneaux de progression par fichier dans FileTree (FileTree rings) — `translated_count`/
      `total_count` maintenant disponibles; rend la tâche dormante Tenmon réalisable
- [ ] Documentation workflow WolfX (pré-étape UberWolf) dans `docs/engines.md` +
      message UI quand `PossibleWolfX` est détecté (ROADMAP F5)
- [ ] Recrutement beta testeurs (Discord fan-trad / F95zone) — ROADMAP F3
- [ ] Diff-aware merge (`core/diff.rs`) — ROADMAP F4
