# Session — Wolf RPG v3.x LZ4 `.dat` database support (Inko)

**Date:** 2026-06-10
**Status:** Complet — branche `feat/wolf-lz4-database`

---

## Contexte

Tâche initiale : faire fonctionner l'extraction sur Densyanai Inko (Wolf RPG
Editor v3.x, label interne "v2.292", UTF-8). Diagnostic préalable a montré que
la prémisse "UTF-8 non géré" était fausse :

- `dat_parser.rs` route déjà `DB_MAGIC_UTF8` → `is_utf8 = true` correctement
  (`DataBase.dat`/`CDataBase.dat`/`SysDatabase.dat` d'Inko matchent ce magic).
- Le vrai bloqueur pour les databases : byte 10 = `0xC4`
  (`DatParseError::Unsupported("LZ4-compressed database...")`).
- `.mps` et `CommonEvent.dat` v3.x ont des incompatibilités structurelles
  séparées et plus profondes (signatures différentes, sémantique de champs
  différente) — déjà documentées dans
  `docs/references/wolfrpg-format-compatibility.md`. Hors scope ici (F5).

Décision utilisateur : implémenter **uniquement** la décompression LZ4 des
`.dat` — la pièce la plus isolée et à plus forte valeur (noms/descriptions
d'objets, compétences, personnages).

---

## Format `0xC4` (confirmé par xxd sur Inko)

```
byte  0      : indicator (0x00 = unencrypted)
bytes 1–9    : 9-byte magic (DB_MAGIC_SJIS / DB_MAGIC_UTF8)
byte  10     : version = 0xC4
bytes 11–14  : u32_le decompressed_size
bytes 15–18  : u32_le compressed_size
bytes 19..   : LZ4 block (raw block format, sans frame header), longueur == compressed_size
```

Vérifié sur les 3 fichiers Inko :
- `compressed_size == file_len - 19` exactement.
- Le bloc LZ4 décompressé (avec `decompressed_size` comme taille connue) est
  **identique octet pour octet** au contenu post-header-11-octets d'un `.dat`
  non compressé : `type_count (u32)` puis sections par type commençant par
  `DAT_TYPE_SEPARATOR`. Confirmé : `type_count` décompressé = 30 pour
  `DataBase` et 40 pour `CDataBase`, identique au `type_count` du `.project`
  correspondant.

Les fichiers `.project` ne sont **pas** compressés (header `u32 type_count, u32
mode_count` standard, identique à Honoka) — aucun changement nécessaire côté
`.project`.

---

## Implémentation

| Fichier | Changement |
|---------|------------|
| `src-tauri/Cargo.toml` | `lz4_flex = "0.11"` (pure Rust, décompression "safe") |
| `src-tauri/src/engines/wolf/dat_parser.rs` | `DAT_VERSION_LZ4 = 0xC4`, `DatParseError::Lz4Decompress`, `decompress_lz4_dat()`, `parse_database` route `0xC4` vers décompression puis `parse_dat_types` sur header(11) + payload décompressé |
| `src-tauri/src/engines/wolf/extractor.rs` | Nouveau test `test_real_inko_database_segments` ; correctif clippy `collapsible_if` préexistant dans `normalize_wolf_command_signatures` |

---

## Tests

- `test_parse_database_lz4_roundtrip` — compresse un `.dat` synthétique avec
  `lz4_flex::block::compress`, vérifie que `parse_database` produit le même
  résultat que la version non compressée.
- `test_parse_database_lz4_header_too_short` / `test_parse_database_lz4_truncated_block`
  — `Err(Io(...))`, pas de panic, sur headers/blocs LZ4 corrompus.
- `test_real_inko_database_segments` — parse les 3 vraies databases Inko.

**259/259 verts.** `cargo fmt` + `cargo clippy -D warnings` + `pnpm typecheck` OK.

---

## Résultat sur Inko (réel)

```
Inko DataBase.dat   → 415 segments
Inko CDataBase.dat  →  23 segments
Inko SysDatabase.dat →  0 segments
Total               → 438 segments
```

---

## Hors scope / reste à faire (F5)

- `.mps` (maps) : signature event `0x6f393000` vs `...22`, champ `skippable`
  avec sémantique différente (1913 sur un fichier de 763 octets) → fork de
  `wolfrpg-map-parser` ou parser custom v3.x.
- `CommonEvent.dat` : magic bytes 6/10 différents (`0x55`/`0x93` vs `0x00`/`0x8F`),
  signature per-event `0xE4` vs `0x8E` → même besoin de fork/parser custom.

Voir `docs/references/wolfrpg-format-compatibility.md` pour le détail complet.
