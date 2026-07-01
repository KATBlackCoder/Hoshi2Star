# Plan de remédiation — audit 2026-07-01

> Plan d'exécution **étape par étape** dérivé de `audit-architecture-2026-07-01.md`
> et de son complément `audit-architecture-2026-07-01-complement.md`. Chaque phase est
> **indépendante et livrable seule**, ordonnée par gravité réelle (perte de données →
> crash → robustesse → correctness → cohérence → dette → tests → structure → perf).
>
> **Gate de vérification obligatoire à la fin de CHAQUE phase** (CLAUDE.md) :
> ```
> pnpm typecheck && \
> cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings && \
> cargo test --manifest-path src-tauri/Cargo.toml
> ```
> Aucune phase n'est « terminée » sans preuve que ça marche (test qui échoue avant le
> fix, passe après).

## Vue d'ensemble

| # | Phase | Prio | Gravité | Dépend de | Risque |
|---|---|---|---|---|---|
| 1 | Réinjection Common Events Wolf | P0 | 🔴 Perte de données | — | Moyen |
| 2 | Paniques parsers binaires Wolf | P1 | 🟠 Crash | — | Moyen |
| 3 | Robustesse promesses UI | P1 | 🟠 Silencieux | — | Faible |
| 4 | Correctness QA (largeur + rapport) | P2 | 🟠 Faux résultats | — | Faible |
| 5 | Cohérence #8 (Option 2) + doc | P2 | 🟡 Trompeur | — | Faible |
| 6 | DRY backend + UI | P3 | 🟡 Dette | 5, (idéal. 7) | Moyen |
| 7 | Couverture de tests front/intégration | P3 | 🟡 Filet | — | Faible |
| 8 | Centraliser le dispatch moteur (OCP) | P4 | 🟡 Prépare Bakin | 7 | Moyen |
| 9 | Performance (batch/lookup) | P4 | ⚪ Sur signal | 7 | Faible |

**Ordre recommandé** : 1 → 2 → 3 → 4 → 5 → 7 → 6 → 8 → 9.
(7 avant 6/8 pour sécuriser les refactos ; 5 avant 6 car #8 touche les mêmes fichiers.)

---

## Phase 1 — 🔴 Réinjecter les Common Events Wolf à l'export

**Pourquoi** : les traductions CE Wolf sont extraites, traduites, validées par la gate,
mais **jamais réécrites** dans le ZIP (`injector.rs:514` `_ => {}`, pas de
`inject_common_events`). Perte de données silencieuse sur le moteur du lancement public.

**Étapes**
1. [ ] **Verify d'abord (read-only)** : sur une fixture réelle (Inko/Honoka), extraire
   un projet Wolf, compter les segments `file_type = 'wolf_common_events'`, traduire (ou
   simuler), exporter, et **prouver** que le ZIP ne contient pas les traductions CE.
   Chiffrer l'impact. Écrire un test qui reproduit la perte (rouge).
2. [ ] `wolf/injector.rs` — écrire `inject_common_events(bytes, translations, version)`
   symétrique de `inject_map` : parser `CommonEvent.dat` (chemin v2) **et** v3/LZ4, ré-
   émettre les `ShowMessage`/`ShowChoice` traduits, redump byte-exact hors segments
   modifiés.
3. [ ] `wolf/injector.rs` — ajouter l'arm `"CommonEvents"` dans **`inject_all_to_memory`
   ET `inject_all`** (charger via un `load_common_event_bytes`-équivalent injecteur).
4. [ ] `commands/export.rs` — vérifier le regroupement `file_key` pour les CE
   (`:327-329` `splitn(3,'/')` → `CommonEvents/{event_name}`) et router correctement.
5. [ ] Test de non-régression : le round-trip existant
   `test_real_inko_common_events_v3_round_trip` (`v3_format/common_events.rs:601`) sert
   de filet ; ajouter un test « traduire 1 CE → présent dans le ZIP » (le rouge de
   l'étape 1 passe au vert).

**Fichiers** : `engines/wolf/injector.rs`, `commands/export.rs`, tests.
**Risque** : moyen (code binaire) — mitigé par les round-trips byte-exacts existants.
**Fallback si trop coûteux** : à défaut d'injecter, **exclure** les CE de l'extraction
pour ne pas mentir à l'utilisateur (moins bon, mais honnête). Décision à prendre si le
parser v3 CE côté injection s'avère plus lourd que prévu.

---

## Phase 2 — 🟠 Éliminer les paniques des parsers binaires Wolf

**Pourquoi** : 5 sites non gardés dans `legacy_xor.rs` crashent le thread de commande
sur un `.wolf` malformé (indépendant du profil de build).

**Étapes**
1. [ ] `legacy_xor.rs:731` (racine) — plafonner `orig_size` contre un maximum sain avant
   `vec![0u8; orig_size]` ; retourner une `Err` propre au lieu de paniquer.
2. [ ] `legacy_xor.rs:591` — vérifier `ns <= toc_data.len()` avant `toc_data[ns..]`.
3. [ ] `legacy_xor.rs:1214/1216/1267/1269` — borner `decoded.len() >= huff_kb` avant les
   slices Huffman (les deux blocs v8).
4. [ ] Tier arithmétique — passer les additions d'offsets en `checked_add` :
   `legacy_xor.rs:554,580,1387,541` et `v3_format/map.rs:355`
   (`width*height*layer_cnt*4`).
5. [ ] `dat_parser.rs:158/193/239/244` — borner les `vec![0u8; n]` / `with_capacity`
   contre un max avant allocation (éviter l'abort OOM).
6. [ ] Ajouter des tests « entrée malformée → `Err`, pas panic » pour chaque site.

**Fichiers** : `engines/wolf/decrypt/legacy_xor.rs`, `wolf/v3_format/map.rs`,
`wolf/dat_parser.rs`.
**Risque** : moyen (code délicat) — filet : round-trips byte-exacts (map/CE/dat/coder).
**Note** : ne PAS toucher `catch_unwind` (`extractor.rs:521,841`) — garde-fou volontaire.

---

## Phase 3 — 🟠 Robustesse des promesses UI

**Pourquoi** : plusieurs `await invoke(...)` sans gestion d'échec → erreurs silencieuses
ou stats qui disparaissent.

**Étapes**
1. [ ] `ProjectList.tsx:44-50` — envelopper le `Promise.all(map(...))` avec un `.catch`
   par projet (ou `Promise.allSettled`) pour qu'un `get_project_stats` en échec ne fasse
   pas tomber toutes les stats.
2. [ ] `SegmentGrid.tsx:230-244` — `handleSave` : try/catch autour de `update_segment` +
   toast d'erreur, ne pas laisser le rejet non géré.
3. [ ] `SegmentGrid.tsx:264-270,108-112` — réinitialiser `rowSelection` sur changement de
   `qaFilter`/`searchQuery` (pas seulement au changement de fichier) pour éviter de
   traduire les mauvais segments après filtrage.
4. [ ] `SegmentGrid.tsx:68-72` — traiter le **plafond 5000 lignes** : soit paginer
   réellement, soit charger en pages successives et afficher le vrai total (footer
   `:505,509` et `statusCounts :150-159` doivent porter sur le total, pas le sous-
   ensemble). **Décision de scope** : pagination complète vs simple augmentation du
   plafond + total correct via une requête `COUNT`.
5. [ ] `QAPanel.tsx:123`, `TMPanel.tsx:73` — garder les `save()` de dialogue.

**Fichiers** : `components/editor/{ProjectList,SegmentGrid,QAPanel,TMPanel}.tsx`.
**Risque** : faible. **Bloquant** : aucun test front n'existe encore (cf. Phase 7) → à
vérifier manuellement d'ici là.

---

## Phase 4 — 🟠 Correctness du moteur QA

**Pourquoi** : mesures et statistiques QA fausses ou mortes.

**Étapes**
1. [ ] `qa.rs:134` — corriger `is_fullwidth` : `U+FF61..=U+FF9F` et `U+FFE8..=U+FFEE`
   valent **1** (katakana/ponctuation demi-largeur), pas 2. Ajouter un test kana
   demi-largeur.
2. [ ] `report.rs:231-232,312` — propager le **vrai** total « vérifié » (tous les
   segments traduits) depuis `collect_qa_details` au lieu de `details.len()`
   (aujourd'hui X == Y, dénominateur inutile).
3. [ ] `report.rs:71` — décider du glossaire dans le rapport : soit **passer les vrais
   termes** à `qa::check` (activer `GlossaryMismatch`), soit **retirer** la stat/filtre/
   pastille glossaire de l'UI du rapport (`:302,332,388`). Ne pas laisser mort.

**Fichiers** : `core/qa.rs`, `core/report.rs`.
**Risque** : faible (logique pure, bien testée).

---

## Phase 5 — 🟡 Cohérence #8 (Option 2) + alignements doc/valeurs

**Pourquoi** : `translatedCount` a 3 définitions → FileTree et Toolbar se contredisent.
**Décision retenue : Option 2** (exposer `needs_review` par fichier). Cf. complément §6.

**Étapes — #8 (Option 2)**
1. [ ] `commands/project.rs` — `get_source_files` (`:291-306`) : `translated_count` =
   `SUM(status='translated')` (au lieu de `target_text != ''`) **+** nouveau
   `needs_review_count = SUM(status='needs_review')`.
2. [ ] `commands/project.rs` — `get_project_stats` (`:456-489`) : ajouter
   `reviewed_count = SUM(status='reviewed')` pour que la somme = total.
3. [ ] `domain/types.rs` — ajouter `needs_review_count` à `SourceFile`, `reviewed_count`
   à `ProjectStats`.
4. [ ] `src/lib/types.ts` — miroir : `SourceFile.needsReviewCount`,
   `ProjectStats.reviewedCount`.
5. [ ] `FileTree.tsx:177-178` — `isComplete` = tout `translated` (plus de ✓ ni d'inject
   tant que `needsReviewCount > 0`) ; afficher `✓ N · ⚠ M`.
6. [ ] `ProjectList.tsx:245-260` — `SegmentStatsBar` rend aussi `reviewed` ; la barre
   somme à 100 %.
7. [ ] Test Rust figeant la sémantique des 3/4 compteurs (un projet avec 1 de chaque
   statut).

**Étapes — alignements annexes (Phase A initiale)**
8. [ ] Modèle par défaut : unifier les **3** valeurs
   (`provider.rs:83` `q4_K_M` · `settings.ts:23` `q8_0` · `llm.ts:50` `qwen3:4b`) sur
   une constante unique (décider laquelle ; probablement `q4_K_M` = référence backend).
9. [ ] Doc : `CONTEXT.md`/`CLAUDE.md` — soit implémenter `H2sError` (thiserror), soit
   acter que la convention est `Result<T, String>` (0 occurrence de `H2sError`
   aujourd'hui). Corriger aussi le statut « skeleton » (le projet est mature).

**Fichiers** : `commands/project.rs`, `domain/types.rs`, `src/lib/types.ts`,
`FileTree.tsx`, `ProjectList.tsx`, `llm/provider.rs`, `stores/{settings,llm}.ts`,
`CONTEXT.md`, `CLAUDE.md`.
**Risque** : faible ; couvrir le calcul stats par un test avant/après.

---

## Phase 6 — 🟡 DRY backend + UI

**Pourquoi** : duplication franche → bugs divergents et coût de maintenance.

**Étapes — backend**
1. [ ] `core/manifest.rs::refresh_stats(db, project_id)` unique → remplace le recalcul
   stats copié 3× (`project.rs:408-437`, `translate.rs:148-174,368-394`).
2. [ ] `core/glossary.rs::relevant_terms(...)` → remplace le filtre glossaire dupliqué
   (`translate.rs:89-134` vs `:304-322`).
3. [ ] `fn is_map_file(name, ext)` partagé → remplace le test « Map ? » écrit **4×**
   (`vx_ace/extractor.rs:435`, `project.rs:781,853,883`).
4. [ ] Table `QaError::kind()→(pénalité,label)` → collapse le `match` répété 5×
   (`qa.rs:257,289,314`, `report.rs:91,241`).
5. [ ] `core/glossary.rs` — const partagée pour le `SELECT` 9-colonnes (répété 3×,
   `:116,127,307`) ; corriger le **N+1** de dédup (`:235-242`) en pré-fetch unique.

**Étapes — UI**
6. [ ] Helper « ouvrir un jeu + mapper l'erreur moteur » → remplace la duplication ×3
   (`AppToolbar.tsx:161-181`, `ProjectList.tsx:53-66,88-107`).
7. [ ] Hook `useExportToFile(...)` → factorise `QAPanel`/`TMPanel` (save→invoke→toast).
8. [ ] Source unique de vérité statut→couleur (aujourd'hui éparpillée : `STATUS_STYLES`,
   `SegmentStatsBar`, `FileTree fileIcon`, `QAPanel`, `TMPanel MatchBadge`).
9. [ ] Dé-dupliquer le listener `h2s://glossary/extraction-done`
   (`SegmentGrid.tsx:211` + `GlossaryPanel.tsx:243`).

**Fichiers** : `core/{manifest,glossary}.rs`, `commands/{project,translate}.rs`,
`engines/**/extractor.rs`, plusieurs composants UI.
**Risque** : moyen — **faire après la Phase 7** (tests) idéalement.

---

## Phase 7 — 🟡 Couverture de tests front + intégration

**Pourquoi** : **aucun** test front (Vitest absent de `package.json`) et
`src-tauri/tests/` **vide** → aucune non-régression sur les fixes UI/IPC.

**Étapes**
1. [ ] Installer Vitest + Testing Library (`pnpm add -D`), ajouter le script `test`,
   config `vitest.config.ts` + jsdom.
2. [ ] Tests stores : `project.ts`, `llm.ts`, `settings.ts`, `editor.ts`.
3. [ ] Tests hooks/composants : `useAppHandlers`, `SegmentGrid` (filtre/sélection),
   `ProjectList` (échec stats), `FileTree` (✓/⚠ Option 2).
4. [ ] Peupler `src-tauri/tests/` : intégration bout-en-bout
   `open → translate → export` sur fixtures (au moins Wolf CE pour verrouiller Phase 1).

**Fichiers** : `package.json`, `vitest.config.ts`, `src/**/*.test.ts(x)`,
`src-tauri/tests/`.
**Risque** : faible ; **idéalement avant 6 et 8**.

---

## Phase 8 — 🟡 Centraliser le dispatch moteur (OCP)

**Pourquoi** : `match engine`/`starts_with("wolf_")` dispersé sur ≥4 sites → ajouter
Bakin (F5) coûte cher et risque des régressions.

**Étapes**
1. [ ] `engines/mod.rs` — enum `FileType` typé + normalisation vers une forme commune
   `(key, source)` avant persistance (remplace le stringly-typed
   `export.rs:228,271,311,490`).
2. [ ] Découper d'abord `open_project` (`project.rs:33-283`, god-fn) en helpers
   (restauration manifest / détection / extraction / insertion) **avant** d'introduire
   l'abstraction.
3. [ ] Router `open_project`, `debug_dump_segments`, `export_project`,
   `debug_inject_file` via le nouveau dispatch centralisé.

**Fichiers** : `engines/mod.rs`, `commands/project.rs`, `commands/export.rs`.
**Risque** : moyen (gros diff sur `open_project`) — **après** Phase 7.
**Bénéfice** : ajouter Bakin = 1 module + 1 arm.

---

## Phase 9 — ⚪ Performance (à déclencher sur signal, pas prématuré)

**Pourquoi** : suffisant à l'échelle actuelle ; à revoir sur gros projets.

**Étapes**
1. [ ] `pipeline.rs:132` `persist_batch_results` — `UPDATE` ligne par ligne → une
   transaction / `UPDATE` groupé.
2. [ ] `pipeline.rs:322-334` `translate_batch` — `tm::lookup_exact` par segment →
   `WHERE source_hash IN (...)`.
3. [ ] `project.rs:408` `update_segment` — la sous-requête corrélée 5×COUNT à chaque
   save → remplacée par `manifest::refresh_stats` (Phase 6).
4. [ ] Extraction — INSERT segments un par un → `INSERT` multi-valeurs (temps
   d'ouverture sur gros jeux).
5. [ ] **Ne PAS** optimiser `lookup_fuzzy` (`tm.rs:149`, scan O(n)) avant d'atteindre
   l'échelle documentée (~5k, trigram index en backlog F5).

**Fichiers** : `llm/pipeline.rs`, `commands/project.rs`, `core/tm.rs`.
**Risque** : faible.

---

## Suivi

Cocher les étapes au fil de l'eau. Chaque phase mergée séparément après passage du gate
de vérification. Reporter dans `CHANGELOG.md` (skill `update-changelog`) et actualiser
`ROADMAP.md` si une phase touche une échéance F4/F5.
