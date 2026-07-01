# Audit complémentaire — analyses détaillées (2026-07-01)

> Complément à `audit-architecture-2026-07-01.md`. Ce document **line-read** les
> zones que l'audit initial avait laissées « sondées par métriques » ou « non
> vérifiées » : `core/{qa,report,glossary}.rs`, les parsers binaires Wolf, les
> 3 extractors, les composants UI, et le schéma SQL. Revue en **lecture seule,
> aucune modification de code**. Chaque conclusion est justifiée par `fichier:ligne`.
>
> **Il corrige aussi l'audit initial là où le line-read le contredit** (§0).

---

## 0. Corrections à apporter à l'audit initial

| Point initial | Réalité après line-read |
|---|---|
| « **Critique : aucune. Le cœur est sain.** » | **Faux** : il existe **1 bug Critique de perte de données** — les traductions de **Common Events Wolf** ne sont jamais réinjectées à l'export (§1.1). Vérifié de bout en bout. |
| `.unwrap()` Wolf : sites nommés `legacy_xor.rs:352,366,976,1052` = risque de panique | Ces 4 sites sont en réalité **gardés** (bornes vérifiées avant, cf. §2). Les **vrais** sites non gardés sont **L591** et **L1214/1216/1267/1269**, tous enracinés dans une allocation non bornée **L731**. |
| Wolf `v3_format/*`, `injector.rs`, `dat_parser.rs` suspects (grep `.unwrap()`) | **Durcis** : tout passe par un `ByteReader`/`Cursor` borné + `checked_add`. Aucune panique de production. La crainte métrique **ne s'y applique pas**. |
| Incohérence #8 = 2 définitions de « traduit » | **Renforcée** : le schéma autorise **4 statuts** ; `get_project_stats` n'en compte que **3** (ignore `reviewed`). Détails et statut de `reviewed` en §6. |

**Impact sur les notes /10** : voir §8 (Qualité globale et « bonnes pratiques » baissent
à cause du bug Critique + des rejets de promesses UI non gérés ; le reste tient).

---

## 1. Bugs de correctness vérifiés (le signal neuf)

### 1.1 🔴 CRITIQUE — Traductions des Common Events Wolf silencieusement perdues à l'export

Chaîne complète vérifiée par line-read :

1. **Extraction & persistance** : `open_project` (arm Wolf) appelle
   `extract_all_wolf` (`project.rs:200`), qui **émet** des segments
   `file_type = "wolf_common_events"` (`extractor.rs:485`, clés
   `CommonEvents/{event_name}/{event_idx}/{cmd_idx}` en `extractor.rs:864,879`).
   Ces `source_files` + segments sont **insérés en DB** (`project.rs:202-224`).
2. **Traduction & comptage** : l'utilisateur les traduit normalement ; ils comptent
   dans `get_project_stats` (qui balaie tous les segments du projet) → **la gate
   d'export les considère « prêts »**.
3. **Export** : `export.rs:311` inclut tout `file_type` commençant par `"wolf_"` ;
   la clé est regroupée en `file_key = "CommonEvents/{event_name}"`
   (`export.rs:327-329`).
4. **Injection** : `inject_all_to_memory` (et `inject_all`) ne matchent que
   `"MapData"` et `"Database"` ; tout le reste tombe dans **`_ => {}`**
   (`injector.rs:514`, idem `inject_all` `:566` env.). **Il n'existe aucune fonction
   `inject_common_events`** (`injector.rs` n'expose que `inject_map` `:84` et
   `inject_dat` `:392`).

**Conséquence** : l'utilisateur traduit les Common Events, l'export « réussit », mais
ces traductions **sont absentes du `hoshi2star.zip`**. Silencieux, aucune erreur.
Wolf étant le moteur du **lancement public (F4)**, c'est le défaut le plus grave du
projet.

**Remède** : implémenter `inject_common_events` (parser `CommonEvent.dat` v2 + chemin
v3/LZ4, symétrique de `inject_map`/`inject_dat`) et ajouter l'arm `"CommonEvents"`
dans les deux dispatchs de `injector.rs`. À défaut immédiat : **exclure les
`wolf_common_events` de l'extraction** pour ne pas faire croire à l'utilisateur
qu'ils seront exportés (moins bon, mais honnête). Le round-trip byte-exact
`test_real_inko_common_events_v3_round_trip` (`v3_format/common_events.rs:601`)
fournit déjà un filet pour la réinjection.

### 1.2 🟠 Important — `qa.rs` : les katakana demi-largeur comptés en pleine largeur

`is_fullwidth` mappe **tout** le bloc `U+FF00..=U+FFEF` en largeur 2 (`qa.rs:134`),
alors que `U+FF61..=U+FF9F` (katakana/ponctuation **demi-largeur**) et
`U+FFE8..=U+FFEE` valent 1. Un texte JP utilisant des kana demi-largeur **sur-mesure
la longueur de ligne** → faux positifs `LineTooLong`. Aucun test ne couvre ce cas
(`qa.rs:345-603`).

### 1.3 🟠 Important — `report.rs` : dénominateur trompeur + check glossaire mort

- **« X segments avec erreurs / Y vérifiés » est toujours X == Y** :
  `total_checked = details.len()` alors que « seuls les segments en erreur sont
  passés » (`report.rs:231-232`, rendu `:312`). Le vrai total « vérifié » (tous les
  segments traduits) est jeté dans `collect_qa_details` et jamais propagé → la
  statistique n'a aucun sens.
- **Le check glossaire est mort dans le rapport** : `collect_qa_details` appelle
  `qa::check(..., &[], ...)` avec un glossaire **vide** (`report.rs:71`), donc
  `GlossaryMismatch` ne peut **jamais** apparaître — pourtant l'UI affiche une stat,
  un filtre et une pastille glossaire (`report.rs:302,332,388`). Feature à moitié
  câblée (le commentaire `report.rs:4` le reconnaît).

### 1.4 🟠 Important — `SegmentGrid.tsx` : trois défauts de robustesse

- **Plafond 5000 lignes en dur** : `loadSegments` fige `page:0, pageSize:5000`
  (`SegmentGrid.tsx:68-72`) malgré une API paginée. Un fichier > 5000 segments est
  **tronqué silencieusement**, et le footer affiche `segments.length` (≤5000) comme
  total (`:505,509`) ; `statusCounts` (`:150-159`) ne portent que sur le sous-ensemble.
- **Rejet non géré dans `handleSave`** : `await invoke("update_segment")` sans
  try/catch (`:230-244`) ; un échec de sauvegarde rejette silencieusement et la ligne
  n'est pas mise à jour.
- **`rowSelection` non réinitialisé sur changement de filtre** : la sélection persiste
  à travers `qaFilter`/`searchQuery` (réinitialisée seulement au changement de fichier,
  `:108-112`) et `selectedIds` mappe par index (`:264-270`) → **risque de traduire les
  mauvais segments** après filtrage.

### 1.5 🟠 Important — `ProjectList.tsx` : stats fragiles + `reviewed` invisible

- **`Promise.all(projects.map(... .then(...)))` sans `.catch`** (`:44-50`) : un seul
  `get_project_stats` en échec **rejette tout le lot** → aucune stat ne s'affiche.
- **`SegmentStatsBar` ignore `reviewed`** (`:245-260`) : ne rend que `translated`
  (vert) + `needs_review` (ambre) ; `ProjectStats` n'a pas de `reviewedCount`, donc la
  barre ne somme pas à 100 % dès qu'un segment est `reviewed` (cf. §6 pour la
  réalité : `reviewed` est aujourd'hui inatteignable → défaut **latent**).

---

## 2. Robustesse des parsers binaires Wolf — inventaire précis

**Verdict corrigé** : `v3_format/*`, `dat_parser.rs` et `injector.rs` sont **durcis**
(lecture via `ByteReader.take()` / `Cursor::read_exact` + `checked_add` ; les
`try_into().unwrap()` ne peuvent pas tirer, les slices ne peuvent pas sortir des
bornes). Les paniques réelles sont **toutes dans `legacy_xor.rs`** (décrypteur DXA/DARC
legacy).

`Cargo.toml` n'a **aucun** `[profile.release]` / `overflow-checks` → défauts Rust :
overflow-checks **ON en debug/test, OFF en release** (wrap silencieux).

**Sites de panique NON gardés (crashent quel que soit le profil — indexation OOB /
capacité `Vec`)** :

| # | `fichier:ligne` | Opération | Pourquoi non gardé |
|---|---|---|---|
| 1 | `legacy_xor.rs:591` | `toc_data[ns..]`, `ns = name_offset as usize` | `name_offset` lu brut du TOC attaquant (`:358/372`) ; la boucle `extract_all` (`:574-604`) ne vérifie jamais `ns <= toc_data.len()`. |
| 2 | `legacy_xor.rs:1214` | `decoded[..huff_kb]` | Branche « gros fichier » de `extract_v8_huffman_only` ; seule la petite branche vérifie (`:1196`). |
| 3 | `legacy_xor.rs:1216` | `&decoded[huff_kb..]` | Même bloc que #2. |
| 4 | `legacy_xor.rs:1267` | `&decoded[..huff_kb]` (`assemble_v8_lz_stream`) | Aucun check de longueur après `huffman_decode` (`:1245`). |
| 4b | `legacy_xor.rs:1269` | `&decoded[huff_kb..]` | Même bloc. |
| 5 (racine) | `legacy_xor.rs:731` | `vec![0u8; orig_size]` | `orig_bits` atteint 64 (`:661`) ⇒ `orig_size` (`:662`) peut dépasser `isize::MAX` ⇒ **panique « capacity overflow »**. Nourrit #2–#4b. |

**Tier arithmétique (panique en debug/test, wrap silencieux en release)** :
`v3_format/map.rs:355` (`width*height*layer_cnt*4`, 4 mult. `u32` bruts, le plus
facile à déclencher), `legacy_xor.rs:554,580,1387,541` (additions d'offsets
attaquant ; les *checks* voisins utilisent `saturating_add` mais l'addition brute peut
wrapper avant).

**DoS / alloc non bornée (abort `handle_alloc_error`, pas une panique rattrapable)** :
`legacy_xor.rs:779` (`vec![0u8; dest_size]`), `dat_parser.rs:158/193/239/244`.

**Filet de sécurité pour un futur fix** (round-trips byte-exacts déjà présents) :
`v3_format/map.rs:657,675,687,713`, `common_events.rs:544,555,601`,
`compression.rs:155`, `coder.rs:173,185,197`, `injector.rs` (`test_round_trip_*`
`:822,887,998`), `dat_parser.rs:647`, `legacy_xor.rs:1945`.

**Remède local** : borner `ns`/`decoded.len()` avant les slices, plafonner
`orig_size`/`dest_size` contre un maximum sain, passer les maths d'offset en
`checked_add`. Aucune refonte nécessaire.

> Nuance importante : `catch_unwind` autour de `Map::parse` /
> `common_events_parser` (`extractor.rs:521,841`) **n'est pas un défaut** — c'est un
> garde-fou volontaire qui convertit une panique de la lib tierce en
> `ExtractorError`. À conserver.

---

## 3. Extractors — duplication inter-moteurs & pollution TM

### 3.1 🟠 `mv_mz` : `gameTitle` pollue la source (et le TM)

`extract_system` réécrit `gameTitle` en `format!("{title} by Hoshi2Star")` et le
pousse comme **`source`** (`mv_mz/extractor.rs:287-295`), en contournant
`needs_translation` (seul garde : `trim().is_empty()`). La source stockée **n'égale
plus le texte du jeu** → empoisonne les lookups TM exacts (`tm::lookup_exact` par
hash de source) et injecte une marque anglaise en dur. `vx_ace` ne le fait **pas**
(`vx_ace/extractor.rs:348-355`) — divergence. Aucun test ne fige la source mutée.

### 3.2 🟠 `vx_ace` : fork figé de `mv_mz`, points d'entrée non testés

- C'est un **jumeau quasi identique de `mv_mz` qui a divergé** : `ExtractedSegment` +
  `new()` **byte-identiques** (`mv:43-62` ↔ `vx:48-66`), `extract_simple_array`
  **identique** (`mv:378-401` ↔ `vx:464-487`), `extract_event_list` quasi identique
  (`mv:407-462` ↔ `vx:495-539`).
- Les **vrais points d'entrée** `extract_from_bytes` (`vx:78`) et `dispatch_extract`
  (`vx:434`) ont **0 test** et aucune fixture `.rvdata2` → le chemin Marshal binaire
  est entièrement non vérifié (moteur désactivé au détecteur, `detector.rs:54-62`).

### 3.3 🟠 `wolf/extractor.rs` : 1712 lignes, duplication interne

- **Deux orchestrateurs quasi jumeaux** : `extract_all_wolf` (`:444-495`, celui
  **réellement utilisé** par `open_project`) et `extract_wolf_project` (`:970-1009`).
- **Boucles v2/v3 dupliquées** (map `:532-575` ↔ `:594-638` ; CE `:853-894` ↔
  `:910-952`) et **idiome scan-archive copié 5×** (`:237-260,294-334,372-387,412-432,
  810-826`).
- Incohérences : `eprintln!` vs `log::warn!` ; deux prédicats « japonais » divergents
  (`:142` vs `:662`) ; `.expect` vs `.unwrap` sur regex statiques (`:129/133` vs `:652`).

### 3.4 Le test « est-ce un fichier Map ? » réécrit **4×**

`starts_with("Map") && parse::<u32>` : `vx_ace/extractor.rs:435-444`,
`project.rs:781-788` (classify_vx), `:853-860` (classify_mv), `:883-890`
(dispatch_mv). Plus deux tables `filename→type` parallèles (`project.rs:792-806` vx,
`:864-878` mv).

---

## 4. UI — findings (au-delà de §1.4/§1.5)

### 4.1 `translatedCount` : **deux sens sous un seul nom** (défaut transverse)

Trois définitions coexistent :

- `get_source_files` → `SUM(CASE WHEN target_text != '')` — **tout target non vide**,
  tous statuts (`project.rs:295`) → alimente `SourceFile` (`types.ts:33`).
- `get_project_stats` → `COUNT(status = 'translated')` — **strict** (`project.rs:472`)
  → alimente `ProjectStats` (`types.ts:47`).
- Rapport QA → `status IN ('translated','reviewed','needs_review')` (`report.rs:61`).

Points d'affichage divergents pour le **même projet** :
`FileTree.tsx:177-178` (`isComplete = translatedCount === totalCount`, def. target≠'')
vs `AppToolbar.tsx:270-274` (%, def. status='translated') vs `ProjectList.tsx:245,263`
(barre + « ✓ N », def. status='translated'). Un fichier entièrement `needs_review`
apparaît **complet dans le FileTree** mais **< 100 % dans la toolbar/liste** — c'est
l'incohérence #8, désormais tracée jusqu'à ses 3 sources SQL.

### 4.2 Modèle par défaut : **divergence à 3 voies**

`provider.rs:83` `qwen3:4b-instruct-2507-q4_K_M` (fallback backend, aussi
`glossary.rs:159`) · `settings.ts:23` `qwen3:4b-instruct-2507-q8_0` · `llm.ts:50`
`qwen3:4b`. Au chargement, `settings.ts:82` pousse `q8_0` dans la config LLM, donc
`qwen3:4b` et le fallback backend `q4_K_M` sont tous deux effectivement masqués — mais
tout chemin s'exécutant **avant** le chargement des settings, ou le fallback backend,
utilise un modèle différent de celui affiché.

### 4.3 Polish / i18n

- **Bouton debug expédié en prod** : `AppToolbar.tsx:300-322` — titre en dur, `alert()`
  brut (bloque la webview, cf. règle harness), chaînes FR « Debug JSON écrit : » /
  « Erreur : ».
- **Chaînes anglaises hors i18n** : `GlossaryPanel.tsx:44-52,95,500,504`,
  `QAPanel.tsx:173` (« ok »). i18n existe (`lib/i18n.ts`, fr/en) mais est contournée à
  ces endroits.
- **`QaErrorType` en snake_case** (`types.ts:105-109`, `QAPanel.tsx:41-43`) casse la
  convention camelCase de tous les autres DTO.
- **SRP** : `SegmentGrid` (fetch + 4 listeners + glossaire + orchestration traduction),
  `ProjectList` (fetch stats), `SettingsModal.fetchModels` — logique qui appartient à
  un store/hook.

---

## 5. `core/{qa,report,glossary}.rs` — détail

- **`glossary.rs` — N+1 SQL** : un `SELECT COUNT(*)` de dédup **par terme** (jusqu'à
  50) dans la boucle (`:235-242`) ; devrait être un pré-fetch unique ou
  `INSERT OR IGNORE`. Pire, `unwrap_or(0)` (`:242`) **masque une erreur DB** → insert
  dupliqué possible ; et la dédup **ignore `lang_pair`** (`:235-236`).
- **`glossary.rs` — parsing `lang_pair` fragile** : `split('-').nth(1).unwrap_or("en")`
  (`:205`) assume la forme exacte `"xx-yy"` et retombe silencieusement sur `"en"`.
- **DRY `QaError` (4 variantes) répété dans 5 endroits** : `qa.rs:257,289,314` +
  `report.rs:91,241`. Une table `kind()→(pénalité, label)` les collapse.
- **`report.rs` — date maison** : algorithme grégorien réécrit à la main (`:192-217`),
  non testé, SRP douteux (préférer une crate).
- Points forts confirmés : `qa.rs` est **pur** et bien testé (20+ tests) ; `report.rs`
  échappe XML partout (`:314,442,459,481`) et fait un **seul** SELECT avec
  `ROW_NUMBER()` (pas de N+1 au fetch) ; `glossary.rs` gère les blocs `<think>` +
  échecs LLM sans panique (`:271-299`).

---

## 6. #8 vérifié sur le schéma réel + statut de `reviewed`

**Schéma** (`migrations/0001_initial.sql`) : `status CHECK(status IN
('untranslated','translated','reviewed','needs_review'))` — **4 statuts**. Colonne
`qa_score INTEGER`. Pas de colonne « reviewed_count » ni de vue de stats.

**`get_project_stats` compte 3 statuts** (`project.rs:469-475`) et **ignore
`reviewed`** → `total_segments ≠ untranslated + translated + needs_review` dès qu'un
segment est `reviewed`.

**Mais `reviewed` est aujourd'hui inatteignable** : `update_segment` écrit **toujours**
`status = 'translated'` en dur (`project.rs:393`) ; le pipeline écrit `translated` ou
`needs_review` ; **aucun** chemin Rust ou UI n'écrit `'reviewed'` (grep : présent
seulement en affichage — `types.ts:8`, `columns.tsx:28`, `locales/*`,
`SegmentGrid.tsx:154,517`). → La divergence est **latente** (capacité morte : schéma +
UI d'affichage sans producteur), **pas un bug actif**. Classée **Mineure**.

**Décision produit — TRANCHÉE (2026-07-01) : Option 2.** On **expose
`needs_review_count` par fichier** au lieu de cacher `needs_review` dans un seuil
binaire.

Concrètement :
- `get_source_files` renvoie `translated_count` (**`status='translated'`**, plus
  `target_text != ''`) **+** un nouveau `needs_review_count`.
- `ProjectStats` gagne un `reviewed_count` pour que la barre somme à 100 %.
- Le FileTree affiche les deux (ex. `✓ 17 · ⚠ 3`) et ne montre plus le ✓ « complet »
  (ni le bouton inject) tant qu'il reste du `needs_review` → **on n'invite plus à
  exporter des fallbacks LLM non revus**.
- `SegmentStatsBar` (ProjectList) rend aussi le segment `reviewed`.

Rejetées : Option 1 (aligner `get_source_files` sur `status='translated'` sans
granularité — trop pauvre pour un outil CAT) et Option 3 (compter `target_text != ''`
côté stats projet — masque le besoin de revue, gate d'export trompeuse).

Bénéfice de bord : l'Option 2 pose la plomberie qui donnera enfin un sens au statut
`reviewed` (aujourd'hui mort, §ci-dessus) via une future action « marquer revu ».
Plan d'implémentation détaillé : voir `docs/audit-remediation-plan-2026-07-01.md`
(Phase 5).

---

## 7. Dette DRY / SRP (tier séparé — pas du nouveau signal correctness)

- **Backend** (déjà en §7 de l'audit initial, confirmé) : recalcul stats ×3, filtre
  glossaire ×2, boucle INSERT ×3, injection par-moteur dupliquée, `is_map_file` ×4.
- **UI** : dialogue open-game + erreur moteur **×3** (`AppToolbar.tsx:161-181`,
  `ProjectList.tsx:53-66,88-107`) ; handlers d'export quasi identiques (`QAPanel.tsx:
  123-139` ↔ `TMPanel.tsx:72-84`) ; `get_glossary` + `"ja-en"` ×3
  (`SegmentGrid.tsx:203,216`, `GlossaryPanel.tsx:228`) ; listener
  `h2s://glossary/extraction-done` doublé (`SegmentGrid.tsx:211`,
  `GlossaryPanel.tsx:243`) ; maps statut→couleur éparpillées (aucune source unique).
- **`vx_ace`** : duplication massive de `mv_mz` (§3.2) — candidat n°1 à un helper
  partagé (les deux moteurs JSON-Pointer), **sans** y forcer Wolf (binaire/typé).

---

## 8. Impact proposé sur les notes /10

| Axe | Initial | Proposé | Motif |
|---|---|---|---|
| Qualité globale | 8 | **7** | 1 bug Critique (perte de données CE Wolf) découvert |
| Bonnes pratiques (§9 initial) | — | ↓ | rejets de promesses UI non gérés, `alert()` en prod, N+1 glossaire |
| Respect DRY | 6 | **5–6** | duplication UI confirmée en plus du backend (`vx_ace`, open-game ×3) |
| Performance | 7 | 7 (inchangé) | N+1 glossaire ajouté au tableau, mais échelle actuelle OK |
| Architecture / Lisibilité / DIP | 8–9 | inchangés | le cœur reste sain ; les défauts sont localisés |

---

## 9. Priorisation consolidée (corrige l'ordre A→F initial)

1. **🔴 §1.1 — `inject_common_events`** : perte de données réelle sur le moteur du
   lancement public. À traiter **en premier**, avant toute cosmétique. Filet :
   round-trips byte-exacts existants.
2. **🟠 §2 — 5 sites de panique `legacy_xor.rs`** (borner L731 + slices L591/1214+) :
   crash sur `.wolf` malformé.
3. **🟠 §1.4/§1.5 — rejets de promesses UI + plafond 5000** : robustesse frontend.
4. **🟠 §1.2/§1.3 — qa largeur kana + report glossaire/dénominateur morts**.
5. **🟡 Phase A initiale** — alignement #8 (**décision : Option 2**, cf. §6), modèle
   par défaut, `H2sError` doc≠code (`H2sError` : **0 occurrence**, tout est
   `Result<T,String>`).
6. **🟡 DRY §7 + Phase E (tests)** — filet avant les refactos structurelles.

---

## 10. Ce qui reste non vérifié après ce complément

- `commands/qa.rs`, `commands/glossary.rs` (façades) : lues indirectement via leurs
  usages, pas line-read intégralement.
- `wolf/injector.rs` : line-read pour les paniques (durci) ; la **logique** d'injection
  v2/v3 map/dat n'a pas été auditée pour la correction sémantique (seuls les
  round-trips tests l'attestent).
- Composants UI secondaires (`AppDialogs`, `TranslateAllDialog`, `FontSizeDialog`,
  `AboutModal`, `SegmentSearchBar`) : non line-read.
- Aucun test frontend n'existe (Vitest absent de `package.json`) et
  `src-tauri/tests/` est vide — donc **aucun** des bugs UI ci-dessus n'est couvert par
  un test de non-régression.
