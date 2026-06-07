# Plan F4-03 — Wolf RPG : Extracteur texte (.mps + .dat)

## Objectif

Extraire tout le texte traduisible des fichiers Wolf RPG binaires :
- `.mps` (cartes — dialogues, choix) via `wolfrpg-map-parser`
- `.dat` (databases + CommonEvents — noms, descriptions, dialogues) via portage manuel de WolfTL

C'est le composant le plus long de F4 (~7–10 jours). Le parsing `.dat` est l'effort technique
dominant, sans bibliothèque Rust existante.

## Statut : [ ] À faire

## Prérequis

- F4-01 complet (Engine::Wolf, tokenizer, stubs) — `[ ]` À faire
- F4-02 complet (décrypteur DXA) — `[ ]` À faire
- `encoding_rs` et `wolfrpg-map-parser` dans `Cargo.toml` — `[ ]` À faire (F4-01 Step 1)
- Référence portage `.dat` : WolfTL C++ source (Sinflower, MIT) sur GitHub

## Estimation

9 steps · ~7–10 jours (dont ~4–6 jours pour le parsing .dat ⚠️)

## Items ROADMAP concernés

```
F4 — Engine Layer — Wolf RPG v1/v2 :
  [ ] src-tauri/src/engines/wolf/extractor.rs
  [ ] Tests round-trip Wolf v1/v2
```

---

## Steps

---

### Step 1 — Créer extractor.rs + types Wolf

**Objectif :** Définir les types de données spécifiques au module Wolf :
`WolfSegmentKind`, réutiliser `ExtractedSegment` de mv_mz si compatible
ou définir un équivalent Wolf.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/extractor.rs`

**Dépend de :** F4-01 Step 2

Tâches :
- [ ] Vérifier si `mv_mz::extractor::ExtractedSegment` peut être partagé :
  - Si oui : re-exporter ou importer directement depuis wolf/extractor.rs
  - Si non (champs incompatibles) : définir `wolf::extractor::WolfSegment { key, source, kind }`
  - Recommandation : extraire `ExtractedSegment` dans `domain/types.rs` ou `engines/mod.rs`
    pour partage cross-moteur — si le refactor est court (<30 min), le faire ; sinon
    dupliquer et noter dans `tasks/todo.md` comme refactor futur
- [ ] Définir `WolfSegmentKind` :
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum WolfSegmentKind {
      /// Dialogue line (commande Message)
      Dialogue,
      /// Choice option
      Choice,
      /// Actor/character name (from Actors.dat or DB)
      ActorName,
      /// Item name
      ItemName,
      /// Item description
      ItemDescription,
      /// Skill name
      SkillName,
      /// Skill description
      SkillDescription,
      /// CommonEvent dialogue
      CommonEventDialogue,
      /// Database field (generic named entry)
      DatabaseField { db_name: String, field_name: String },
  }
  ```
- [ ] Définir les erreurs :
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum ExtractorError {
      #[error("encoding error (Shift-JIS decode failed): {0}")]
      Encoding(String),
      #[error("parse error in {file}: {reason}")]
      Parse { file: String, reason: String },
      #[error("I/O error: {0}")]
      Io(#[from] std::io::Error),
  }
  ```

**Test de validation :**
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Résultat attendu : compile sans erreur

Commit message : `feat(wolf/extractor): define WolfSegmentKind + ExtractorError types`

---

### Step 2 — Décodage Shift-JIS → UTF-8 (encoding_rs)

**Objectif :** Implémenter les deux fonctions d'encodage indispensables :
`decode_shiftjis` (lecture fichiers v2) et `encode_shiftjis` (pour F4-04 injection v2).
Placer dans un module `wolf/encoding.rs` séparé pour réutilisation.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/encoding.rs` ← créer
- `src-tauri/src/engines/wolf/mod.rs` ← ajouter `pub mod encoding;`

**Dépend de :** F4-01 Step 1 (encoding_rs dans Cargo.toml)

**⚠️ Critique :** Wolf v2 crashe si du texte UTF-8 est injecté dans des `.dat` Shift-JIS.
Cette fonction est le garde-fou pour l'injection (F4-04).

Tâches :
- [ ] Implémenter `pub fn decode_shiftjis(bytes: &[u8]) -> Result<String, ExtractorError>` :
  ```rust
  use encoding_rs::SHIFT_JIS;
  pub fn decode_shiftjis(bytes: &[u8]) -> Result<String, ExtractorError> {
      let (result, _enc, had_errors) = SHIFT_JIS.decode(bytes);
      if had_errors {
          return Err(ExtractorError::Encoding("Shift-JIS decode had unmappable bytes".into()));
      }
      Ok(result.into_owned())
  }
  ```
- [ ] Implémenter `pub fn encode_shiftjis(text: &str) -> Result<Vec<u8>, ExtractorError>` :
  ```rust
  pub fn encode_shiftjis(text: &str) -> Result<Vec<u8>, ExtractorError> {
      let (bytes, _enc, had_errors) = SHIFT_JIS.encode(text);
      if had_errors {
          return Err(ExtractorError::Encoding(
              format!("text contains characters not encodable in Shift-JIS: {text}")
          ));
      }
      Ok(bytes.into_owned())
  }
  ```
- [ ] Tests :
  - `test_decode_shiftjis_hello` : bytes Shift-JIS de "こんにちは" → UTF-8 "こんにちは"
  - `test_decode_shiftjis_ascii` : bytes ASCII → identique (ASCII est un sous-ensemble de SJIS)
  - `test_encode_shiftjis_round_trip` : "悪魔の物語" → encode → decode → identique
  - `test_encode_shiftjis_rejects_non_encodable` : emoji ou caractère hors SJIS → `Err`

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::encoding::tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 4 tests verts, clippy vert

Commit message : `feat(wolf/encoding): Shift-JIS decode/encode via encoding_rs (critical for v2 injection)`

---

### Step 3 — Parsing .mps via wolfrpg-map-parser

**Objectif :** Extraire les segments de dialogue et de choix depuis les fichiers `.mps`
(cartes Wolf RPG) en utilisant `wolfrpg-map-parser`. Filtrer les textes vides et
placeholder-only (pattern identique à mv_mz).

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/extractor.rs`

**Dépend de :** Step 1, Step 2

**Référence :** `wolfrpg_map_parser::Map::parse(&bytes)` → structs Rust.
Lire la doc de la crate pour les codes de commande Wolf : Message=101(?), Choices=102(?).
⚠️ Vérifier les codes exacts dans wolfrpg-map-parser — ils peuvent différer des valeurs MV/MZ.

Tâches :
- [ ] Lire la doc `wolfrpg-map-parser` (types publics, codes commandes) avant d'implémenter
  ⚠️ Si les codes de commande Message/Choices ne sont pas 101/102 comme MV/MZ, adapter
- [ ] Implémenter `pub fn extract_map(bytes: &[u8], version: &WolfVersion) -> Result<Vec<ExtractedSegment>, ExtractorError>` :
  1. Si v2 → décode Shift-JIS les bytes en UTF-8 d'abord ? OU passer les bytes bruts à la crate ?
     ⚠️ Vérifier si `wolfrpg-map-parser` gère Shift-JIS ou attend de l'UTF-8. Si attend UTF-8 :
     décoder d'abord. Si gère SJIS : passer directement.
  2. `let map = Map::parse(bytes).map_err(|e| ExtractorError::Parse { ... })?`
  3. Pour chaque événement → pour chaque page → pour chaque commande :
     - Message (code?) → `WolfSegmentKind::Dialogue`, clé = `"MapData/{map_name}/events/{e}/pages/{p}/{cmd_idx}"`
     - Choices (code?) → `WolfSegmentKind::Choice`, clé = `"MapData/{map_name}/events/{e}/pages/{p}/{cmd_idx}/choices/{c}"`
  4. Filtrer textes vides + whitespace-only (même logique que MV/MZ)
  5. Appliquer `is_placeholder_only()` avec `Engine::Wolf` (tokenizer Wolf)
- [ ] Fonctions helper :
  ```rust
  fn is_wolf_placeholder_only(text: &str) -> bool {
      // Port du is_placeholder_only MV/MZ avec Engine::Wolf
  }
  ```
- [ ] Tests :
  - `test_extract_map_dialogue` : bytes .mps synthétique avec une commande Message → 1 segment Dialogue
  - `test_extract_map_choices` : commande Choices avec 2 options → 2 segments Choice
  - `test_extract_map_skips_empty` : commande Message vide → 0 segments
  - `test_extract_map_key_format` : vérifier le format de clé `MapData/...`

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::extractor::tests::test_extract_map
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 4 tests verts, clippy vert

Commit message : `feat(wolf/extractor): parse_map via wolfrpg-map-parser — Dialogue + Choice segments`

---

### Step 4 — Discovery fichiers Wolf + déchiffrement conditionnel

**Objectif :** Créer `collect_wolf_files()` qui scanne le dossier Data/ et produit la liste
des (nom_fichier, type, bytes_utf8) — en déchiffrant les `.wolf` si présents, ou en lisant
directement les `.mps`/`.dat` si le projet n'est pas chiffré.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/extractor.rs`

**Dépend de :** Step 1, Step 2, F4-02 (decryptor)

Tâches :
- [ ] Définir l'enum de type de fichier Wolf :
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub enum WolfFileType {
      Map(String),      // nom de la carte (ex: "TitleMap")
      Database(String), // nom de la DB (ex: "BasicData", "UserDB0")
      CommonEvents,
  }
  ```
- [ ] Implémenter `pub fn collect_wolf_files(game_dir: &Path, version: &WolfVersion) -> Result<Vec<(WolfFileType, Vec<u8>)>, ExtractorError>` :
  1. Chercher `Data.wolf` (archive unique) ou dossier `Data/` avec plusieurs `.wolf`
  2. Si `.wolf` trouvé : appeler `decryptor::extract_all()` → obtenir `WolfArchive`
  3. Si pas de `.wolf` (projet non chiffré) : lire les `.mps`/`.dat` directement
  4. Pour chaque fichier : si v2 → décode Shift-JIS → UTF-8 (sauf bytes bruts conservés pour injector)
  5. Classer par type : `MapData/*.mps` → `Map(nom)`, `BasicData/*.dat` → `Database(nom)`
  6. Filtrer les fichiers non-texte (images, sons, etc.)
- [ ] ⚠️ Décision architecturale : les bytes passés à `extract_map` et aux parsers `.dat` sont-ils
  UTF-8 ou bruts ? Recommandation : passer les bytes bruts et gérer l'encodage dans chaque parser.
  Documenter ce choix dans un commentaire.
- [ ] Test : `test_collect_wolf_files_unencrypted` : dossier avec .mps et .dat non chiffrés →
  liste correcte de fichiers classés par type

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::extractor::tests::test_collect
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : test vert, clippy vert

Commit message : `feat(wolf/extractor): collect_wolf_files — discovery + conditional DXA decryption`

---

### Step 5 — ⚠️ Portage parsing .dat — structure binaire (en-tête)

**Objectif :** Implémenter la lecture de l'en-tête des fichiers `.dat` Wolf RPG.
Format binaire documenté par WolfTL. Couvre les databases (types, champs, données)
et CommonEvents.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/dat_parser.rs` ← créer
- `src-tauri/src/engines/wolf/mod.rs` ← ajouter `pub mod dat_parser;`

**Dépend de :** Step 1, Step 2

**⚠️ Étape la plus complexe de F4 — prévoir 2–3 jours**

**Référence :** WolfTL C++ source (`WolfTL.cpp`, `Database.h`, `CommonEvents.h`).
Lire le code WolfTL sur GitHub (Sinflower/WolfTL) AVANT d'implémenter.
Format .dat Wolf RPG (depuis WolfTL + Wolf Trans documentation) :

Structure d'un fichier `.dat` Database (type : User DB / System DB / Variable DB) :
```
[Header]
  u32 : magic number (identifier le type de fichier)
  u32 : number_of_types
  for each type:
    u32 : type_id
    string : type_name (longueur préfixée ou null-terminated selon version)
    u32 : field_count
    for each field:
      u32 : field_id
      string : field_name
      u32 : field_type  (0=int, 1=string, 2=string_array, ...)
[Data section]
  u32 : data_count
  for each data row:
    [field values]
```

⚠️ Le format exact (longueur des strings, endianness, versions) doit être vérifié dans
WolfTL source avant implémentation. La structure ci-dessus est approximative.

Tâches :
- [ ] Créer `src-tauri/src/engines/wolf/dat_parser.rs`
- [ ] Struct `DatField { id: u32, name: String, field_type: DatFieldType }`
- [ ] Enum `DatFieldType { Int, String, StringArray, Unknown(u32) }`
- [ ] Struct `DatType { id: u32, name: String, fields: Vec<DatField>, data: Vec<Vec<DatValue>> }`
- [ ] Enum `DatValue { Int(i32), String(String), StringArray(Vec<String>), Null }`
- [ ] Struct `DatFile { types: Vec<DatType> }`
- [ ] Implémenter `fn read_string(cursor: &mut Cursor<&[u8]>, version: &WolfVersion) -> Result<String, ExtractorError>` :
  - Gère les strings longueur-préfixée (u32 + bytes) vs null-terminated selon version
  - Décode Shift-JIS si v2, UTF-8 directement si v3+
- [ ] Implémenter `pub fn parse_dat_header(bytes: &[u8], version: &WolfVersion) -> Result<DatFile, ExtractorError>` :
  - Lire magic number → valider
  - Lire types et champs (pas les données — juste le schéma)
  - ⚠️ Stopper ici pour Step 5 — les données sont Step 6
- [ ] Tests :
  - `test_parse_dat_header_synthetic` : fichier .dat synthétique minimal → schéma correct

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::dat_parser::tests::test_parse_header
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : test header vert, clippy vert

Commit message : `feat(wolf/dat_parser): parse_dat_header — types, fields, schema (no data yet)`

---

### Step 6 — ⚠️ Parsing .dat — extraction champs texte traduisibles

**Objectif :** Implémenter la lecture des données (valeurs de champs) dans les `.dat` et
extraire uniquement les champs texte traduisibles (noms, descriptions). Ignorer les champs
numériques et les champs techniques (IDs, flags, etc.).

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/dat_parser.rs`

**Dépend de :** Step 5

**⚠️ Étape la plus lourde — prévoir 2–3 jours**

Tâches :
- [ ] Compléter `parse_dat_header` pour lire aussi les données :
  ```rust
  pub fn parse_dat_file(bytes: &[u8], version: &WolfVersion) -> Result<DatFile, ExtractorError>
  ```
- [ ] Implémenter `pub fn extract_translatable_segments(dat: &DatFile, db_name: &str) -> Vec<ExtractedSegment>` :
  - Pour chaque type → pour chaque donnée → pour chaque champ :
    - Si `field_type == String` ET `value != ""` ET valeur non-technique :
      → créer un segment avec clé `"Database/{db_name}/{type_id}/{data_idx}/{field_name}"`
  - **Champs traduisibles identifiés :**
    - Champs nommés "name", "名前", "description", "説明", "note", "備考"
    - Champs de type String qui contiennent du texte japonais (heuristique : contient hiragana/katakana/kanji)
  - **Champs à ignorer :** IDs, flags, paths de fichiers (`.png`, `.wav`), nombres
  - ⚠️ La liste des champs traduisibles dépend du jeu — noter que l'extraction est
    opportuniste et peut surextraire. Un filtre manuel (glossaire de champs connus) pourrait
    être ajouté en F5.
- [ ] Tests :
  - `test_extract_dat_actor_names` : .dat avec champs "name" → segments `ActorName`
  - `test_extract_dat_skips_numbers` : champs numériques → 0 segments
  - `test_extract_dat_key_format` : vérifier format de clé `Database/...`

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::dat_parser::tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : tests dat verts, clippy vert

Commit message : `feat(wolf/dat_parser): extract translatable string fields from .dat databases`

---

### Step 7 — ⚠️ Parsing CommonEvents.dat

**Objectif :** Parser le fichier `CommonEvents.dat` (ou équivalent) qui contient les
dialogues des événements communs (non liés à une carte). Structure similaire aux databases
mais avec des blocs de commandes event.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/dat_parser.rs` (ou `common_events.rs` séparé si la structure est très différente)

**Dépend de :** Step 5, Step 6

**⚠️ À investiguer :** Le nom exact du fichier et sa structure varient selon les versions Wolf RPG.
Vérifier dans WolfTL source quel fichier contient `CommonEvents` et quel est son magic number.

Tâches :
- [ ] Identifier le fichier CommonEvents dans la structure de données Wolf :
  - `Data/BasicData/CommonEvent.dat` ? `Data/BasicData/CommonEvents.dat` ?
  - ⚠️ Vérifier sur un vrai jeu Wolf v2 (Mad Father freeware)
- [ ] Implémenter `pub fn extract_common_events(bytes: &[u8], version: &WolfVersion) -> Result<Vec<ExtractedSegment>, ExtractorError>` :
  - Structure similaire à `.mps` mais dans un `.dat`
  - Commandes Message et Choices dans les listes de commandes des CommonEvents
  - Clé : `"CommonEvents/{event_name}/commands/{cmd_idx}"`
- [ ] Tests :
  - `test_extract_common_events_dialogue` : CommonEvent synthétique → 1 segment Dialogue
  - `test_extract_common_events_choices` : CommonEvent avec Choices → N segments Choice

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::dat_parser::tests::test_common_events
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 2 tests verts, clippy vert

Commit message : `feat(wolf/extractor): extract_common_events from CommonEvents.dat`

---

### Step 8 — Orchestrateur extract_all_wolf()

**Objectif :** Créer la fonction d'orchestration `extract_all_wolf()` qui combine tous les
extracteurs : discovery des fichiers, extraction .mps + .dat + CommonEvents, retourne une
liste plate de `ExtractedSegment` (comme MV/MZ dispatch_extract).

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/extractor.rs`

**Dépend de :** Step 3, 4, 6, 7

Tâches :
- [ ] Implémenter `pub fn extract_all_wolf(game_dir: &Path, version: &WolfVersion) -> Result<Vec<(String, String, Vec<ExtractedSegment>)>, ExtractorError>` :
  - Retourne une liste de `(file_name, file_path, segments)` — même pattern que MV/MZ
  - Appeler `collect_wolf_files()` → pour chaque fichier :
    - `.mps` → `extract_map()`
    - `.dat` Database → `extract_translatable_segments()`
    - CommonEvents → `extract_common_events()`
  - `file_name` = nom court (`MapData/TitleMap.mps`), `file_path` = chemin absolu
- [ ] ⚠️ Gestion erreur : si un fichier échoue à parser → logger un warning et continuer (ne pas
  stopper l'extraction entière) — cohérent avec le comportement MV/MZ
- [ ] Tests :
  - `test_extract_all_wolf_synthetic` : dossier synthétique avec .mps + .dat → segments corrects
  - `test_extract_all_wolf_empty_dir` : dossier vide → Ok(vec![])

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::extractor::tests::test_extract_all
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 2 tests verts, clippy vert

Commit message : `feat(wolf/extractor): extract_all_wolf() — orchestrator combining .mps + .dat + CommonEvents`

---

### Step 9 — Tests round-trip .mps synthétique

**Objectif :** Valider que extract → les clés générées sont cohérentes et permettront
à l'injector (F4-04) de retrouver exactement les bons emplacements binaires.
Test de cohérence des clés, pas encore de vrai round-trip injection (F4-04).

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/extractor.rs` ← section tests

**Dépend de :** Step 3, 8

Tâches :
- [ ] Test `test_key_uniqueness` : un seul fichier .mps ne doit pas produire 2 segments avec la même clé
- [ ] Test `test_key_format_mps` : toutes les clés .mps suivent le format `"MapData/{name}/events/{e}/pages/{p}/{cmd}"`
- [ ] Test `test_key_format_dat` : toutes les clés .dat suivent le format `"Database/{name}/{type}/{data}/{field}"`
- [ ] Test `test_segment_count_realistic` : sur un jeu Wolf v2 freeware (Mad Father si disponible
  dans les assets de test), le nombre de segments est > 0 et < 10 000 (sanity check)
  ⚠️ Ne pas commiter des assets de jeux — utiliser des données synthétiques ou
  des fixtures non-copyrightées dans `src-tauri/tests/fixtures/wolf/`

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::extractor::tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : tous les tests extractor verts (au moins 12 en tout), clippy vert

Commit message : `test(wolf/extractor): key uniqueness + format validation + round-trip prep`

---

## Tests obligatoires avant push GitHub

```bash
# Rust — tous les tests
cargo test --manifest-path src-tauri/Cargo.toml
# Attendu : 100% verts

# Rust — qualité code
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
# Attendu : 0 warning, 0 erreur

# Rust — formatage
cargo fmt --manifest-path src-tauri/Cargo.toml
# Attendu : aucun fichier modifié

# TypeScript — aucun fichier TS modifié
pnpm typecheck
# Attendu : 0 erreur
```

## Commandes git

```bash
git checkout -b feat/f4-03-wolf-extractor
# ... commits intermédiaires par step ...
git checkout main
git merge --no-ff feat/f4-03-wolf-extractor -m "feat(f4-03): Wolf RPG extractor — .mps (wolfrpg-map-parser) + .dat (WolfTL port)"
git push origin main
git branch -d feat/f4-03-wolf-extractor
```

## Mise à jour après complétion

- `ROADMAP.md` : cocher `src-tauri/src/engines/wolf/extractor.rs`
- `CHANGELOG.md` : entrée `Added` Wolf extractor .mps + .dat
- `docs/journal/` : nouvelle entrée `YYYY-MM-DD-f4-03-wolf-extractor.md`
- `docs/engines.md` : documenter les formats .mps/.dat Wolf RPG et les limites (champs
  non-traduisibles ignorés, LZSS si non supporté)
- Mettre à jour la mémoire session : F4-03 terminé, passer à F4-04
