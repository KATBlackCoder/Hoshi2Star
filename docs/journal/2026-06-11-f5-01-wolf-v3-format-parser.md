# Journal — 2026-06-11 — F5-01 : parser maison Wolf RPG v3.x (`v3_format/`)

**Phase** : F5
**Durée estimée** : ~1 journée (plusieurs sessions)
**Statut** : ✅ Complété

---

## Ce qui a été fait

- Abandon de l'approche fork-surgery sur `wolfrpg-map-parser` pour Inko
  (Wolf RPG Editor v3.x, label interne "v2.292") — le modèle récursif
  `Page`/`EventCommand` de la crate est incompatible avec le modèle plat
  `Command::Init` de v3.x.
- Implémentation d'un parser maison `src-tauri/src/engines/wolf/v3_format/`,
  porté byte-pour-byte depuis le modèle plat de WolfTL (MIT,
  github.com/Sinflower/WolfTL) :
  1. `coder.rs` — primitives de lecture/écriture (entiers LE, strings
     UTF-8/Shift-JIS, etc.)
  2. `command.rs` — frame `Command::Init`/`Dump` plate, partagée maps +
     common events
  3. `map.rs` — `.mps` v3.x (header + events + commandes plates), avec
     décision-gate round-trip byte-exact validé sur les vrais fichiers Inko
  4. `compression.rs` — décompression/recompression LZ4 générique
     (`decompress_block`/`recompress_block`, header de taille variable selon
     `.mps` 25 octets ou `CommonEvent.dat` 11 octets)
  5. Câblage `extract_map_segments` pour v3.x
  6. Câblage `inject_map` pour v3.x
  7. `common_events.rs` — `CommonEvent.dat` v3.5 (header LZ4 11 octets +
     événements + commandes plates via `command.rs`), réutilise
     `compression::decompress_block`/`recompress_block`
  8. Validation e2e + nettoyage docs + suppression du spike d'exploration
- Round-trip byte-exact (`parse(bytes).dump() == bytes`) validé sur les 4
  maps réelles d'Inko + son `CommonEvent.dat` (1251 events, version 0x93).
- Extraction câblée : `.mps` Inko (TitleMap → 8 segments) et `CommonEvent.dat`
  Inko (1585 segments depuis 1251 events).
- Réécriture de `docs/references/wolfrpg-format-compatibility.md` :
  remplacement de l'ancienne section "6 incompatibilités v2.292" (analyse
  erronée — comparait des octets de header LZ4 à des constantes v2.x) par la
  description du vrai format (header LZ4 versionné + modèle plat) et de la
  solution `v3_format/`.
- Mise à jour `ROADMAP.md` (section F5 → "[~] En cours", items F5-01 cochés).
- Ajout d'une entrée dans `tasks/lessons.md` (2026-06-11) sur l'abandon du
  fork-surgery et la règle "v3.x = parser maison, v2.x = wolfrpg_map_parser,
  ne pas unifier".

## Fichiers créés

- `src-tauri/src/engines/wolf/v3_format/mod.rs`
- `src-tauri/src/engines/wolf/v3_format/coder.rs`
- `src-tauri/src/engines/wolf/v3_format/command.rs`
- `src-tauri/src/engines/wolf/v3_format/compression.rs`
- `src-tauri/src/engines/wolf/v3_format/map.rs`
- `src-tauri/src/engines/wolf/v3_format/common_events.rs`

## Fichiers modifiés

- `src-tauri/src/engines/wolf/extractor.rs` — câblage `extract_map_segments`
  et `extract_common_events` vers `v3_format` quand `is_lz4_v3(bytes)`,
  nouvelle fonction `extract_common_events_v3`, nouveaux tests
  `test_real_inko_maps_v3*` / `test_real_inko_common_events_v3*` (remplacent
  les anciens tests `*_known_failure`)
- `src-tauri/src/engines/wolf/injector.rs` — câblage `inject_map` pour v3.x
- `docs/references/wolfrpg-format-compatibility.md` — réécriture des
  sections Inko/v3.x (cause réelle + solution `v3_format/`), section Honoka
  v2.225 conservée telle quelle
- `ROADMAP.md` — section F5 : statut "[~] En cours", items F5-01 cochés
- `tasks/lessons.md` — entrée 2026-06-11 (fork-surgery abandonné →
  parser maison)

## Fichiers supprimés

- `src-tauri/examples/wolf_mps_spike.rs` (spike d'exploration Phase 0, devenu
  obsolète une fois sa logique portée dans `v3_format/`)
- répertoire `src-tauri/examples/` (devenu vide)

## Dépendances ajoutées

- (aucune nouvelle — `lz4_flex` déjà présent depuis la session LZ4 .dat)

## Décisions prises

- v3.x (Inko) vit dans un module maison `v3_format/`, indépendant de
  `wolfrpg_map_parser` ; v2.x (Honoka) reste sur `wolfrpg_map_parser` (fork)
  — pas d'unification des deux modèles.
- `compression.rs` expose des fonctions génériques `decompress_block`/
  `recompress_block` paramétrées par `header_size`, réutilisées par `.mps`
  (25 octets) et `CommonEvent.dat` (11 octets).
- `CommonEvent.dat` v3.x : extraction + injection (recompress) implémentées,
  `recompress`/`dump`/`is_utf8`/`is_v35` marqués `#[allow(dead_code)]` là où
  l'injecteur n'est pas encore câblé, par parité avec les précédents
  (`dat_parser.rs`, `encoding.rs`).

## Problèmes rencontrés

- Dérivation du layout `CommonEvent.dat` v3.5 (header 11 octets, byte 0 =
  `0x00`, magic 9 octets avec flag UTF-8 à l'index 5, version `0x93`/`0xCC`
  → mode v3.5/LZ4 `Unpack`) via hex-dump croisé avec `FileCoder.hpp` et
  `CommonEvents.hpp` de WolfTL.
- 3 erreurs clippy "dead code" après création de `common_events.rs`
  (`recompress`, `CommonEventV3::dump`, `CommonEventsV3::is_utf8/is_v35/dump`)
  — résolues par `#[allow(dead_code)]` documenté, suivant le précédent du
  projet.

## Tâches ROADMAP cochées

- [x] F5-01 `src-tauri/src/engines/wolf/v3_format/` — parser maison
      `.mps`/`CommonEvent.dat` v3.x (LZ4 + modèle plat `Command::Init`),
      round-trip byte-exact validé sur les 4 maps + le `CommonEvent.dat`
      réels d'Inko (v2.0)
- [x] Tests sur jeux Wolf v3.x réels (Inko v2.0 : maps + CommonEvent.dat)

## Prochaine session

- F5 reste "[~] En cours" : `decryptor_v3.rs` (support WolfX, hash-based,
  UberWolf v3.5+) et documentation du format WolfX dans `docs/engines.md`
  restent à faire.
- Vérification finale effectuée cette session : `pnpm typecheck` ✅,
  `cargo test` (299 passed / 0 failed / 4 ignored) ✅, `cargo clippy
  -- -D warnings` ✅ (vérifié en amont).

---
*Généré par Claude Code — Hoshi2Star*
