# Plan F4-02 — Wolf RPG : Décrypteur DXA XOR

## Objectif

Implémenter le déchiffrement des archives `.wolf` (format DXA de DxLib) en Rust natif.
Algorithme XOR 12 octets symétrique, porté depuis GARbro `ArcDX.cs`. Couvre les versions
DXA v5 (32-bit), v6 et v8 (64-bit) — soit Wolf RPG v1/v2/v3 (~75–80% du catalogue DLsite
ciblé). WolfX/v3.5+ (ChaCha20) reste hors scope F4.

## Statut : [ ] À faire

## Prérequis

- F4-01 complet (`Engine::Wolf` dans detector + tokenizer) — `[ ]` À faire
- `src-tauri/src/engines/wolf/decryptor.rs` stub créé (F4-01 Step 2) ✅
- Référence code : GARbro `ArcFormats/DxLib/ArcDX.cs` (morkt/GARbro, licence MIT)
  — disponible sur GitHub, à consulter pendant l'implémentation

## Estimation

8 steps · ~3–4 jours

## Items ROADMAP concernés

```
F4 — Engine Layer — Wolf RPG v1/v2 :
  [ ] src-tauri/src/engines/wolf/decryptor.rs
  [ ] Tests round-trip Wolf v1/v2
```

---

## Steps

---

### Step 1 — Structs DXA + WolfArchive

**Objectif :** Définir les types de données qui représentent l'archive DXA et son contenu.
Exposer les erreurs via `thiserror`. Aucune logique d'I/O ici — types purs.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/decryptor.rs`

**Dépend de :** F4-01 Step 2

Tâches :
- [ ] Ajouter les structs et erreurs :
  ```rust
  use thiserror::Error;

  #[derive(Debug, Error)]
  pub enum DecryptorError {
      #[error("not a DXA archive (invalid signature)")]
      InvalidSignature,
      #[error("unsupported DXA version: {0}")]
      UnsupportedVersion(u8),
      #[error("DXA header too short")]
      HeaderTooShort,
      #[error("cannot guess key (header fields not null)")]
      CannotGuessKey,
      #[error("I/O error: {0}")]
      Io(#[from] std::io::Error),
  }

  /// One file extracted from a DXA archive.
  #[derive(Debug, Clone)]
  pub struct WolfFile {
      pub name: String,
      pub data: Vec<u8>,
      pub unpacked_size: u64,
  }

  /// A fully parsed DXA archive.
  #[derive(Debug)]
  pub struct WolfArchive {
      pub version: u8,           // 5, 6, or 8
      pub code_page: Option<u32>, // 932 = Shift-JIS, 65001 = UTF-8 (DXA v6+ only)
      pub files: Vec<WolfFile>,
  }
  ```
- [ ] ⚠️ Tous les champs `WolfFile` et `WolfArchive` non encore utilisés → pas de `#[allow(dead_code)]`
  global — laisser le compilateur signaler les champs inutilisés individuellement pour suivre
  la progression, puis retirer quand chaque champ est consommé.

**Test de validation :**
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Résultat attendu : compile sans erreur (types définis, pas encore de logique)

Commit message : `feat(wolf/decryptor): define WolfArchive, WolfFile, DecryptorError structs`

---

### Step 2 — KeyConv XOR 12 octets

**Objectif :** Implémenter la fonction `key_conv` — le cœur du déchiffrement DXA.
Algorithme XOR symétrique à clé 12 octets. Porté depuis DxLib/GARbro.
**Critique : l'algorithme de déchiffrement = l'algorithme de chiffrement (XOR symétrique).**

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/decryptor.rs`

**Dépend de :** Step 1

**Référence :** DxLib `DXArchive::KeyConv` / GARbro `Decrypt` :
```c
Position %= 12;
for i in 0..size { data[i] ^= key[j]; if ++j == 12 { j = 0; } }
```

Tâches :
- [ ] Implémenter `pub(crate) fn key_conv(data: &mut [u8], offset: u64, key: &[u8; 12])` :
  ```rust
  /// XOR `data` in place with key, starting at key_pos = offset % 12.
  /// Symmetric: applying twice restores original. Used for both decrypt and encrypt.
  pub(crate) fn key_conv(data: &mut [u8], offset: u64, key: &[u8; 12]) {
      let mut key_pos = (offset % 12) as usize;
      for byte in data.iter_mut() {
          *byte ^= key[key_pos];
          key_pos += 1;
          if key_pos == 12 {
              key_pos = 0;
          }
      }
  }
  ```
- [ ] ⚠️ Bug spécifique Wolf RPG (documenté dans le rapport) : pour les données de fichier,
  l'offset passé à `key_conv` est la **taille décompressée du fichier**, pas sa position
  dans l'archive. Documenter ce comportement dans un commentaire :
  ```rust
  // Wolf RPG bug: file data decryption offset = unpacked_size % 12
  // NOT the file position in the archive. See: docs/wolf-rpg-research.md §3
  ```
- [ ] Tests unitaires :
  - `test_key_conv_identity` : appliquer deux fois → données originales restaurées (symétrie)
  - `test_key_conv_known_vector` : vecteur de test calculé à la main pour clé Wolf v2.20 :
    key = `[0x38, 0x50, 0x40, 0x28, 0x72, 0x4F, 0x21, 0x70, 0x3B, 0x73, 0x35, 0x38]`,
    data = `[0x00]`, offset = 0 → `[0x38]` (XOR avec key[0])
  - `test_key_conv_offset_wraps` : offset = 12 → key_pos = 0 (modulo correct)
  - `test_key_conv_offset_wolf_bug` : offset = `unpacked_size` d'un fichier → résultat correct

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::decryptor::tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 4 tests verts, clippy vert

Commit message : `feat(wolf/decryptor): implement key_conv XOR-12 (symmetric, Wolf offset bug handled)`

---

### Step 3 — Table de clés hardcodées

**Objectif :** Définir les clés connues de Wolf RPG dans une table. Implémenter la fonction
de recherche par version. Ces clés évitent la détection automatique pour les jeux courants.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/decryptor.rs`

**Dépend de :** Step 2

**Source :** Table `DECRYPT_MODES` de WolfDec `main.cpp` (Sinflower).

Tâches :
- [ ] Définir les clés connues :
  ```rust
  /// Known Wolf RPG XOR keys, indexed by game version string.
  /// Sources: WolfDec DECRYPT_MODES table, GARbro, game-specific community docs.
  const WOLF_KEYS: &[(&str, [u8; 12])] = &[
      // Wolf v2.20 — clé la plus répandue (source : WolfDec DECRYPT_MODES)
      ("v2.20", [0x38,0x50,0x40,0x28,0x72,0x4F,0x21,0x70,0x3B,0x73,0x35,0x38]),
      // Wolf v2.01
      ("v2.01", [0x0F,0x53,0xE1,0x3E,0x04,0x37,0x12,0x17,0x60,0x0F,0x53,0xE1]),
      // Wolf v2.10
      ("v2.10", [0x4C,0xD9,0x2A,0xB7,0x28,0x9B,0xAC,0x07,0x3E,0x77,0xEC,0x4C]),
      // Wolf v2.255 — clé de 月咲流ホノカ (lue directement dans le header DXA à 0x40)
      // Vérifiée empiriquement : header Honoka non chiffré, clé stockée en clair.
      ("v2.255", [0xb8,0x58,0x8c,0x7b,0xca,0x3d,0x6f,0x3d,0x8c,0x34,0xf8,0x1a]),
      // No-key (DXA_FLAG_NO_KEY): memset(Key, 0xAA, 12) → keyCreate → constant key
      ("no_key", [0x55,0xAA,0x20,0x55,0x55,0x06,0x55,0xAA,0x55,0xD5,0x7C,0x66]),
      // Wolf v3.10/v3.173 : hors scope F4 — ne pas implémenter ici.
      // ⚠️ AMBIGUÏTÉ NON RÉSOLUE (à traiter avant F5) :
      //    docs/wolf-rpg-research.md §3 mentionne "clés plus longues (40-46 octets)"
      //    pour v3.10/v3.173 dans WolfDec, mais ne précise pas si ce sont :
      //    (1) les mots de passe bruts AVANT keyCreate() → clé XOR toujours 12 octets
      //        (DXA_KEYSTR_LENGTH = 12 est documenté comme constant dans le rapport)
      //    (2) les clés XOR elles-mêmes → DXA_KEYSTR_LENGTH aurait changé pour v3.x
      //        → [u8; 12] insuffisant, nécessiterait Vec<u8> ou enum WolfKey
      //    Vérifier WolfDec main.cpp AVANT d'implémenter le support v3.x.
  ];
  ```
- [ ] Implémenter `pub fn known_key(version_hint: Option<&str>) -> Option<[u8; 12]>` :
  - Si `version_hint` fourni → chercher dans `WOLF_KEYS`
  - Retourne `None` si inconnu → déclencher GuessKey (Step 6)
- [ ] Note sur Densyanai Inko ver2.0 : clé inconnue — header chiffré, GuessKeyV6 nécessaire (Step 6)
  ⚠️ Contrairement à Honoka, Densyanai Inko a un header DXA visiblement chiffré (valeurs
  aléatoires en dehors du CodePage). Valider GuessKeyV6 sur ce jeu comme cas de test réel.
- [ ] ⚠️ Tâche obligatoire avant tout support v3.10/v3.173 (F5, pas F4) :
  Lire WolfDec `main.cpp` section `DECRYPT_MODES` — les "40-46 octets" mentionnés dans
  docs/wolf-rpg-research.md §3 désignent-ils le mot de passe brut (→ [u8; 12] correct)
  ou la clé XOR elle-même (→ migrer vers `Vec<u8>` ou `enum WolfKey { Fixed([u8; 12]), Variable(Vec<u8>) }`) ?
  Ne pas coder cette partie avant d'avoir la réponse.

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::decryptor::tests::test_known_key
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : tests clés verts, clippy vert

Commit message : `feat(wolf/decryptor): hardcoded key table (v2.01/v2.10/v2.20/no_key)`

---

### Step 4 — Lecture en-tête DXA v5 (32-bit)

**Objectif :** Lire et déchiffrer l'en-tête d'une archive DXA version 5 (format 32-bit).
Les champs v5 sont des `u32` (32-bit). Retourner les offsets TOC nécessaires pour la suite.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/decryptor.rs`

**Dépend de :** Step 2, Step 3

**Référence :** GARbro `ArcDX.cs` — struct `DxHeader` v5 + `ReadHeader`.

Tâches :
- [ ] Struct `DxHeaderV5` (interne) :
  ```rust
  struct DxHeaderV5 {
      index_size: u32,
      base_offset: u32,
      index_offset: u32,
      file_table_offset: u32,
      dir_table_offset: u32,
  }
  ```
- [ ] Implémenter `read_header_v5(data: &[u8], key: &[u8; 12]) -> Result<DxHeaderV5, DecryptorError>` :
  - Vérifier longueur minimale (20+ octets pour v5)
  - Déchiffrer les bytes [4..24] avec `key_conv(&mut buf, 4, key)` (offset=4)
  - Lire les 5 champs u32 little-endian
- [ ] Impl `read_signature(data: &[u8]) -> Result<u8, DecryptorError>` :
  - Vérifier bytes [0..2] == `b"DX"`
  - Retourner byte [2] comme version (5, 6, ou 8)
  - Erreur si signature invalide ou version non supportée (> 8)
- [ ] Test : `test_read_header_v5_synthetic` — construire un en-tête v5 synthétique chiffré,
  déchiffrer, vérifier les champs

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::decryptor::tests::test_read_header
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : test v5 vert, clippy vert

Commit message : `feat(wolf/decryptor): DXA v5 header reading (32-bit fields)`

---

### Step 5 — Lecture en-tête DXA v6/v8 (64-bit) + CodePage

**Objectif :** Étendre la lecture de l'en-tête pour DXA v6 et v8 (champs 64-bit). Extraire
le champ `CodePage` (@0x24) pour détecter Shift-JIS (932) vs UTF-8 (65001) — cela
finalise `WolfVersion.is_utf8()` de F4-01.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/decryptor.rs`
- `src-tauri/src/engines/detector.rs` ← mettre à jour `guess_wolf_version_from_structure`

**Dépend de :** Step 4

**Référence :** GARbro — en-tête v6 à 0x2C octets.
⚠️ **Offset CodePage empirique :** sur 月咲流ホノカ (DXA v8, header non chiffré), CodePage 932
se trouve à l'offset **0x28** (et non 0x24 comme dans GARbro v6). Adapter la lecture selon la
version : v5 n'a pas de CodePage, v6 → vérifier 0x24, v8 → offset 0x28 confirmé empiriquement.

Tâches :
- [ ] Struct `DxHeaderV6` (interne, aussi utilisée pour v8) :
  ```rust
  struct DxHeaderV6 {
      index_size: u32,
      base_offset: u64,
      index_offset: u64,
      file_table_offset: u64,
      dir_table_offset: u64,
      code_page: u32,          // 932 = Shift-JIS, 65001 = UTF-8
  }
  ```
- [ ] Implémenter `read_header_v6(data: &[u8], key: &[u8; 12]) -> Result<DxHeaderV6, DecryptorError>` :
  - Longueur minimale : 0x2C (44 octets)
  - Déchiffrer [4..0x2C] avec `key_conv(&mut buf, 4, key)`
  - Lire tous les champs (u32/u64 little-endian)
  - Note : v8 utilise la même structure que v6 (à confirmer en lisant GARbro)
- [ ] Fonction unifiée :
  ```rust
  pub fn read_header(data: &[u8], key: &[u8; 12])
      -> Result<(u8, u64, u64, u64, u64, Option<u32>), DecryptorError>
  // Returns: (version, base_offset, index_offset, file_table, dir_table, code_page)
  ```
- [ ] Mettre à jour `detector::guess_wolf_version_from_structure` pour appeler le décrypteur
  si des `.wolf` sont présents — lire le CodePage et retourner la version correcte :
  ```rust
  pub fn detect_wolf_version(game_dir: &Path) -> WolfVersion {
      // Try to read CodePage from first .wolf archive found in Data/
      // If CodePage = 65001 → UTF-8 → v3+
      // If CodePage = 932 or absent → Shift-JIS → v2
  }
  ```
- [ ] Tests :
  - `test_read_header_v6_synthetic` : en-tête v6 synthétique chiffré → champs corrects + CodePage
  - `test_code_page_932_is_shiftjis` : CodePage 932 → version.is_utf8() = false
  - `test_code_page_65001_is_utf8` : CodePage 65001 → version.is_utf8() = true

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::decryptor::tests::test_read_header_v6
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 3 tests verts, clippy vert

Commit message : `feat(wolf/decryptor): DXA v6/v8 header + CodePage (UTF-8 vs Shift-JIS detection)`

---

### Step 6 — GuessKeyV6 (attaque texte clair connu)

**Objectif :** Implémenter la détection automatique de clé par attaque texte clair connu.
Les champs 64-bit nuls de l'en-tête DXA v6 révèlent directement les octets de clé via XOR.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/decryptor.rs`

**Dépend de :** Step 5

**Référence :** GARbro `GuessKeyV6` + code Python de himeworks dans `docs/wolf-rpg-research.md §3`.

La logique Python du rapport :
```python
if (header[0xC:0x10] == header[0x24:0x28]) and (header[0x14:0x18] == header[0x2C:0x30]):
    key = header[0xC:0x10] + header[0x1C:0x20] + header[0x14:0x18]
```

Tâches :
- [ ] Implémenter `pub fn guess_key_v6(raw_header: &[u8]) -> Option<[u8; 12]>` :
  - Les champs `FileTable` (@ 0x14, 8 bytes) et `DirTable` (@ 0x1C, 8 bytes) et `IndexOffset` (@ 0x0C, 8 bytes)
    contiennent des octets nuls dans leurs parties hautes (valeurs < 4 Go)
  - XOR(chiffré[offset..offset+4], 0x00000000) = key[offset%12 .. (offset%12+4)]
  - Vérifier la cohérence (la même dérivation de clé sur des champs différents doit donner le même résultat)
  - Valider en tentant de déchiffrer la signature "DX" — si échoue → `None`
  - Retourner `Some(key)` si cohérent, `None` si trop d'ambiguïté
- [ ] Ajouter `guess_key_v5` si GARbro en a une (sinon noter ⚠️ "À investiguer")
- [ ] Tests :
  - `test_guess_key_v6_known_archive` : construire un en-tête v6 synthétique chiffré avec une
    clé connue → `guess_key_v6` retrouve la clé
  - `test_guess_key_v6_random_data` : données aléatoires → retourne `None` (pas de fausse clé)

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::decryptor::tests::test_guess_key
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 2 tests verts, clippy vert

Commit message : `feat(wolf/decryptor): GuessKeyV6 — automatic key recovery from null header fields`

---

### Step 7 — Parsing TOC (DirTable + FileTable)

**Objectif :** Parser la table des répertoires et la table des fichiers du DXA pour
obtenir la liste des fichiers avec leurs offsets et tailles.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/decryptor.rs`

**Dépend de :** Step 5

**Référence :** GARbro `ArcDX.cs` — `ReadIndex`, `DxEntry`, `DxDirectory`.

Tâches :
- [ ] Struct interne `DxFileEntry` :
  ```rust
  struct DxFileEntry {
      name_offset: u64,    // offset in name table
      attributes: u32,     // bit 0x10 = directory
      data_offset: u64,    // offset in data section
      unpacked_size: u64,  // uncompressed size
      packed_size: i64,    // -1 = uncompressed
  }
  ```
- [ ] Implémenter `parse_index(index_data: &mut [u8], key: &[u8; 12], version: u8, base_offset: u64) -> Vec<DxFileEntry>` :
  - Déchiffrer `index_data` en place avec `key_conv`
  - Lire la table de répertoires (structure selon version : 32-bit v5 vs 64-bit v6+)
  - Lire la table de fichiers (offset, taille, flags)
  - Construire la liste plate des entrées fichier (ignorer les entrées répertoire pour F4)
  - ⚠️ La structure exacte (taille des entrées, offsets) diffère entre v5 et v6 — vérifier
    GARbro pour les tailles exactes de struct avant d'implémenter
- [ ] Tests :
  - `test_parse_index_v5_synthetic` : TOC synthétique v5 → liste de fichiers correcte
  - `test_parse_index_v6_synthetic` : TOC synthétique v6 → liste de fichiers correcte

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::decryptor::tests::test_parse_index
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 2 tests verts, clippy vert

Commit message : `feat(wolf/decryptor): parse DXA TOC (DirTable + FileTable, v5 + v6)`

---

### Step 8 — extract_all() + gestion LZSS + tests round-trip ⚠️

**Objectif :** Implémenter la fonction principale `extract_all()` qui prend les bytes bruts
d'un `.wolf` et retourne une `WolfArchive` avec tous les fichiers déchiffrés. Gérer le
cas LZSS si nécessaire.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/decryptor.rs`

**Dépend de :** Step 2, 3, 5, 6, 7

**⚠️ LZSS — À investiguer avant d'implémenter :**
- WolfDec source indique que `packed_size > 0 && packed_size != unpacked_size` signifie LZSS
- Vérifier sur les jeux de test disponibles (月咲流ホノカ `Data.wolf` 95MB, Densyanai Inko `BasicData.wolf`
  10KB) si des entrées ont `packed_size != -1 && packed_size != unpacked_size`
- Si aucun fichier v2/v3 ciblé n'utilise LZSS : marquer `packed_size != -1` comme `Err(UnsupportedCompression)`
  et documenter — ne pas implémenter LZSS pour F4 si non nécessaire
- Si LZSS présent : porter depuis DxLib `DXArchive_MinMaxC.cpp` — noter l'effort (~1 jour supplémentaire)

Tâches :
- [ ] Implémenter `pub fn extract_all(data: &[u8]) -> Result<WolfArchive, DecryptorError>` :
  1. Lire signature → version
  2. Essayer les clés hardcodées pour cette version (Step 3)
  3. Si aucune clé connue → `guess_key_v6` ou `guess_key_v5`
  4. Lire l'en-tête (Step 4/5) avec la clé trouvée
  5. Lire et déchiffrer la TOC (Step 7)
  6. Pour chaque fichier : extraire les bytes, déchiffrer avec `key_conv(offset=unpacked_size)`
  7. Si `packed_size > 0 && packed_size != unpacked_size` :
     - Pour F4 : retourner `Err(DecryptorError::UnsupportedCompression)` avec message clair
     - ⚠️ Implémenter LZSS seulement si des jeux ciblés en ont besoin
  8. Construire et retourner `WolfArchive { version, code_page, files }`
- [ ] Tests :
  - `test_extract_all_synthetic_v5` : archive DXA v5 synthétique → WolfArchive correcte
  - `test_extract_all_synthetic_v6` : archive DXA v6 synthétique → WolfArchive correcte
  - `test_extract_all_round_trip` : chiffrer → déchiffrer → données originales
  - `test_extract_all_no_key` : archive avec clé "no_key" → déchiffrée avec la clé constante
  - `test_extract_all_bad_signature` : données aléatoires → `Err(InvalidSignature)`

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::decryptor::tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : tous les tests decryptor verts (au moins 15 en tout), clippy vert

Commit message : `feat(wolf/decryptor): extract_all() — full DXA extraction (v5/v6/v8, key table + GuessKey)`

---

## Tests obligatoires avant push GitHub

```bash
# Rust — tous les tests (decryptor + fondations F4-01)
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
git checkout -b feat/f4-02-wolf-decryptor
# ... commits intermédiaires par step ...
git checkout main
git merge --no-ff feat/f4-02-wolf-decryptor -m "feat(f4-02): Wolf RPG DXA decryptor — XOR-12, GuessKey, full extract_all()"
git push origin main
git branch -d feat/f4-02-wolf-decryptor
```

## Mise à jour après complétion

- `ROADMAP.md` : cocher `src-tauri/src/engines/wolf/decryptor.rs`
- `CHANGELOG.md` : entrée `Added` Wolf decryptor DXA v5/v6/v8
- `docs/journal/` : nouvelle entrée `YYYY-MM-DD-f4-02-wolf-decryptor.md`
- Mettre à jour la mémoire session : F4-02 terminé, passer à F4-03
- ⚠️ Documenter si LZSS a été trouvé dans les jeux de test ou non
