# Session — Wolf RPG `CommonEvent.dat` v2.x signatures + v3.x LZ4/UTF-8 (Honoka + Inko)

**Date:** 2026-06-10
**Status:** Complet — branche `feat/wolf-v3-common-events`

---

## Contexte

Suite de `2026-06-10-fix-wolf-lz4-database.md`. Deux échecs distincts sur
`CommonEvent.dat` :

- **Honoka (v2.225)** : panique sur `Command::parse` avec
  `0x04D20000`/`0x09D20000` ("Unknown command code") — variantes de
  CallCommonEvent jamais enregistrées dans `wolfrpg-map-parser` 0.6.0.
- **Inko (v2.292, en réalité un build v3.x)** : `COMMON_EVENTS_MAGIC` (byte6/byte10
  = `0x00`/`0x8F`) ne matche pas le header d'Inko (`0x55`/`0x93`) ; signature
  per-event `0x8E` vs (supposément) `0xE4` ; chaînes UTF-8 et non Shift-JIS.

Décisions utilisateur (3 `AskUserQuestion`) :
- Inclure les signatures D2 manquantes (Honoka).
- `.mps` v3.x hors scope (déjà documenté, F5).
- Réintroduire `[patch.crates-io]` git + pousser une branche sur le fork
  `KATBlackCoder/wolfrpg-map-parser`.

---

## Fork : `KATBlackCoder/wolfrpg-map-parser`, branche `fix/wolf-v3-format`

Basée sur `origin/fix/unknown-call-event-variants` (déjà existante sur le
fork), qui ajoute `CallEvent4-7` (`0x04D20000`/`0x08D20000`/`0x09D20000`/
`0x0AD20000`) + `dispatch_unknown_by_type` (route générique `0x??D20000` →
`parse_call_common_event`, `0x??D30000` → `parse_reserve_common_event`). Cette
base **résout déjà Honoka** — aucune signature supplémentaire nécessaire.

### Découverte clé : Inko n'a pas de signature `0xE4`

L'hypothèse initiale (`EVENT_SIGNATURE_V3 = 0xE4`) était fausse. Le vrai
problème : **`CommonEvent.dat` v3.x est un bloc LZ4** (même format que les
`.dat` de databases vu dans la session précédente) :

```
bytes 0–10  : header 11 octets (byte6=0x55, byte10=0x93 pour v3.x)
bytes 11–14 : u32_le decompressed_size
bytes 15–18 : (taille ~compressée, non utilisée — voir piège ci-dessous)
bytes 19..EOF : bloc LZ4 brut → decompressed_size octets
```

Une fois décompressé, le payload est **structurellement identique à v2.x** :
`event_count (u32)` puis events avec signature `0x8E` (la même qu'en v2.x).
Seul l'encodage des chaînes change (UTF-8 au lieu de Shift-JIS). Vérifié sur
Inko : décompression → exactement 4 790 657 octets, `event_count = 1251`,
premier `event_name` décodé = `"○アイテム増減"` (texte japonais valide).

**Piège** : contrairement à `dat_parser.rs` (databases `0xC4`), le champ à
bytes 15–18 ne correspond **pas** à `compressed_size` au sens
`block_end = 19 + compressed_size` (donnerait 930 039 > 929 527 = taille
fichier réelle). La décompression fonctionne en passant `bytes[19..]` (jusqu'à
EOF) tel quel à `lz4_flex::block::decompress` avec `decompressed_size` connu.

### Changements appliqués au fork

| Fichier | Changement |
|---------|------------|
| `Cargo.toml` | `lz4_flex = "0.11"` en dépendance |
| `src/db_parser.rs` | `COMMON_EVENTS_MAGIC` (constante) → `check_common_events_magic(header) -> bool` (retourne `is_v3`), valide les deux headers v2.x/v3.x |
| `src/byte_utils.rs` | `thread_local! UTF8_MODE: Cell<bool>` + `set_utf8_mode()` ; `as_string` décode UTF-8 si actif, sinon Shift-JIS (inchangé) |
| `src/db_parser/parsers/common_events_parser.rs` | `parse_bytes` : si `is_v3`, décompresse le bloc LZ4 (`decompress_v3`) puis parse `event_count` + events sur le payload décompressé ; sinon comportement v2.x inchangé |
| `src/map.rs` | Fix `clippy::doc_overindented_list_items` préexistant (bloquait `-D warnings` sur tout le crate) |
| `src/db_parser/models/common_event.rs` | Aucun changement net (la tentative `EVENT_SIGNATURE_V3=0xE4` a été retirée — `0x8E` suffit pour v2.x et v3.x) |

Push : `https://github.com/KATBlackCoder/wolfrpg-map-parser/tree/fix/wolf-v3-format`

---

## Hoshi2Star

`src-tauri/Cargo.toml` :
```toml
[patch.crates-io]
wolfrpg-map-parser = { git = "https://github.com/KATBlackCoder/wolfrpg-map-parser", branch = "fix/wolf-v3-format" }
```

`src-tauri/src/engines/wolf/extractor.rs` :
- `test_real_honoka_common_events_known_failure` → **`test_real_honoka_common_events`**
  (passe maintenant, assert `Ok` + non-vide).
- `test_real_inko_common_events_known_failure` : reste `known_failure`, message
  mis à jour.

---

## Résultats

```
Honoka CommonEvent.dat → 2195 segments  ✅
Inko   CommonEvent.dat → header + UTF-8 + LZ4 décodés ✅, mais command parsing
                          panique encore : "Unknown command code 00062c01"
```

**259/259 tests verts.** `cargo fmt` (diff uniquement sur fichiers touchés) +
`cargo clippy -D warnings` + `pnpm typecheck` OK.

---

## Inko : nouveau blocage (hors scope, F5+)

Une fois le payload v3.x décompressé, le layout par event/header/`event_name`
est correct (vérifié octet par octet). Mais la commande `Comment` (signature
`0x01670000`) avec une chaîne vide est suivie d'**un octet `0x00`
supplémentaire** que le parseur v2.x ne consomme pas, ce qui décale la lecture
de la signature suivante d'un octet : `0x062c0100` (`CallEventByName1`) est lu
comme `0x00062c01` → "Unknown command code".

Portée inconnue (peut affecter d'autres types de commandes v3.x) — décision
utilisateur : ne pas creuser cette session, garder
`test_real_inko_common_events_known_failure` avec un message à jour.

---

## Hors scope / reste à faire (F5)

- `.mps` v3.x maps (signature event, layout layers) — voir
  `docs/references/wolfrpg-format-compatibility.md`.
- Inko `CommonEvent.dat` : layout par commande v3.x (octet supplémentaire après
  `ShowText`/`Comment` avec chaîne vide, et potentiellement d'autres).
- Pas de PR ouverte vers `G1org1owo/wolfrpg-map-parser` upstream — option future.
