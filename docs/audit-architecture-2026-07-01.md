# Audit architecture & qualité — Hoshi2Star

> Revue en lecture seule, 2026-07-01. Aucune modification de code effectuée.
> Chaque conclusion est justifiée par `fichier:ligne`.

## Périmètre analysé

**Line-read (analyse ligne à ligne)** :
`lib.rs`, `state.rs`, `domain/types.rs`, `engines/mod.rs`, `engines/detector.rs`,
`llm/pipeline.rs`, `llm/provider.rs`, `llm/tokenizer.rs`, `core/tm.rs`,
`commands/project.rs`, `commands/export.rs`, `commands/translate.rs`,
`src/App.tsx`, `src/hooks/useAppHandlers.ts`, `src/stores/editor.ts`, `src/stores/llm.ts`.
Signatures/types vérifiés pour les 3 extractors + `WolfSegmentKind`.

**Sondé par métriques uniquement (pas ligne à ligne)** — conclusions marquées comme telles :
`engines/wolf/decrypt/legacy_xor.rs` (2056), `wolf/extractor.rs` (1712),
`wolf/injector.rs` (1031), `wolf/v3_format/*`, `mv_mz/extractor.rs` (936),
`vx_ace/extractor.rs` (809), `core/qa.rs` (603), `core/report.rs` (638),
`core/glossary.rs` (560), et la majorité des composants UI (`SegmentGrid`,
`GlossaryPanel`, etc.).

Total : ~18 750 LOC Rust (55 fichiers), ~5 435 LOC TS/TSX (36 fichiers), 197 commits.

---

## Résumé exécutif

Hoshi2Star est un **éditeur CAT + orchestrateur LLM** desktop (Tauri v2 / React 19 /
Rust) pour la traduction fan de RPG japonais. Contrairement à ce que dit `CLAUDE.md`
(« skeleton/template state »), le projet est **mature et fonctionnel** : pipeline LLM
complet avec retry/split adaptatif, TM SQLite (exact + fuzzy), QA, glossaire,
3 moteurs de jeu supportés (MV/MZ, Wolf v1/v2/v3, VX Ace prêt mais désactivé),
export ZIP, 349 tests Rust.

La **qualité du code lu est élevée** : documentation de module systématique,
nommage clair, gestion d'erreurs disciplinée aux frontières (`Result<T, String>`
+ `thiserror` interne), tests unitaires exhaustifs et pédagogiques (le tokenizer
et la TM sont exemplaires). La discipline de dev est réelle (`tasks/lessons.md`,
ADRs, CONTEXT.md).

Les faiblesses sont principalement de la **dette de maintenabilité** : duplication
dans la couche `commands/` (boucles d'insertion, filtre glossaire, recalcul de
stats), dispatch par moteur en `match`/`starts_with` plutôt que par abstraction,
et un `lang_pair`/`target_lang` **hardcodé `"ja-en"`** partout (déjà identifié
comme dette F4 dans la ROADMAP). Aucun test frontend. Niveau de maturité :
**production-ready sur le cœur, avec dette localisée dans les commandes et l'UI**.

### Notes /10

| Axe | Note | Confiance | Justification courte |
|-----|------|-----------|----------------------|
| Architecture | 8 | Élevée | 5 couches nettes, frontière IPC stricte ; mais dispatch moteur non abstrait |
| Lisibilité | 9 | Élevée | Doc-comments partout, headers de section, nommage domaine cohérent |
| Modularité | 8 | Moyenne | Bonne séparation ; couche engines sans trait unifiant, `open_project` god-fn |
| Maintenabilité | 8 | Moyenne | 349 tests + lessons.md ; freinée par duplication + hardcode ja-en |
| Performance | 7 | Moyenne | Suffisante à l'échelle actuelle, mais UPDATE par ligne, lookup TM par segment |
| Réutilisabilité | 7 | Moyenne | Fonctions pures excellentes (tokenizer/tm/detector) ; commands peu réutilisables |
| Respect DRY | 6 | Élevée | Duplication franche dans `commands/` (3 sites de recalcul stats, filtre glossaire ×2) |
| Respect SOLID | 7 | Moyenne | SRP bon, DIP excellent (trait `LlmProvider` + pipeline générique) ; OCP violé côté moteurs |
| Qualité globale | 8 | Moyenne | Cœur solide, dette périphérique bien circonscrite |

---

## 1. Architecture globale

**Découpage 5 couches** (conforme à CONTEXT.md) :

```
UI React (src/)  →  IPC invoke()  →  commands/  →  { llm/, core/, engines/ }  →  db/
```

- `lib.rs:52` — un **unique** `generate_handler![...]` (26 commandes) — règle respectée.
- `state.rs:7` — `AppState { db: SqlitePool }` injecté via `.setup()` (`lib.rs:49`).
- `domain/types.rs:1` — types IPC extraits des commands pour casser la dépendance
  cyclique (bon découplage, permet à `sync`/`export`/F5 de dépendre des types sans
  dépendre de `commands/`).
- **Frontière Rust↔TS étanche** : tout passe par `invoke()`, aucun accès DB depuis JS.
  Events préfixés `h2s://` (`pipeline.rs:224`, `llm.ts:85`). Règle respectée.

**Points d'attention architecturaux :**

1. **Dispatch par moteur non abstrait (OCP).** `engines/mod.rs` ne déclare que des
   sous-modules — pas de `trait Engine`. Chaque opération multi-moteur est un
   `match engine { MvMz | VxAce | Wolf }` répété : `open_project` (`project.rs:127`),
   `debug_dump_segments` (`project.rs:574`), `export_project` (`export.rs:228`),
   `debug_inject_file` (`export.rs:490`). Ajouter Bakin (F5) impose de toucher ≥4
   sites. Voir §7/§8.

2. **Typage « stringly-typed ».** L'engine et le `file_type` circulent en `String`
   avec dispatch par `starts_with("wolf_")` / `starts_with("vx_")`
   (`export.rs:228,271,311,490,513`). Un `enum` éliminerait les erreurs de frappe
   silencieuses.

3. **`open_project` fait trop.** `project.rs:33-283` : restauration manifest +
   détection + localisation data-dir + lecture titre + transaction + 3 branches
   d'extraction/insertion. ~250 lignes, responsabilités mêlées.

---

## 2. Structure du code

- **`commands/`** — 5 fichiers, un par domaine (project/translate/export/qa/glossary).
  Toutes `async`, `Result<T, String>`, `tauri::State`. Cohérent.
- **`core/`** — logique métier pure et testable : `tm.rs` (hash SHA-256, Levenshtein
  2-lignes O(m), TMX), `qa.rs`, `glossary.rs`, `manifest.rs`, `report.rs`. Excellente
  séparation UI-agnostique.
- **`llm/`** — `provider.rs` (trait + Ollama), `pipeline.rs` (orchestration TM→LLM),
  `tokenizer.rs` (ADR-002), `split.rs` (split récursif), `batch.rs`, `prompts.rs`
  (templates TOML externalisés). Découpage fin et propre.
- **`engines/`** — un dossier par moteur + `detector.rs`. Wolf est le plus lourd
  (decrypt DXA, v3_format porté byte-à-byte de WolfTL — cf. `lessons.md`).
- **Frontend** — `components/editor/` (panneaux CAT), `stores/` (4 slices Zustand
  typées), `hooks/useAppHandlers.ts` (orchestration dialogues), `lib/` (utils, i18n,
  types). `features/editor/columns.tsx` sépare les colonnes TanStack (convention
  respectée).

---

## 3. Flux de fonctionnement

1. **Démarrage** : `main.rs` → `run()` (`lib.rs:31`, ADR-005) → `.setup()` crée la DB
   dans `app_data_dir`, exécute les migrations (`db::pool::init`), `app.manage(AppState)`.
   Frontend : `App.tsx` monte les panneaux resizable, `loadSettings()` au mount.
2. **Ouvrir un projet** : UI → `open_project(path)` → manifest check → `detect_engine`
   (`detector.rs:37`) → extraction → INSERT transactionnel projet/fichiers/segments →
   écriture manifest best-effort.
3. **Traduire** : `llm.ts::startTranslation` enregistre les listeners `h2s://llm/*` →
   `invoke("translate_segments")` → `tokio::spawn` (non-bloquant) → `health_check` →
   charge glossaire → `pipeline::run` → par batch : dedup hash → `tm::lookup_exact` →
   tokenize → `llm_translate_with_split` → validate/restore placeholders → persist
   incrémental → emit `progress`/`segments-updated`/`placeholder-warning`.
4. **Exporter** : `handleExportAll` → `get_project_stats` (gate untranslated) →
   `scan_font_status` → `export_project` → injection par moteur → `hoshi2star.zip`
   à la racine du jeu.

Réseau : uniquement `reqwest` vers Ollama (`/api/chat`, `/api/tags`), retry ×3.

---

## 4. Qualité du code

**Forts** : doc-comments de module systématiques avec diagrammes ASCII
(`tokenizer.rs:1-16`, `pipeline.rs:1-21`) ; nommage domaine strict (Segment,
SourceFile, lang_pair) ; sections `// ---` cohérentes ; commentaires qui expliquent
le *pourquoi* (ordre des groupes regex `tokenizer.rs:31-36`, `⏎` marker
`provider.rs:172`). Gestion d'erreurs disciplinée : `map_err(|e| e.to_string())` aux
frontières, `thiserror` en interne, best-effort explicite (`let _ =` + `log::warn!`
sur inserts non critiques `pipeline.rs:147`).

**Faibles** :
- Duplication (voir §7).
- Chaînes UTF hardcodées `"ja-en"` / `"ja"` / `"en"` (translate.rs:86,137-138,265,
  325-326 ; useAppHandlers.ts:69,101 ; App.tsx:118). 43 occurrences repo-wide.
- Incohérence de valeur par défaut : modèle Rust `qwen3:4b-instruct-2507-q4_K_M`
  (`provider.rs:83`) vs frontend `qwen3:4b` (`llm.ts:50`).
- Convention CONTEXT.md non tenue : `H2sError` prescrit mais **0 occurrence** — tout
  est `Result<T, String>`. Ce n'est pas grave en soi (le pattern est cohérent), mais
  la doc et le code divergent.

---

## 5. Fonctions et méthodes

- `pipeline::run_inner` (`pipeline.rs:171`) — cœur de l'orchestration, 8 arguments
  (`#[allow(clippy::too_many_arguments)]`). Long mais cohérent ; le `global_progress:
  Option<(usize,usize)>` mériterait un type nommé.
- `translate_batch` (`pipeline.rs:303`) — dedup → TM → LLM. Correct ; le lookup TM est
  fait **par segment unique** en boucle (§11).
- `open_project` (`project.rs:33`) — god-function, à découper (§10).
- `translate_segments` vs `translate_all_segments` (`translate.rs:24` / `:207`) — le
  bloc de chargement/filtrage glossaire (`:89-134` vs `:304-322`) est **dupliqué quasi
  mot pour mot**.
- Fonctions pures exemplaires (courtes, testées, sans I/O) : `is_mv_mz_system`
  (`detector.rs:71`), `hash_source` (`tm.rs:43`), `parse_numbered_response`
  (`provider.rs:333`), tout `Tokenizer`.
- Pas de code mort notable côté lu ; VX Ace est volontairement désactivé mais conservé
  (`detector.rs:54-63`, décision documentée).

---

## 6. Patterns utilisés

- **Strategy / Dependency Inversion (DIP)** — `trait LlmProvider` (`provider.rs:52`),
  pipeline **générique** `P: LlmProvider` (`pipeline.rs:171`) testé avec `MockProvider`.
  Excellent : le meilleur usage de pattern du projet.
- **Repository (implicite)** — `core/tm.rs`, `core/glossary.rs` encapsulent l'accès
  SQL ; mais les commands font aussi du SQL inline (repository partiel).
- **Facade** — `commands/` expose une API IPC stable au frontend.
- **Observer** — events `h2s://*` Rust→TS, listeners Zustand (`llm.ts:74`).
- **State (machine)** — `status` segment (`untranslated`/`translated`/`needs_review`).
- **Flux Unidirectionnel (Zustand)** — slices typées + sélecteurs exportés
  (`editor.ts:50`), conforme CONTEXT.md.
- **Absents/partiels** : pas de Factory pour les moteurs (d'où le `match`), pas de
  vrai Repository côté commands.

---

## 7. DRY — duplications (par priorité)

| # | Prio | Localisation | Nature |
|---|------|--------------|--------|
| D1 | Haute | `project.rs:408-437`, `translate.rs:148-174`, `translate.rs:368-394` | **Recalcul stats manifest** : la même sous-requête corrélée (5 COUNT) est copiée 3× |
| D2 | Haute | `translate.rs:89-134` vs `:304-322` | **Filtre glossaire** (filter-by-content + fallback 10 plus courts) dupliqué entre les 2 commandes de traduction |
| D3 | Haute | `project.rs:128-161 / 163-196 / 198-247` | **Boucle INSERT source_files+segments** répétée dans les 3 arms de `open_project` |
| D4 | Moyenne | `export.rs:242-289` vs `debug_inject_file` `:490-535` | Logique d'injection par moteur dupliquée entre export projet et export fichier |
| D5 | Moyenne | `project.rs:851-879` (`classify_mv_mz_file`) vs `:882-908` (`dispatch_extract`) | Le test « est-ce un fichier Map ? » (`starts_with("Map") && parse::<u32>`) réécrit 3× |
| D6 | Basse | `translate.rs:75-82` vs `:256-263` | Bloc `health_check` + message d'erreur Ollama identique |

**Mutualisation proposée** (sans réécrire ici) :
- D1 → `manifest::refresh_stats(db, project_id)` unique.
- D2 → `glossary::relevant_terms(db, project_id, lang_pair, &pairs)`.
- D3 → normaliser chaque moteur vers `Vec<(file_name, file_type, file_path,
  Vec<(key, source)>)>` puis **une** boucle d'insertion.
- D5 → `fn is_map_file(name, ext)` partagé.

---

## 8. SOLID

- **S (SRP)** — Globalement bon (core/ très SRP). Violations : `open_project`
  (extraction + persistance + manifest), et `update_segment` qui fait QA + UPDATE +
  TM insert + recalcul stats (`project.rs:368-448`).
- **O (OCP)** — **Violé** pour l'ajout de moteur : `match engine`/`starts_with`
  dispersés (§1). Ajouter Bakin = éditer ≥4 fichiers. *Impact* : coût et risque de
  régression à chaque moteur. *Amélioration* : `trait EngineAdapter` OU, plus réaliste
  vu l'asymétrie des entrées (JSON `Value` / bytes / `game_dir+WolfVersion`) et des
  sorties (`ExtractedSegment.source` + `SegmentKind` vs `WolfSegment.source_text` +
  `WolfSegmentKind`), un **enum `FileType` typé** + normalisation vers une forme
  commune `(key, source)` avant persistance.
- **L (Liskov)** — N/A significatif (peu d'héritage/trait objects).
- **I (ISP)** — `LlmProvider` a 3 méthodes (`translate`/`health_check`/`chat`) ;
  `chat` sert la seule extraction glossaire. Acceptable, à surveiller.
- **D (DIP)** — **Excellent** : pipeline dépend de l'abstraction `LlmProvider`, pas
  d'`OllamaProvider`. Testabilité maximale.

---

## 9. Bonnes pratiques

- **Sécurité** : `get_segments` vérifie l'appartenance fichier↔projet
  (`project.rs:319-330`). `escape_xml` sur export TMX (`tm.rs:249`). `reqwest` en
  `rustls-tls` (pas d'OpenSSL — fix documenté v0.4.1). Pas de secret en dur.
- **Gestion d'erreurs** : cohérente, best-effort explicite sur les opérations non
  critiques. Pas de `.unwrap()` dans les chemins de production lus.
- **Validation** : placeholders validés post-LLM avec retry+fallback `needs_review`
  (`pipeline.rs`, tests `:547-675`). Gate export sur `untranslatedCount`.
- **Logging** : `log::warn!` sur best-effort ; pas de niveau `info`/`debug`
  structuré — acceptable pour une app desktop.
- **Robustesse binaire** : parsers Wolf (`legacy_xor.rs`) contiennent ~16 `.unwrap()`
  sur `try_into()` de slices (sondé, non line-read). La plupart sont gardés par des
  checks de longueur, mais certains offsets calculés (`legacy_xor.rs:352,366,976,
  1052`) peuvent paniquer le thread de commande sur une archive malformée. À
  transformer en `Result` (§ Faiblesses – Important).
- **Async** : tâches LLM en `tokio::spawn` — le thread IPC n'est jamais bloqué
  (`translate.rs:68,249`). Conforme CONTEXT.md.

---

## 10. Refactoring — cibles

- Fonctions trop longues : `open_project`, `debug_dump_segments`, les 2 `translate_*`.
- Responsabilités mêlées : `update_segment`, `open_project`.
- Dépendances/dispatch : centraliser la logique par-moteur.
- Code mort : aucun significatif (VX Ace désactivé volontairement, documenté).
- `src-tauri/tests/` **existe mais est vide** — les 349 tests sont tous inline. La
  convention CONTEXT.md « intégration dans tests/ » n'est pas honorée.

---

## 11. Performance

*(échelle actuelle OK ; à revoir si projets volumineux)*

- `persist_batch_results` (`pipeline.rs:132`) — `UPDATE` **ligne par ligne** en boucle,
  hors transaction. Un batch de 100 segments = 100 aller-retours SQLite.
- `translate_batch` (`pipeline.rs:322-334`) — `tm::lookup_exact` **par segment unique**
  (1 requête chacun) au lieu d'un `WHERE source_hash IN (...)`.
- `update_segment` (`project.rs:408`) — sous-requête corrélée lourde (5 COUNT sur tous
  les segments du projet) à **chaque sauvegarde manuelle**.
- `lookup_fuzzy` (`tm.rs:149`) — scan O(n) en mémoire de toute la TM. **Auto-documenté**
  comme acceptable ~5k entrées (trigram index en backlog F5). Pas un défaut, à surveiller.
- Extraction : INSERT segments un par un dans la transaction `open_project` — un
  `INSERT` multi-valeurs réduirait le temps d'ouverture sur gros jeux.

---

## 12. Évolutivité

**Forces** : abstraction LLM générique (ajouter OpenAI/DeepSeek = un `impl
LlmProvider`) ; TM globale cross-projet (différenciateur) ; prompts externalisés TOML
(terrain préparé pour langues cibles F4) ; types IPC isolés dans `domain/`.

**Limites/risques** :
- Ajout de moteur coûteux tant que le dispatch n'est pas centralisé (OCP).
- `lang_pair` hardcodé `"ja-en"` bloque les langues cibles multiples — **déjà planifié
  F4** (ROADMAP:76-88), donc dette connue et cadrée.
- Aucun test frontend + `src-tauri/tests/` vide → régressions UI/intégration non
  couvertes.
- `provider.rs` couple le protocole « réponse numérotée `[n] texte` » au parsing ;
  un provider structuré (JSON mode) demanderait une refonte du parser.

---

## 13. Points positifs

- Frontière Rust↔TS étanche, un seul `generate_handler!`, events `h2s://` — conventions
  respectées à la lettre.
- 349 tests Rust, pédagogiques et couvrant les cas limites (placeholders,
  multi-lignes, split adaptatif, fuzzy).
- Doc-comments de niveau professionnel + ADRs + `lessons.md` (self-improvement réel).
- DIP/Strategy sur le LLM = testabilité exemplaire (`MockProvider`).
- Fonctions pures bien isolées (tokenizer, tm, detector).
- Persistance incrémentale par batch (un crash ne perd que le batch en vol).
- Sécurité correcte (scoping requêtes, escape XML, rustls, pas de secrets).
- Gestion best-effort explicite et cohérente (manifest/TM ne cassent jamais l'action).

---

## Faiblesses classées par gravité

**Critique** : *aucune.* Le cœur est sain.

**Important**
1. `.unwrap()` sur parsing binaire non totalement gardé (`legacy_xor.rs` ~16
   occurrences, offsets calculés) → panique possible du thread commande sur archive
   Wolf malformée. → convertir en `Result`.
2. Duplication D1/D2/D3 (recalcul stats ×3, filtre glossaire ×2, boucle insert ×3) →
   dette de maintenance active, source de bugs divergents.
3. Violation OCP du dispatch moteur → coût/risque à chaque nouveau moteur (Bakin F5).
4. Aucun test frontend + `src-tauri/tests/` vide malgré l'intention affichée
   (Vitest/webapp-testing dans CONTEXT.md, package.json sans script `test`).

**Mineur**
5. `lang_pair`/`target_lang` hardcodé `"ja-en"` (dette **déjà cadrée F4**).
6. Modèle par défaut incohérent Rust (`…q4_K_M`) vs TS (`qwen3:4b`).
7. `H2sError` prescrit par CONTEXT.md mais inexistant (doc ≠ code).
8. **Incohérence de comptage « traduit »** : `get_source_files` compte
   `target_text != ''` (`project.rs:295`) alors que `get_project_stats` compte
   `status = 'translated'` (`project.rs:471`). Les segments `needs_review` ont un
   `target_text` mais un statut ≠ `translated` → la progression par fichier (FileTree)
   et les stats projet **divergent**. À vérifier/aligner.
   → **Confirmé sur le schéma** (4 statuts, stats en comptent 3) et **tranché
   (2026-07-01) : Option 2** — exposer `needs_review_count` par fichier +
   `reviewed_count` projet. Détails : `audit-architecture-2026-07-01-complement.md` §6.
9. `CLAUDE.md` décrit le projet comme « skeleton » alors qu'il est mature (doc obsolète).

---

## Plan d'amélioration (phases indépendantes)

### Phase A — Alignement correctness & doc (faible risque)
- **Objectif** : supprimer les incohérences silencieuses.
- **Fichiers** : `project.rs` (aligner le critère « traduit » entre `get_source_files`
  et `get_project_stats`), `llm.ts` / `provider.rs` (aligner le modèle par défaut),
  `CLAUDE.md`/CONTEXT.md (statut réel, `H2sError`).
- **Bénéfices** : progression cohérente pour l'utilisateur, doc fiable.
- **Risques** : quasi nuls ; un test pour figer la sémantique de `translated_count`.

### Phase B — DRY des commandes (risque moyen, fort ROI maintenabilité)
- **Objectif** : éliminer D1/D2/D6.
- **Fichiers** : `core/manifest.rs` (`refresh_stats`), `core/glossary.rs`
  (`relevant_terms`), `commands/translate.rs`, `commands/project.rs`.
- **Bénéfices** : ~120 lignes en moins, un seul point de vérité pour stats/glossaire.
- **Risques** : régression sur le calcul stats → couvrir par tests avant extraction.

### Phase C — Robustesse parsers binaires (risque moyen)
- **Objectif** : plus aucune panique sur entrée malformée.
- **Fichiers** : `engines/wolf/decrypt/legacy_xor.rs`, `wolf/v3_format/*`,
  `wolf/dat_parser.rs`.
- **Bénéfices** : erreurs propres remontées à l'UI au lieu de crash silencieux.
- **Risques** : churn sur du code binaire délicat → s'appuyer sur les round-trips
  byte-exacts existants comme garde-fou.

### Phase D — Centraliser le dispatch moteur (risque moyen, prépare F5/Bakin)
- **Objectif** : lever la violation OCP.
- **Fichiers** : `engines/mod.rs` (enum `FileType` + normalisation vers `(key,
  source)`), `commands/project.rs` (`open_project`, `debug_dump_segments`),
  `commands/export.rs`.
- **Bénéfices** : ajouter Bakin = 1 module + 1 arm, plus un dispatch éparpillé.
- **Risques** : refonte de `open_project` → gros diff ; découper d'abord `open_project`
  en helpers avant d'introduire l'abstraction.

### Phase E — Couverture de tests frontend/intégration (faible risque)
- **Objectif** : combler le trou de tests UI + intégration IPC.
- **Fichiers** : ajouter Vitest + Testing Library (stores, `useAppHandlers`), peupler
  `src-tauri/tests/` (open→translate→export end-to-end sur fixtures).
- **Bénéfices** : filet de sécurité pour les refactos B/C/D.
- **Risques** : nuls ; à faire **avant** B/C/D idéalement.

### Phase F — Performance (à déclencher sur signal, pas prématuré)
- **Objectif** : batch UPDATE + lookup TM groupé.
- **Fichiers** : `llm/pipeline.rs` (`persist_batch_results` en transaction,
  `lookup_exact` → `IN (...)`), `commands/project.rs` (INSERT multi-valeurs).
- **Bénéfices** : temps d'ouverture et de traduction réduits sur gros projets.
- **Risques** : faibles ; ne pas optimiser `lookup_fuzzy` avant d'atteindre l'échelle
  documentée (~5k).

**Ordre recommandé** : A → E → B → C → D → F (E avant B/C/D pour sécuriser les
refactos).

---

## Informations manquantes / non vérifiées

- `core/qa.rs`, `core/report.rs`, `core/glossary.rs` : notés/scorés à partir de leurs
  usages (`update_segment`, `export_qa_report`) et signatures, **pas line-read**.
- Internes de `legacy_xor.rs` / `wolf/extractor.rs` / `injector.rs` : sondés par
  métriques (LOC, grep `.unwrap()`), pas ligne à ligne.
- Composants UI (`SegmentGrid`, `GlossaryPanel`, `QAPanel`…) : non line-read → les
  jugements UI portent sur `App.tsx`, `useAppHandlers.ts`, les stores.
- Migrations SQL (`src-tauri/migrations/`) non ouvertes → le schéma est inféré des
  requêtes ; l'incohérence #8 devrait être confirmée sur le schéma réel.
