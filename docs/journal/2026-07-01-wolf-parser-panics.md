# Journal — 2026-07-01 — Phase 2 remédiation : paniques des parsers binaires Wolf

**Phase** : F4
**Durée estimée** : 2h
**Statut** : ✅ Complété

---

## Ce qui a été fait

Phase 2 du plan de remédiation issu de l'audit du 2026-07-01
(`docs/audit-remediation-plan-2026-07-01.md`) — 🟠 P1 crash sur `.wolf`/`.mps`/`.dat`
malformé. Méthode : **11 tests rouges prouvés avant fix** (9 paniques + 2 SIGABRT),
tous verts après. Sites vérifiés sur le code réel avant d'agir : tous encore aux
lignes exactes de l'audit.

- **`legacy_xor.rs` — racine (A)** : `huffman_decode` acceptait un `orig_size`
  jusqu'à `u64::MAX` (header 6 bits → `orig_bits=64`) → panique « capacity
  overflow » sur `vec![0u8; orig_size]`. Plafond `MAX_DECODE_OUTPUT = 1 GiB`
  partagé avec `lz_decode` (`dest_size` u32 → alloc 4 GiB possible).
- **`legacy_xor.rs` — TOC v5/v6 (B)** : `toc_data[ns..]` avec `name_offset` lu brut
  du TOC, sans borne → `Err(HeaderTooShort)` + `usize::try_from`.
- **`legacy_xor.rs` — Huffman v8 (C)** : `decoded[..huff_kb]`/`decoded[huff_kb..]`
  non gardés dans les grandes branches d'`extract_v8_huffman_only` et
  `assemble_v8_lz_stream` (un blob décodant vers un tampon vide suffisait) →
  garde `decoded.len() >= huff_kb * 2`.
- **`legacy_xor.rs` — tier arithmétique (D)** : tous les champs `DxFileEntry` sont
  des `u64` attaquants → additions d'offsets en `checked_add` + casts `u64→usize`
  en `try_from` : `:541/:554/:580/:1387` (liste audit) + sites frères du même chemin
  (`:1185/:1204/:1238/:1257/:1396/:1419`, `key_offset + huff_sz`) + helpers v8
  (`read_original_name`, `build_per_file_key_str`, `find_parent_dir`).
- **`v3_format/map.rs` (E)** : `width*height*layer_cnt*4` en multiplications `u32`
  brutes (panique debug, wrap release) → `tile_data_len()` en `checked_mul` u64,
  nouveau variant `V3FormatError::TileSizeOverflow` ; branche utf8 : `read_bytes`
  (borné, sans alloc) **avant** `Vec::with_capacity(tile_len)`.
- **`dat_parser.rs` (F)** : `read_bytes` refuse `n > bytes restants` avant
  d'allouer ; helper `bounded_cap(cursor, count)` sur les 5 `with_capacity`
  pilotés par un u32 attaquant. Rouge le plus grave du lot : un `.project` forgé
  avec `type_count = u32::MAX` **abortait tout le process** (SIGABRT, tentative
  d'allocation de 309 Go) — pas rattrapable par `catch_unwind`.
- Les `catch_unwind` d'`extractor.rs` (`:521/:841`) **non touchés** (garde-fou voulu).

## Fichiers créés

- `docs/journal/2026-07-01-wolf-parser-panics.md` — cette entrée

## Fichiers modifiés

- `src-tauri/src/engines/wolf/decrypt/legacy_xor.rs` — const `MAX_DECODE_OUTPUT`,
  bornes/`checked_add`/`try_from` sur ~15 sites, 2 helpers de test
  `patch_v5_toc`/`patch_v6_toc` (XOR symétrique), 7 tests malformed-input
- `src-tauri/src/engines/wolf/v3_format/map.rs` — `tile_data_len()`, lecture avant
  allocation (branche utf8), 1 test
- `src-tauri/src/engines/wolf/v3_format/mod.rs` — variant `TileSizeOverflow`
- `src-tauri/src/engines/wolf/dat_parser.rs` — garde `read_bytes`, `bounded_cap`,
  `saturating_mul(4)`, 3 tests
- `CHANGELOG.md` — entrée Fixed
- `tasks/todo.md` — suivi Phase 2

## Fichiers supprimés

- (aucun)

## Dépendances ajoutées

- (aucune)

## Décisions prises

- **Réutiliser `DecryptorError::HeaderTooShort`** pour toutes les nouvelles bornes
  (cohérent avec les checks voisins existants) — pas de nouveau variant côté
  décrypteur. Un seul variant ajouté côté v3 : `TileSizeOverflow` (aucun existant
  ne décrivait un overflow de dimension).
- **Plafond 1 GiB** (`MAX_DECODE_OUTPUT`) pour les sorties décompressées
  Huffman/LZ : largement au-dessus de tout fichier Wolf réel, en dessous du
  territoire OOM. Validé par les fixtures réelles (Honoka v8 passe).
- **Extension de périmètre déclarée et validée** : sites frères de la même classe
  dans le même chemin de dispatch v8 (sans eux, le wrap release restait atteignable
  au slice juste en dessous des sites listés).
- `:554` non testable en rouge (champs v5/v6 en u32 → pas de wrap possible en
  64-bit) → fix défensif sans test dédié.

## Problèmes rencontrés

- **Tests rouges à risque d'abort** : les 2 tests `dat_parser` (`type_count`,
  `fields_size` = `u32::MAX`) tuent le harness de test entier (SIGABRT) — prouvés
  rouges en exécution **isolée** (`--exact`), pas dans la suite.
- **`lz_decode` pré-fix ne paniquait pas** : il retournait `Some(vec 4 GiB)`
  (overcommit Linux) — le rouge observé est l'assertion `is_none()` qui échoue,
  pas une panique.

## Tâches ROADMAP cochées

- [ ] (aucune tâche ROADMAP formelle — remédiation audit hors backlog planifié)

## Prochaine session

- Phase 3 — 🟠 Robustesse des promesses UI (`ProjectList.tsx` `Promise.all` sans
  catch, `SegmentGrid.tsx` `handleSave` + `rowSelection` + plafond 5000 lignes).
  Note : décision de scope à prendre sur le plafond (pagination vs COUNT).

---
*Généré par Claude Code — Hoshi2Star*
