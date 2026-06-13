# Plan — Fix collision DataBase.dat dans archives .wolf v8 (2026-06-13)

## Contexte / diagnostic confirmé

Sur 月咲流ホノカ ver1.03 (`.wolf)` archive, `test/月咲流ホノカver1.03(.wolf)/Data.wolf` v8),
l'extraction affiche :
```
warn: skipping sysdatabasebasic.dat — dat parse error in sysdatabasebasic: invalid magic number
warn: skipping database.dat — dat parse error in database: unsupported format: encrypted database not supported in F4-03 (deferred to F4-05)
```

Diagnostic vérifié par un test temporaire (`extract_all` sur l'archive réelle) :
- L'archive contient **deux copies** de `DataBase.dat` (et `CDataBase.dat`,
  `SysDatabase.dat`, `SysDataBaseBasic.dat`) :
  - `/BasicData/DataBase.dat` (85738 octets, `byte0=0x00`, magic SJIS valide
    → identique au `DataBase.dat` du dossier `Data/` en clair, qui parse OK)
  - `/データ集/（完全初期状態データ）/Data/BasicData/DataBase.dat` (257 octets,
    `byte0=0xbd`, format non reconnu — un "jeu de données état initial" fourni
    en bonus par le développeur, PAS une vraie base Wolf)
- `legacy_xor::WolfFile` ne stocke que le **nom de base** (`DataBase.dat`), pas
  le chemin. `extract_dat_pairs_from_archives` (extractor.rs:190-233) indexe
  par `stem` dans un `HashMap` → la 2e copie écrase la 1ère. Résultat : la
  copie de 257 octets (non valide) est celle envoyée à `parse_database`, qui
  la rejette à cause de `byte0 != 0x00` → message "encrypted... F4-05" (faux,
  ce n'est pas un chiffrement de base de données, F4-05 est déjà fait et ne
  couvrait pas ce cas).
- `CDataBase.dat` a la même collision mais **silencieuse** : les deux copies
  ont un en-tête valide (`byte0=0x00` + magic OK), donc selon l'ordre
  d'itération c'est potentiellement la copie de 257 octets (tronquée) qui
  "gagne" → perte silencieuse de segments à traduire, sans aucun warning.
- `SysDataBaseBasic.dat` a un format différent (`byte0=0x00` mais magic tout à
  zéro, ne correspond à aucun des deux magics attendus) — il est censé être
  ignoré (comme côté dossier `Data/` en clair, extractor.rs:621), mais le
  test de comparaison sur le chemin archive est sensible à la casse
  (`stem == "SysDataBaseBasic"` alors que `stem` est déjà en minuscules ici)
  et ne matche jamais.

Vérification de faisabilité : chemin complet reconstruit avec succès via la
table `DARC_DIRECTORY` (v8) en réutilisant la logique de
`build_per_file_key_str` (parent chain walk), donné `/BasicData/DataBase.dat`
vs `/データ集/.../Data/BasicData/DataBase.dat`. Confirmé sur l'archive réelle.

## Solution retenue

Reconstruire le chemin complet de chaque fichier dans les archives DXA **v8**
(c'est le format vérifié ; v5/v6 restent en tâche dormante, déclencheur =
collision constatée sur un jeu v5/v6), puis filtrer dans
`extract_dat_pairs_from_archives` pour ne garder que les fichiers situés
**directement** sous `/BasicData/` (1 seul niveau depuis la racine de
l'archive), miroir exact du filtre `Data/BasicData/` côté dossier en clair
(`load_dat_files`, extractor.rs:605-635).

## Étapes

- [x] **1. `legacy_xor.rs` — ajouter le champ `path: String` à `WolfFile`**
  (additif, ne casse pas les tests existants qui vérifient `.name`).
  `path` = chemin complet depuis la racine de l'archive, commence par `/`,
  séparateur `/`, ex. `/BasicData/DataBase.dat`.

- [x] **2. `extract_all_v8` — peupler `path`** en réutilisant le walk de
  `parse_darc_dirs` / `find_parent_dir` (déjà calculés pour la dérivation de
  clé par fichier), mais avec `read_original_name` (nom original, pas
  uppercase) pour chaque ancêtre, dans l'ordre racine → fichier, joints par
  `/`, préfixés par `/`.

- [x] **3. `extract_all` (v5/v6, fonctions internes correspondantes) —
  peupler `path` = `/` + `name`** (comportement actuel inchangé, pas de
  reconstruction d'arbre pour ces formats — hors scope, tâche dormante si
  collision constatée).

- [x] **4. `extractor.rs::extract_dat_pairs_from_archives` (lignes
  190-233)** :
  - Ne garder que les fichiers dont `path` a exactement la forme
    `/BasicData/<nom>` (1 niveau, comparaison `eq_ignore_ascii_case` sur le
    segment de dossier `BasicData`), avant de construire `project_map` /
    `dat_map`. Les copies imbriquées (ex. sous `データ集/.../BasicData/`) sont
    ainsi exclues d'office — fini la collision, pour `DataBase.dat`,
    `CDataBase.dat`, `SysDatabase.dat` ET `SysDataBaseBasic.dat`.
  - Corriger le skip de `SysDataBaseBasic` (actuellement `stem ==
    "SysDataBaseBasic"`, jamais vrai car `stem` est en minuscules sur ce
    chemin) en `stem.eq_ignore_ascii_case("SysDataBaseBasic")`, comme
    `extractor.rs:621` côté dossier en clair.

- [x] **5. `dat_parser.rs:377-382` — message d'erreur**
  Corriger le texte trompeur "encrypted database not supported in F4-03
  (deferred to F4-05)" — F4-05 est terminé et ne couvrait pas ce cas. Nouveau
  message neutre, ex. `"unsupported database format (indicator byte {X:#04x},
  expected 0x00)"`, sans référence à une phase de roadmap inexistante.

- [x] **6. Tests**
  - `legacy_xor.rs` : test sur l'archive réelle
    `test/月咲流ホノカver1.03(.wolf)/Data.wolf` vérifiant que `path` des deux
    entrées `DataBase.dat` diffère et correspond aux valeurs attendues
    (`/BasicData/DataBase.dat` vs le chemin imbriqué `データ集/...`).
  - `extractor.rs` : test `extract_dat_pairs_from_archives` sur la même
    archive → vérifie que la paire `database` retournée a bien 85738 octets
    de `.dat` (pas 257), et que `sysdatabasebasic` n'apparaît PAS dans le
    résultat (skip).
  - `dat_parser.rs` : adapter/ajouter un test pour le nouveau message
    d'erreur si un test existant vérifie le texte exact.

- [x] **7. Gate de vérification**
  `cargo fmt && cargo clippy --manifest-path src-tauri/Cargo.toml -- -D
  warnings && cargo test --manifest-path src-tauri/Cargo.toml` — OK (305
  tests, 0 warning). Vérification via `extract_all_wolf` : archive `.wolf`
  → `database.dat`: 349 segments, `cdatabase.dat`: 30 segments — identique au
  dossier en clair `test/月咲流ホノカver1.03/` (`DataBase.dat`: 349,
  `CDataBase.dat`: 30). `sysdatabasebasic` absent des deux côtés. Plus aucun
  warning "skipping ...".

- [x] **8. Documentation**
  `ROADMAP.md` (note de fix sous F5), `CHANGELOG.md` (entrée Fixed),
  `tasks/lessons.md` (règle générale ajoutée).

## Solution de contournement immédiate (sans coder)

Le dossier en clair `test/月咲流ホノカver1.03/` (sans `(.wolf)`) contient déjà
`Data/BasicData/DataBase.dat` non chiffré et correct (85738 octets, magic
valide). `load_dat_files` (extractor.rs:605-635) préfère ce chemin si présent
— ouvrir CE dossier dans Hoshi2Star aujourd'hui extrait tout sans aucun
warning, le bug ne touche QUE le chemin "archive `.wolf` seule".

## Hors scope

- v5/v6 path reconstruction (tâche dormante, déclencheur = collision
  constatée sur jeu utilisant ces versions DXA).
- Tout vrai "Database Protect" (chiffrement interne `database.dat`,
  `byte0 != 0x00` ET aucune copie valide disponible) — n'est PAS le cas ici ;
  si rencontré sur un autre jeu, ce sera une tâche séparée (recherche d'algo).

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
- [ ] **Filtre "segments non-traduisibles" (skip)** — analyse 2026-06-13 sur
  `Densyanai_Inko_ver2.0` (export complet des 2030 segments :
  `test/Densyanai_Inko_ver2.0_segments.json`, champ `skip`/`skip_reason`
  ajouté par l'analyse) :
  - 9 segments `Database/CDataBase/18/*/文字列` contenant
    `この値は「自動ｼｽﾃﾑ初期化」処理で...セットしてます` — notes internes Wolf
    indiquant que la valeur est réécrite au runtime par le système, jamais
    affichée telle quelle.
  - 7 segments `CommonEvents/X[移]パラメータ増減/70/*`, tous identiques
    (`@1\n\\>\\cself[9]`) — message de debug affichant une variable interne,
    aucun contenu narratif.
  - 1 segment valeur numérique nue (`"12"`, `◆立絵03 前立絵/1118/164`) —
    probablement un paramètre de coordonnée stocké comme texte.
  - Total : 17/2030 (0.84%). Pas de perte de contenu si on ne fait rien (au
    pire on traduit ~17 chaînes inutiles).
  - Pistes écartées (trop incertaines) : `CommonEvents/WOLF RPGエディター使用Ev`
    (4 segments, event par défaut du template WOLF RPG Editor) et
    `TitleMap.mps` (8 segments, dialogue tutoriel ウルファール par défaut) —
    possiblement morts/jamais appelés, mais vérifier nécessiterait de mapper
    les `cid` WolfTL v3.5 pour `CommonEvent call`/`Transfer` (risque de
    classer à tort du contenu réel comme mort). Abandonné.
  - **Déclencheur** : si le même pattern (`自動ｼｽﾃﾑ初期化` CDataBase type 18,
    debug `\cself[]` dump) est observé sur un **2e jeu** Wolf RPG → ça devient
    un vrai pattern de "base system" réutilisé → construire
    `core/skip_rules.rs` (fonction pure `classify_segment`) + colonne
    `segments.skip_reason TEXT NULL` (migration additive) + filtre dans
    `llm/pipeline.rs::translate_batch` + badge UI. Tant qu'un seul jeu est
    concerné, ne pas implémenter (cf. CLAUDE.md Scope Discipline).

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
