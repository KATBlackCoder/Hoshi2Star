# Référence binaire DXA — DxLib Archive Format

Document de référence pour l'implémentation Rust de F4-02.  
**Source primaire :** `docs/wolf-rpg-research.md` — seule source autorisée.  
Les sections marquées **⚠️** nécessitent une vérification dans GARbro `ArcFormats/DxLib/ArcDX.cs`.

---

## 1. Vue d'ensemble

**DXA (DX Archive)** est le format d'archive de DxLib (bibliothèque japonaise). Wolf RPG utilise
les versions 5, 6 et 8 de l'archiveur DxLib. Le chiffrement est un XOR symétrique à clé 12 octets
(`DXA_KEYSTR_LENGTH = 12`) — identique pour chiffrer et déchiffrer.

**Signature :** `"DX"` (`0x44 0x58`) — toujours en clair aux octets [0..2].

Layout général d'une archive :

```
[0x00 .. BaseOffset]      En-tête  (4 bytes plaintext + corps chiffré)
[BaseOffset ..]           Section données  (fichiers individuels, chiffrés)
[index_offset ..]         TOC  (table répertoires + table fichiers, chiffrée)
```

> ⚠️ Relation entre `index_offset` et `BaseOffset` non précisée dans le rapport.
> Le rapport indique `Data offset = BaseOffset + entry.data_offset` pour les fichiers.
> Vérifier dans GARbro si la TOC est à `index_offset` (absolu) ou `BaseOffset + index_offset`.

---

## 2. Structures d'en-tête

### 2.1 DXA v5 — 32-bit (Wolf RPG v1.x / v2.0 ancien)

> ⚠️ Le rapport ne donne pas le layout champ-par-champ pour v5. La structure ci-dessous est
> déduite du contexte "32-bit" (champs u32 au lieu d'i64, pas de CodePage) + plan F4-02.
> Vérifier dans GARbro struct `DxHeader` pour `version == 5`.

| Offset fichier | Taille | Champ             | Type  | Chiffré | Description                             |
|----------------|--------|-------------------|-------|---------|-----------------------------------------|
| 0x00           | 2      | `signature`       | [u8;2]| non     | `"DX"` = [0x44, 0x58]                  |
| 0x02           | 1      | `version`         | u8    | non     | `5`                                     |
| 0x03           | 1      | `flags`           | u8    | non     | `DXA_FLAG_NO_KEY` etc. (⚠️ valeur ?)   |
| 0x04           | 4      | `index_size`      | u32   | oui     | Taille TOC (non compressée)             |
| 0x08           | 4      | `base_offset`     | u32   | oui     | Offset début section données            |
| 0x0C           | 4      | `index_offset`    | u32   | oui     | Offset de la TOC                        |
| 0x10           | 4      | `file_table`      | u32   | oui     | Offset table fichiers dans la TOC       |
| 0x14           | 4      | `dir_table`       | u32   | oui     | Offset table répertoires dans la TOC    |
| **0x18**       |        |                   |       |         | **Fin — total 24 octets** (⚠️ à confirmer) |

**Pas de champ `CodePage`** → encodage Shift-JIS (932) implicite.

**Chiffrement du corps :** `key_conv(file[0x04..0x18], offset=4, key)`

**Déchiffrement TOC :** `key_conv(toc, offset = index_offset % 12, key)`

---

### 2.2 DXA v6 / v8 — 64-bit (Wolf RPG v2.x récent / v3.x)

Source : `docs/wolf-rpg-research.md §3` — *"En-tête v6 (0x2C octets, champs 64-bit)"*.

Les offsets corps (colonne **Corps**) sont relatifs à file[4], là où commence le corps chiffré.

| Offset fichier | Offset corps | Taille | Champ             | Type  | Chiffré | Description                                 |
|----------------|-------------|--------|-------------------|-------|---------|---------------------------------------------|
| 0x00           | —           | 2      | `signature`       | [u8;2]| non     | `"DX"` = [0x44, 0x58]                      |
| 0x02           | —           | 1      | `version`         | u8    | non     | `6` ou `8`                                  |
| 0x03           | —           | 1      | `flags`           | u8    | non     | `DXA_FLAG_NO_KEY` etc. (⚠️ valeur ?)       |
| 0x04           | 0x00        | 4      | `index_size`      | u32   | oui     | Taille TOC (non compressée)                 |
| 0x08           | 0x04        | 8      | `base_offset`     | i64   | oui     | Offset début section données                |
| 0x10           | 0x0C        | 8      | `index_offset`    | i64   | oui     | Offset de la TOC                            |
| 0x18           | 0x14        | 8      | `file_table`      | i64   | oui     | Offset table fichiers dans la TOC           |
| 0x20           | 0x1C        | 8      | `dir_table`       | i64   | oui     | Offset table répertoires dans la TOC        |
| 0x28           | 0x24        | 4      | `code_page`       | u32   | oui     | 0=auto, 932=SJIS, 65001=UTF-8               |
| **0x2C**       |             |        |                   |       |         | **Fin — total 44 octets**                   |

**Note CodePage :** offset corps 0x24 = file offset 0x28. Le plan F4-02 précise
*"v8 → offset 0x28 confirmé empiriquement sur Honoka"* et *"v6 → vérifier corps 0x24"* —
ce sont la même position. ⚠️ Si v6 et v8 diffèrent, adapter la lecture selon la version.

**Chiffrement du corps :** `key_conv(file[0x04..0x2C], offset=4, key)`

**Déchiffrement TOC :** `key_conv(toc, offset=0, key)` (v6+ — key_pos repart de 0)

**Octets file[0x2C..0x30] :** situés après l'en-tête formel, valeur `0x00` en clair.
Utilisés par `GuessKeyV6` comme deuxième point de validation (voir §4).

---

## 3. Algorithme KeyConv

**Source :** DxLib `DXArchive::KeyConv` / GARbro `ArcDX.cs Decrypt`.

```c
// C original (DxLib)
void DXArchive::KeyConv(void *Data, int Size, int Position, unsigned char *Key) {
    Position %= DXA_KEYSTR_LENGTH;   // 12
    int j = Position;
    for (int i = 0; i < Size; i++) {
        ((u8*)Data)[i] ^= Key[j];
        if (++j == DXA_KEYSTR_LENGTH) j = 0;
    }
}
```

```rust
/// XOR `data` in place with `key`, starting at key_pos = offset % 12.
/// Symmetric: applying twice restores original. Used for both encrypt and decrypt.
///
/// `offset` = position de data[0] dans l'archive DXA (détermine le point de départ dans la clé).
pub(crate) fn key_conv(data: &mut [u8], offset: u64, key: &[u8; 12]) {
    let mut pos = (offset % 12) as usize;
    for byte in data.iter_mut() {
        *byte ^= key[pos];
        pos += 1;
        if pos == 12 { pos = 0; }
    }
}
```

### 3.1 Valeur d'`offset` selon le contexte

| Données déchiffrées         | Valeur d'`offset` passée à `key_conv`                     |
|-----------------------------|-----------------------------------------------------------|
| En-tête — corps (file[4..]) | `4`  (la signature 4-byte précède dans le fichier)         |
| TOC, version ≤ 5            | `index_offset % 12`  (position archive de la TOC)         |
| TOC, version 6+             | `0`  (key_pos repart de zéro)                             |
| Données fichier individuel  | `unpacked_size`  (**bug Wolf RPG** — voir §3.2)            |

### 3.2 Bug Wolf RPG — offset des données fichier

> **⚠️ CRITIQUE.** Pour chaque fichier dans l'archive, l'offset passé à `key_conv` est
> la **taille décompressée du fichier** (`unpacked_size`), **pas** sa position dans l'archive.

Citation exacte du rapport §3 :

> *"le décalage de clé = `(UnpackedSize % 12)`. La position dans l'archive n'a aucune
> importance. GARbro implémente ce comportement : pour version > 5, `dec_offset = dx_ent.UnpackedSize`."*

```rust
// Wolf RPG bug: file data decryption offset = unpacked_size % 12
// NOT the file position in the archive. See: docs/wolf-rpg-research.md §3
key_conv(&mut file_data, entry.unpacked_size, key);
```

---

## 4. Algorithme GuessKeyV6

**Source :** GARbro `GuessKeyV6` + code Python de himeworks, via `docs/wolf-rpg-research.md §3`.

**Principe :** Les champs 64-bit de l'en-tête ont leurs 4 octets supérieurs nuls en clair
(valeurs pratiques < 4 Go). `XOR(chiffré, 0x00) = octet de clé`.

### 4.1 Positions clés dans l'en-tête chiffré

Corps chiffré avec `key_conv(offset=4)` → pour body byte `i`, `key_pos = (4 + i) % 12`.
En termes de position fichier P : `key_pos = P % 12`.

| Champ                        | Octets HIGH  | Pos. fichier | `key_pos`   | Donne        |
|------------------------------|--------------|--------------|-------------|--------------|
| `base_offset` — octets hauts | file[0x0C..0x10] | 12..15   | 0, 1, 2, 3  | key[0..4]    |
| `index_offset` — octets hauts| file[0x14..0x18] | 20..23   | 8, 9, 10, 11| key[8..12]   |
| `file_table` — octets hauts  | file[0x1C..0x20] | 28..31   | 4, 5, 6, 7  | key[4..8]    |
| `dir_table` — octets hauts   | file[0x24..0x28] | 36..39   | 0, 1, 2, 3  | key[0..4] ← **même** |
| post-en-tête (zéros)         | file[0x2C..0x30] | 44..47   | 8, 9, 10, 11| key[8..12] ← **même** |

Les deux colonnes **même** servent de validation : deux champs distincts utilisant les mêmes
positions de clé doivent produire des bytes identiques s'ils sont tous deux nuls en clair.

### 4.2 Code Python du rapport (référence exacte)

```python
# header = premiers bytes du fichier (minimum 0x30 bytes)
if (header[0xC:0x10] == header[0x24:0x28]) and (header[0x14:0x18] == header[0x2C:0x30]):
    key = header[0xC:0x10] + header[0x1C:0x20] + header[0x14:0x18]
```

Décomposé :

```
key[0..4]  = file[0x0C..0x10]  — octets HIGH de base_offset
key[4..8]  = file[0x1C..0x20]  — octets HIGH de file_table
key[8..12] = file[0x14..0x18]  — octets HIGH de index_offset

Validation 1 : file[0x0C..0x10] == file[0x24..0x28]  (base_offset high == dir_table high)
Validation 2 : file[0x14..0x18] == file[0x2C..0x30]  (index_offset high == post-header zeros)
```

### 4.3 Pseudo-Rust

```rust
pub fn guess_key_v6(data: &[u8]) -> Option<[u8; 12]> {
    if data.len() < 0x30 {
        return None;
    }
    let read4 = |pos: usize| -> [u8; 4] { data[pos..pos + 4].try_into().unwrap() };

    let high_base  = read4(0x0C); // → key[0..4]
    let high_idx   = read4(0x14); // → key[8..12]
    let high_ftbl  = read4(0x1C); // → key[4..8]
    let high_dtbl  = read4(0x24); // doit == high_base  (validation 1)
    let post_hdr   = read4(0x2C); // doit == high_idx   (validation 2)

    if high_base != high_dtbl || high_idx != post_hdr {
        return None;
    }

    let mut key = [0u8; 12];
    key[0..4].copy_from_slice(&high_base);
    key[4..8].copy_from_slice(&high_ftbl);
    key[8..12].copy_from_slice(&high_idx);
    Some(key)
}
```

> ⚠️ GARbro ajoute une validation supplémentaire après dérivation : déchiffrer l'en-tête
> avec la clé candidate et vérifier que `version ∈ {5, 6, 8}` ET `index_size < 0x1000000`.
> Implémenter cette validation pour éviter les faux positifs.

---

## 5. Structure de la TOC

### 5.1 Layout mémoire de la TOC

```
TOC[0 .. file_table]          Table de noms  (chaînes nul-terminées, SJIS ou UTF-8)
TOC[file_table ..]            Entrées fichier  (DxFileEntry × N)
TOC[dir_table ..]             Entrées répertoire  (DxDirEntry × M)
```

`file_table` et `dir_table` sont les offsets lus dans l'en-tête DXA — ils sont
**relatifs au début des données TOC** (pas au fichier).

### 5.2 DxFileEntry — entrée fichier

**Source rapport :** *"Entrée fichier = {name offset, attributs (bit 0x10 = répertoire),
data offset, unpacked size, packed size (-1 si non compressé)}"*.  
**Source rapport :** *"Entrée répertoire = 0x40 octets en v6"*.

> ⚠️ La taille exacte des **entrées fichier** n'est pas dans le rapport (seule la taille des
> entrées répertoire est mentionnée — 0x40 pour v6). Les tailles ci-dessous proviennent d'une
> recherche séparée dans GARbro (à confirmer). Le rapport ne liste pas non plus les champs
> timestamp — leur présence et taille sont à vérifier.

**DxFileEntry v5 (32-bit) — ⚠️ taille totale à confirmer dans GARbro, estimée 0x2C :**

| Offset | Taille | Champ           | Type | Description                               |
|--------|--------|-----------------|------|-------------------------------------------|
| 0x00   | 4      | `name_offset`   | u32  | Offset dans la table de noms (TOC[0..])   |
| 0x04   | 4      | `attributes`    | u32  | Bit 0x10 = répertoire (ignorer)           |
| 0x08   | 24     | (timestamps)    | —    | ⚠️ Champs horodatage — vérifier GARbro   |
| 0x20   | 4      | `data_offset`   | u32  | Offset dans la section données            |
| 0x24   | 4      | `unpacked_size` | u32  | Taille décompressée                       |
| 0x28   | 4      | `packed_size`   | i32  | Taille compressée ; -1 = non compressé   |
| **0x2C** |      |                 |      | **Fin** ⚠️                                |

**DxFileEntry v6/v8 (64-bit) — ⚠️ taille totale à confirmer dans GARbro, estimée 0x40 :**

| Offset | Taille | Champ           | Type | Description                               |
|--------|--------|-----------------|------|-------------------------------------------|
| 0x00   | 8      | `name_offset`   | i64  | Offset dans la table de noms (TOC[0..])   |
| 0x08   | 8      | `attributes`    | u64  | Bit 0x10 = répertoire (ignorer)           |
| 0x10   | 24     | (timestamps)    | —    | ⚠️ Champs horodatage — vérifier GARbro   |
| 0x28   | 8      | `data_offset`   | i64  | Offset dans la section données            |
| 0x30   | 8      | `unpacked_size` | i64  | Taille décompressée                       |
| 0x38   | 8      | `packed_size`   | i64  | Taille compressée ; -1 = non compressé   |
| **0x40** |      |                 |      | **Fin** ⚠️                                |

**Nombre d'entrées fichier (v6, si taille = 0x40 confirmée) :**
```
n_files = (dir_table - file_table) / 0x40
```

**Adresse absolue des données d'un fichier :**
```
absolute_pos = base_offset + entry.data_offset
```

---

## 6. Algorithme extract_all — séquence complète

```
1. file[0x00..0x02] == "DX" ?  Non → Err(InvalidSignature)

2. version = file[0x02]
   version ∈ {5, 6, 8} ?  Non → Err(UnsupportedVersion(v))

3. Essai des clés hardcodées (WOLF_KEYS) :
   Pour chaque clé candidate :
     - Copier file[0x04..0x2C] dans buf
     - key_conv(&mut buf, offset=4, clé)
     - Déchiffrer → lire code_page, index_size
     - Valide si : code_page ∈ {0, 932, 65001} ET index_size raisonnable
     → Si valide : utiliser cette clé, passer à 5.

4. Aucune clé connue → GuessKeyV6(file[0..0x30])
   Résultat Option<[u8; 12]>
   None → Err(CannotGuessKey)

5. Déchiffrer l'en-tête complet avec la clé.
   Lire : base_offset, index_offset, index_size, file_table, dir_table, code_page.

6. Lire et déchiffrer la TOC :
   toc = file[index_offset .. index_offset + ???]  ← ⚠️ compressé ou non ?
   offset_toc = if version <= 5 { index_offset % 12 } else { 0 }
   key_conv(&mut toc, offset=offset_toc, key)

7. Parser la TOC :
   - Entrées fichier : TOC[file_table ..]
   - Ignorer les entrées où attributes & 0x10 != 0 (répertoires)

8. Pour chaque entrée fichier :
   a. data = file[base_offset + entry.data_offset
                  .. base_offset + entry.data_offset + packed_or_unpacked_size]
   b. key_conv(&mut data, offset=entry.unpacked_size, key)
      // Wolf RPG bug : offset = unpacked_size, pas la position archive
   c. Si packed_size != -1 ET packed_size != unpacked_size :
      → LZSS compressé : Err(UnsupportedCompression)  [scope F4]
      ⚠️ Vérifier si les jeux de test contiennent des entrées LZSS avant d'implémenter
   d. Nom : TOC[entry.name_offset] → chaîne nul-terminée
      Si code_page == 65001 → UTF-8
      Sinon → Shift-JIS (encoding_rs::SHIFT_JIS.decode)

9. Retourner WolfArchive { version, code_page: Some(code_page), files }
```

---

## 7. Table des clés connues (WOLF_KEYS)

Source : table `DECRYPT_MODES` de WolfDec (`docs/wolf-rpg-research.md §3`).

| Identifiant | Clé (hex 12 bytes)                               | Notes                                          |
|-------------|--------------------------------------------------|------------------------------------------------|
| `v2.20`     | `38 50 40 28 72 4F 21 70 3B 73 35 38`            | La plus répandue. ASCII : `8P@(rO!p;s58`       |
| `v2.01`     | `0F 53 E1 3E 04 37 12 17 60 0F 53 E1`            |                                                |
| `v2.10`     | `4C D9 2A B7 28 9B AC 07 3E 77 EC 4C`            |                                                |
| `v2.255`    | `b8 58 8c 7b ca 3d 6f 3d 8c 34 f8 1a`            | 月咲流ホノカ — stockée en clair à file[BaseOffset..+12] |
| `no_key`    | `55 AA 20 55 55 06 55 AA 55 D5 7C 66`            | `DXA_FLAG_NO_KEY` → clé constante DxLib        |

> ⚠️ Wolf v3.10 / v3.173 : le rapport mentionne *"clés plus longues (40-46 octets)"* dans WolfDec.
> Ambiguïté : sont-ce des mots de passe bruts AVANT `keyCreate()` (→ clé XOR reste 12 octets)
> ou les clés XOR elles-mêmes (→ `DXA_KEYSTR_LENGTH` aurait changé) ?
> **Ne pas implémenter v3.10/v3.173 avant lecture de WolfDec `main.cpp`.**

---

## 8. Flags DXA

| Flag                    | Description                                                      |
|-------------------------|------------------------------------------------------------------|
| `DXA_FLAG_NO_KEY`       | Clé constante `55AA2055550655AA55D57C66` (voir §7)               |
| `DXA_FLAG_NO_HEAD_PRESS`| En-tête/TOC non compressée (désactive LZSS pour la TOC)          |

> ⚠️ Les valeurs numériques exactes des flags ne sont pas dans le rapport.
> Vérifier dans GARbro ou DxLib source.

---

## 9. Encodage du texte

| `code_page` | Valeur | Encodage  | Wolf RPG |
|-------------|--------|-----------|----------|
| 932         | 932    | Shift-JIS | v1 / v2  |
| 65001       | 65001  | UTF-8     | v3+      |
| 0           | 0      | auto      | supposer SJIS (932) |

```rust
// Shift-JIS → String UTF-8 (crate encoding_rs, déjà dans Cargo.toml)
let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(raw_bytes);
let name = decoded.into_owned();
```

---

## 10. Cas particuliers documentés

### 月咲流ホノカ ver1.03 — DXA v8, ~90 MB

- En-tête **non chiffré** (lisible directement en clair)
- Clé `v2.255` stockée en clair à `file[BaseOffset .. BaseOffset + 12]`
- `CodePage = 932` (Shift-JIS)
- Layout : `Data.wolf` unique à la racine (⚠️ Pas `Data/*.wolf`)
- `GuessKeyV6` : échoue (header non chiffré → bytes hauts = octets de clé réels, pas des zéros)

### Densyanai Inko ver2.0 — DXA v8, ~10 MB

- En-tête **chiffré**
- Clé inconnue → `GuessKeyV6` **échoue** : la validation
  `file[0x0C..0x10] == file[0x24..0x28]` ne passe pas sur toutes les archives
- Clé à investiguer (UberWolf / OllyDbg)
- Layout : `Data/BasicData.wolf` (multi-archives)

---

## 11. Points ouverts avant implémentation

| # | Question                                                    | Où vérifier                  |
|---|-------------------------------------------------------------|------------------------------|
| 1 | `index_offset` : absolu ou relatif à `base_offset` ?        | GARbro `ReadIndex`           |
| 2 | TOC compressée (LZSS) dans les jeux de test ?               | Hex-dump Honoka + Densyanai  |
| 3 | Taille exacte `DxFileEntry` v5 (0x2C ?)                     | GARbro `DxEntry` v5          |
| 4 | Taille exacte `DxFileEntry` v6 (0x40 ?)                     | GARbro `DxEntry` v6          |
| 5 | Différence structurelle v6 vs v8 (CodePage au même offset ?)| GARbro `ReadArcHeaderV6`     |
| 6 | Valeurs numériques des flags DXA                            | GARbro / DxLib source        |
| 7 | `GuessKeyV5` existe dans GARbro ?                           | GARbro `GuessKey`            |
| 8 | Clé Densyanai Inko — origine ?                              | UberWolf / OllyDbg           |
| 9 | Clés v3.10/v3.173 : mot de passe brut ou clé XOR ?         | WolfDec `main.cpp`           |
