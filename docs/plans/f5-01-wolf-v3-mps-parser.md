# Plan F5-01 — Wolf RPG v3.x (Inko) : parser `.mps`/`CommonEvent.dat` maison

## Objectif

Remplacer la dépendance à `wolfrpg-map-parser` (fork `KATBlackCoder/wolfrpg-map-parser`,
branche `fix/wolf-v3-format`) **pour le format v3.x (WOLF RPG Editor ≥ v3.00, jeu Inko)**
par un parser/writer maison, écrit en interne dans `src-tauri/src/engines/wolf/v3_format/`,
basé sur le **modèle réel de WolfTL** (référence C++ fonctionnelle) plutôt que sur le
modèle arborescent de `wolfrpg-map-parser` qui s'est révélé incompatible avec le format
v3.5+.

Le format v2.x (Honoka, `wolfrpg-map-parser` non-fork ou fork sans `V35_MODE`) **n'est pas
touché** par ce plan — il continue de fonctionner tel quel.

---

## Contexte — pourquoi on abandonne la chirurgie sur le fork

### Ce qui a été tenté et pourquoi ça bloque (sessions précédentes)

Le plan précédent (`polished-munching-lantern.md`, Phase 0) proposait de décompresser le
LZ4, "repacker" l'en-tête pour le faire correspondre au format attendu par
`wolfrpg-map-parser` 0.6, puis appeler `Map::parse` tel quel. Le repack d'en-tête a
fonctionné (validé sur `SampleMap.mps`), mais `Map::parse` panique ensuite **dans les
commandes des events**, sur des "Unknown command code" différents par fichier.

Root cause identifiée : `wolfrpg-map-parser` modélise les commandes "conteneurs" (boucle
`LoopCount`, branches, choix) comme un **arbre récursif** — `LoopCount { loop_count, commands:
Vec<Command> }` avec une signature de fin (`LOOP_END_SIGNATURE`) lue après les commandes
imbriquées. Le format v3.5+ ajoute un champ `v35Unknown` (1 octet de taille + N octets)
**après chaque commande, de façon plate et uniforme** (`Command::s_v35` dans WolfTL). Ce
champ ne se compose pas proprement avec le modèle récursif : un patch de `LoopCount`
(`/tmp/wolfrpg-map-parser/src/command/event_control_command/loop_count.rs`) a permis de
parser un `LoopCount` complet, mais a immédiatement révélé 4 nouveaux panics différents (un
par carte testée) sur d'autres types de conteneurs (branches `01000000`/`Exit`, choix
`0579`/`05d2`/`06d2`/`04d2`, etc.). Chaque type de conteneur nécessiterait son propre
correctif ad hoc — travail ouvert, non borné.

### Ce que WolfTL révèle : le format réel est PLAT, pas arborescent

Lecture de `/home/blackat/project/test/WolfTL-main/WolfTL/WolfRPG/{Command.hpp,Map.hpp,
RouteCommand.hpp}` (référence C++ qui fonctionne sur des fichiers v3.5+ réels) :

- **`Page::Init`** (`Map.hpp:72-80`) : `commandCount = ReadInt()`, puis une simple boucle
  `for i in 0..commandCount { Command::Init(coder) }`. **Aucune récursion.** La hiérarchie
  boucle/branche est encodée via le champ `m_indent` (indentation), pas via imbrication
  binaire.
- **`Command::Init`** (`Command.hpp:706-764`) : frame **générique**, identique pour
  `StartLoop`, `LoopEnd`, `ChoiceCase`, `Message`, etc. :
  ```
  argsCount   = ReadByte() - 1
  cid         = ReadInt()              // u32 — 101=Message, 102=Choices, 170=StartLoop, 498=LoopEnd, ...
  args[0..argsCount] = ReadInt() chacun
  indent      = ReadByte()
  strCount    = ReadByte()
  stringArgs[0..strCount] = ReadString() chacun   // u32 size + bytes (UTF-8 ou SJIS selon flag global)
  terminator  = ReadByte()             // 0x00 normal, 0x01 = Move (route embarquée, cf RouteCommand)
  if s_v35:                            // version >= 0x67, FLAG GLOBAL, posé une seule fois
      unknownSize = ReadByte()
      v35Unknown  = Read(unknownSize)  // 0 à 255 octets, à préserver tel quel pour le round-trip
  ```
- **`Map::load`** (`Map.hpp:496-550`) : header déjà validé par le spike (`version`,
  `unknown2`, `unknown3` (string), `tileset`, `width`, `height`, `eventCount`, puis si
  `version >= 0x67` : `unknown4` + `layerCnt` (→ pose `s_v35 = true`), puis tuiles
  (`width*height*layerCnt*4` octets, **absentes** si UTF-8 et marqueur `-1`), puis une
  boucle `while ReadByte() == EVENT_INDICATOR { Event::Init }` jusqu'au `TERMINATOR`.
- **`Event::Init`** (`Map.hpp:315-348`) : magic 4 octets, `id`, `name` (string), `x`, `y`,
  `pageCount`, magic 4 octets, puis boucle `while ReadByte() == 0x79 { Page::Init }`.
- **`Page::Init`** (`Map.hpp:44-96`) : `unknown1`, graphique (string + 4 bytes), conditions
  (37 bytes) + movement (4 bytes), `flags`, `routeFlags`, `routeCount` + N×`RouteCommand`
  (route "non-déclenchée" de la page), `commandCount` + N×`Command::Init` (la boucle
  plate ci-dessus), `features`, 3 bytes (shadow/collision), `pageTransfer` optionnel si
  `features > 3`, terminateur `0x7A`.
- **`RouteCommand::Init`** (`RouteCommand.hpp:37-48`) : `id` (byte), `argCount` (byte) +
  N×`ReadInt`, magic 2 bytes (`0x01 0x00`).

**Conséquence** : porter ce modèle plat est un travail **borné** — un seul lecteur
générique de "frame de commande" + un seul cas spécial (`Move`/route, lui-même générique
via `RouteCommand`). On n'a pas besoin de connaître la sémantique de chaque `cid` : pour
l'extraction CAT, seuls comptent `cid` (pour repérer `Message=101`/`Choices=102`),
`stringArgs` (texte à traduire) et la capacité à **réécrire** la frame à l'identique
(round-trip byte-exact, `v35Unknown` préservé en bytes opaques).

### Bonus : injection symétrique

Avec un parser maison, `dump()` est l'inverse exact de `parse()` — l'injection devient
"parser → remplacer les `stringArgs` des commandes `Message`/`Choices` ciblées → dump →
recompresser LZ4", sans le scan binaire séquentiel fragile (`patch_mps_strings`,
Approche B) utilisé aujourd'hui pour le v2.x.

---

## Statut : [ ] À démarrer

## Prérequis

- `Cargo.toml` `[patch.crates-io]` revenu à
  `wolfrpg-map-parser = { git = "...", branch = "fix/wolf-v3-format" }` (fait, vérifié via
  `cargo check`).
- Aucun changement requis sur `/tmp/wolfrpg-map-parser` (fork) — il continue de servir le
  chemin v2.x (Honoka) tel quel.
- Fichiers de test réels disponibles : `test/Densyanai Inko ver2.0/Data/MapData/{Map001,
  Map001_1, Map001_2, TitleMap}.mps` et `Data/CommonEvent.dat`.

## Estimation

8 phases · ~4–6 jours

## Items ROADMAP concernés

```
F5 — Engine Layer — Wolf RPG v3/WolfX :
  [ ] Parser .mps/CommonEvent.dat v3.x maison (v3_format/) — remplace wolfrpg-map-parser
      pour Inko
  [ ] Tests sur jeux Wolf v3.x réels (Densyanai Inko)
  [ ] Documentation format v3.x dans docs/engines.md
```

---

## Architecture du nouveau module

```
src-tauri/src/engines/wolf/v3_format/
├── mod.rs           // API publique : parse_map, dump_map, parse_common_events, dump_common_events
├── coder.rs         // ByteReader/ByteWriter génériques (u8/u32 LE, string préfixée par taille,
│                     // mode UTF-8 vs Shift-JIS via byte global IsUTF8)
├── compression.rs    // is_lz4_v3 / decompress_v3 / recompress_v3 (mirroring dat_parser.rs)
├── command.rs        // Command { cid: u32, args: Vec<u32>, indent: u8, string_args: Vec<String>,
│                     //            route: Option<Vec<RouteCommand>>, v35_unknown: Vec<u8> }
│                     // + RouteCommand { id: u8, args: Vec<u32> }
│                     // + parse/dump symétriques
└── map.rs            // MapV3 { tileset, width, height, layer_cnt, tiles, events: Vec<EventV3> }
                       // EventV3 { id, name, x, y, pages: Vec<PageV3> }
                       // PageV3 { ..., route: Vec<RouteCommand>, commands: Vec<Command>, ... }
                       // + parse/dump symétriques (s_v35 = version >= 0x67, posé une fois)
```

`extractor.rs`/`injector.rs` ne dépendent de ce module que pour les fichiers détectés
v3.x (LZ4) ; le chemin v2.x (`wolfrpg_map_parser::Map`) reste inchangé.

---

## Phases

---

### Phase 1 — `coder.rs` : lecteur/écrivain d'octets générique

**Objectif :** Fondation bas niveau, testée isolément, sans dépendance au reste.

**Fichiers :**
- `src-tauri/src/engines/wolf/v3_format/mod.rs` (squelette, déclare les sous-modules)
- `src-tauri/src/engines/wolf/v3_format/coder.rs`

Tâches :
- [ ] `ByteReader<'a>` : `read_u8`, `read_u32_le`, `read_bytes(n)`, `read_string(is_utf8:
  bool)` (u32 size LE → bytes → décodage UTF-8 ou SJIS via `encoding_rs`, trailing `0x00`
  stripping), `remaining()`, `position()`.
- [ ] `ByteWriter` : `write_u8`, `write_u32_le`, `write_bytes`, `write_string(&str,
  is_utf8: bool)` (encode + size prefix + trailing `0x00`).
- [ ] Erreurs : `V3FormatError` (via `thiserror`) — `UnexpectedEof { offset, needed,
  available }`, `InvalidString { offset }`. Pas de `panic!`/`.unwrap()` hors tests.
- [ ] Tests unitaires : round-trip `write_string` → `read_string` pour UTF-8 et SJIS,
  cas `EOF` retourne `Err` (pas de panic).

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::v3_format::coder
```

Commit message : `feat(wolf/v3): coder.rs — generic byte reader/writer (UTF-8/SJIS strings)`

---

### Phase 2 — `command.rs` : frame de commande générique + `RouteCommand`

**Objectif :** Porter `Command::Init`/`Dump` et `RouteCommand::Init`/`Dump` de WolfTL,
flat, avec gestion de `s_v35` (flag global passé en paramètre, pas de thread-local — éviter
le piège qui a coûté cher dans le fork).

**Fichiers :**
- `src-tauri/src/engines/wolf/v3_format/command.rs`

Tâches :
- [ ] `struct RouteCommand { id: u8, args: Vec<u32> }` + `parse`/`dump` (vérifie le magic
  2 octets `[0x01, 0x00]` après les args).
- [ ] `struct Command { cid: u32, args: Vec<u32>, indent: u8, string_args: Vec<String>,
  route: Option<Vec<RouteCommand>>, v35_unknown: Vec<u8> }`.
- [ ] `Command::parse(reader: &mut ByteReader, is_utf8: bool, v35: bool) ->
  Result<Self, V3FormatError>` :
  - `argsCount = read_u8() - 1`, `cid = read_u32_le()`, `argsCount` × `read_u32_le()`
  - `indent = read_u8()`, `strCount = read_u8()`, `strCount` × `read_string(is_utf8)`
  - `terminator = read_u8()` :
    - `0x00` → `route = None`
    - `0x01` → `route = Some(...)` : lire `routeCount = read_u32_le()` puis N ×
      `RouteCommand::parse` (cas `Move`, `cid == 201`)
    - autre → `Err(V3FormatError::InvalidTerminator { offset, found })`
  - si `v35` : `unknownSize = read_u8()`, `v35_unknown = read_bytes(unknownSize)`
- [ ] `Command::dump(&self, writer: &mut ByteWriter, is_utf8: bool, v35: bool)` — inverse
  exact (réécrit `argsCount+1`, `cid`, args, indent, strCount, strings, terminator
  `0x00`/`0x01` selon `route.is_some()`, route si présente, puis `v35_unknown` si `v35`).
- [ ] Constantes `pub const CID_MESSAGE: u32 = 101;` et `pub const CID_CHOICES: u32 =
  102;` — pour le repérage dans `extractor.rs`/`injector.rs` (pas de gros enum
  `CommandType`, on n'en a pas besoin pour le CAT).
- [ ] Tests unitaires sur **bytes synthétiques construits à la main** :
  - `Message` simple (1 string arg) avec et sans `v35_unknown`
  - `Choices` (plusieurs string args)
  - Commande avec `terminator == 0x01` (Move + route avec 2 `RouteCommand`)
  - round-trip `parse` → `dump` == bytes d'origine, pour chaque cas

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::v3_format::command
```

Commit message : `feat(wolf/v3): command.rs — generic flat command frame (port of WolfTL Command::Init/Dump)`

---

### Phase 3 — `map.rs` : structure Map/Event/Page + round-trip réel

**Objectif :** Porter `Map::load`/`dump`, `Event::Init`/`Dump`, `Page::Init`/`Dump`. Valider
le round-trip `parse → dump == identity` sur les 4 cartes Inko réelles (décompressées).

**Fichiers :**
- `src-tauri/src/engines/wolf/v3_format/map.rs`

Tâches :
- [ ] `struct PageV3 { unknown1: u32, graphic_name: String, graphic_bytes: [u8;4],
  conditions: Vec<u8> /* 37 bytes opaques */, movement: Vec<u8> /* 4 bytes */, flags: u8,
  route_flags: u8, route: Vec<RouteCommand>, commands: Vec<Command>, features: u32,
  shadow_collision: Vec<u8> /* 3 bytes */, page_transfer: Option<u8> }`.
- [ ] `struct EventV3 { id: u32, name: String, x: u32, y: u32, pages: Vec<PageV3> }`.
- [ ] `struct MapV3 { version: u32, unknown2: u8, unknown3: String, tileset: u32, width:
  u32, height: u32, layer_cnt: u32, tiles: Vec<u8>, events: Vec<EventV3> }`.
  - `is_utf8` et `v35` (`version >= 0x67`) sont dérivés de `version` et du flag global
    UTF-8 (déjà connu via le header `.mps`, cf. `extractor.rs::normalize_wolf_command_signatures`
    pour la détection actuelle du flag) — passés explicitement aux fonctions `parse`/`dump`,
    jamais en variable globale/thread-local.
- [ ] `MapV3::parse(bytes: &[u8], is_utf8: bool) -> Result<Self, V3FormatError>` — header,
  tuiles (gère le cas UTF-8 + marqueur `-1` = pas de tuiles → `tiles = vec![]`), boucle
  `while read_u8() == EVENT_INDICATOR`, vérifie `event_count` et le terminateur final.
- [ ] `MapV3::dump(&self, is_utf8: bool) -> Vec<u8>` — inverse exact.
- [ ] Test d'intégration (`#[cfg(test)] mod tests`, `#[ignore]`-able si fixtures absentes du
  repo — suivre le pattern existant pour les fichiers de `test/`) :
  - décompresser LZ4 (réutiliser `dat_parser::decompress_lz4_dat` ou équivalent — voir
    Phase 4) chacune des 4 cartes Inko
  - `MapV3::parse` ne doit **pas** retourner `Err`
  - `MapV3::parse(bytes).dump() == bytes` (round-trip byte-exact)
  - logguer `events.len()`, total `commands.len()`, et au moins un `Command { cid:
    CID_MESSAGE, .. }` avec un `string_args[0]` non vide et lisible (japonais, pas de
    mojibake)

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::v3_format::map
```
Résultat attendu : parse + round-trip OK sur les 4 cartes Inko réelles.

**⚠️ Décision gate (comme Phase 0 du plan précédent) :** si une carte échoue (parse error
ou round-trip non byte-exact), **STOP** — documenter l'octet/offset exact en désaccord
avec le modèle WolfTL avant de continuer. Ne pas "patcher pour faire passer le test".

Commit message : `feat(wolf/v3): map.rs — MapV3/EventV3/PageV3 (port of WolfTL Map/Event/Page), round-trip on real Inko maps`

---

### Phase 4 — `compression.rs` : LZ4 wrapper

**Objectif :** Isoler la détection/décompression/recompression LZ4 pour `.mps` v3.x,
factorisée et testée séparément (actuellement dupliquée par le spike).

**Fichiers :**
- `src-tauri/src/engines/wolf/v3_format/compression.rs`
- `src-tauri/src/engines/wolf/dat_parser.rs` (lecture seule — comparer avec
  `decompress_lz4_dat:417-437` pour cohérence du format, pas de modification si déjà
  réutilisable tel quel)

Tâches :
- [ ] `pub fn is_lz4_v3(bytes: &[u8]) -> bool` — `bytes.len() >= 33`, magic 16 octets,
  `bytes[16] ∈ {0x00, 0x55}` (flag UTF-8), `bytes[17..20] == [0,0,0]`, `bytes[20] >= 0x65`.
- [ ] `pub fn decompress_v3(bytes: &[u8]) -> Result<(Vec<u8>, bool /* is_utf8 */),
  V3FormatError>` — lit `dec_size`@25, `enc_size`@29, bloc LZ4 @33, retourne
  `header[0..25] ++ decompressed`. Si `dat_parser::decompress_lz4_dat` a la même logique
  exacte avec des offsets identiques, **réutiliser/extraire en commun** plutôt que
  dupliquer (vérifier d'abord — ne pas supposer).
- [ ] `pub fn recompress_v3(header_25: &[u8], payload: &[u8]) -> Vec<u8>` —
  `lz4_flex::block::compress`, réécrit `dec_size`/`enc_size`, reconstruit le fichier
  complet.
- [ ] Tests : round-trip `decompress_v3(bytes)` puis `recompress_v3(...)` sur les 4 cartes
  Inko → fichier final décompresse au même payload (pas forcément byte-identique au
  fichier d'origine si LZ4 produit un encodage différent — vérifier le **contenu
  décompressé**, pas les octets compressés).

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::v3_format::compression
```

Commit message : `feat(wolf/v3): compression.rs — LZ4 decompress/recompress for .mps v3.x`

---

### Phase 5 — Wiring `extractor.rs::extract_map_segments`

**Objectif :** Brancher `v3_format` dans le chemin d'extraction existant, sans toucher au
chemin v2.x (Honoka).

**Fichiers :**
- `src-tauri/src/engines/wolf/extractor.rs` (`extract_map_segments`, `extractor.rs:407-472`)

**Dépend de :** Phases 1-4

Tâches :
- [ ] En tête de `extract_map_segments` : si `v3_format::is_lz4_v3(bytes)` →
  `decompress_v3` → `MapV3::parse(decompressed_payload, is_utf8)` → walk
  `events/pages/commands`, repérer `cid == CID_MESSAGE` (→ `string_args[0]`) et `cid ==
  CID_CHOICES` (→ tous les `string_args`), même format de `key` que le chemin existant
  (`MapData/{map_name}/events/{i}/pages/{j}/{k}` et `.../choices/{l}`).
- [ ] Sinon (chemin existant inchangé) : `normalize_wolf_command_signatures` +
  `wolfrpg_map_parser::Map::parse` (v2.x).
- [ ] Remplacer le test `test_real_inko_maps_known_failure` (si toujours présent) par
  `test_real_inko_maps_v3` : assert `segments.len() > 0` sur les 4 cartes Inko, et au
  moins un segment avec du texte japonais lisible.

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::extractor
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : nouveaux tests verts, tests Honoka (v2.x) inchangés et toujours verts.

Commit message : `feat(wolf): extract_map_segments — v3.x (Inko) via v3_format, v2.x (Honoka) unchanged`

---

### Phase 6 — Wiring `injector.rs::inject_map`

**Objectif :** Injection AST-based pour v3.x (parse → remplace `string_args` → dump →
recompress), en parallèle du scan binaire existant pour v2.x.

**Fichiers :**
- `src-tauri/src/engines/wolf/injector.rs` (`inject_map`, `injector.rs:81-117`)

**Dépend de :** Phase 5

Tâches :
- [ ] En tête de `inject_map` : si `v3_format::is_lz4_v3(bytes)` → `decompress_v3` →
  `MapV3::parse` → pour chaque `Command { cid: CID_MESSAGE | CID_CHOICES, .. }`, calculer
  la même `key` que l'extraction et remplacer `string_args[i]` par la traduction si
  présente dans `translations` (sinon garder le texte source) → `MapV3::dump` →
  `recompress_v3`.
- [ ] Chemin v2.x (`patch_mps_strings`, scan binaire) **inchangé**.
- [ ] Test round-trip : extraire les segments d'une carte Inko réelle, injecter une
  traduction factice sur un `Command::ShowMessage`, dé-LZ4 le résultat, vérifier que
  `MapV3::parse` du résultat contient la traduction au bon endroit et que le fichier
  recompressé reste un LZ4 valide (`decompress_v3` réussit).

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::injector
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

Commit message : `feat(wolf): inject_map — v3.x (Inko) AST-based round-trip via v3_format`

---

### Phase 7 — `CommonEvent.dat` v3.x (Inko) — réutilisation de `command.rs`

**Objectif :** D'après la mémoire projet, le `.mps` Inko et le jeu de commandes
`CommonEvent.dat` Inko sont tous les deux bloqués (statut "F5 scope"). `CommonEvent.dat`
utilise le **même format `Command::Init`** (WolfTL, `CommonEvents.hpp:218` appelle
`Command::Command::Init` exactement comme `Map.hpp:75`) — donc `command.rs` (Phase 2) est
directement réutilisable ici.

**Fichiers :**
- `src-tauri/src/engines/wolf/v3_format/common_events.rs` (nouveau)
- `src-tauri/src/engines/wolf/extractor.rs::extract_common_events` (`extractor.rs:679+`)
- `src-tauri/src/engines/wolf/injector.rs` (injection CommonEvent.dat si applicable)

**Dépend de :** Phase 2 (et Phase 4 si `CommonEvent.dat` Inko est aussi LZ4 — déjà confirmé
par la mémoire projet : "LZ4 .dat (438 segs) + CommonEvent.dat header/UTF-8/LZ4 fixed").

Tâches :
- [ ] Lire `/home/blackat/project/test/WolfTL-main/WolfTL/WolfRPG/CommonEvents.hpp` autour
  de la ligne 218 pour confirmer la structure exacte du conteneur (header de
  CommonEvent, puis `commandCount` + boucle `Command::Init`, comme `Page::Init`).
- [ ] `struct CommonEventV3 { id: u32, name: String, ..., commands: Vec<Command> }` +
  `parse_common_events`/`dump_common_events` (réutilise `command::Command`).
- [ ] Brancher dans `extract_common_events`/injection, même logique de détection LZ4 que
  Phase 5/6.
- [ ] Test sur `Densyanai Inko/Data/CommonEvent.dat` réel : segments extraits > 0, texte
  lisible.

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

Commit message : `feat(wolf/v3): common_events.rs — CommonEvent.dat v3.x (Inko) via shared command.rs`

---

### Phase 8 — Tests end-to-end + docs + cleanup

**Objectif :** Validation manuelle complète sur Densyanai Inko, nettoyage de la
documentation et du spike.

**Fichiers :**
- `src-tauri/examples/wolf_mps_spike.rs` → **supprimé**
- `docs/references/wolfrpg-format-compatibility.md` → mis à jour (nouvelle section "v3.x
  (Inko) — résolu via v3_format/, modèle plat WolfTL")
- `docs/wolf_parser_audit.md`, `docs/wolf_rpg_history.md`, `docs/wolf_version_inventory.md`
  → conserver tel quel (contexte historique) ou résumer dans
  `wolfrpg-format-compatibility.md` avec lien
- `tasks/lessons.md` → entrée sur la leçon "modèle plat vs arborescent"
- `ROADMAP.md` → cocher items F5 concernés

Tâches :
- [ ] `pnpm tauri dev` → ouvrir `Densyanai Inko ver2.0/` → vérifier segments `.mps` ET
  `CommonEvent.dat` visibles, japonais lisible, export sans erreur, jeu démarre.
- [ ] Supprimer `wolf_mps_spike.rs` (cf. demande initiale de l'utilisateur).
- [ ] Mettre à jour `docs/references/wolfrpg-format-compatibility.md`.
- [ ] `tasks/lessons.md` :
  ```
  [2026-06-XX] Tentative de fork wolfrpg-map-parser pour v3.5 (v35Unknown) →
  Root cause: le modèle arborescent récursif de la crate (LoopCount{commands:Vec<Command>})
  ne compose pas avec le trailer v35Unknown plat de WolfTL →
  Règle: pour les formats binaires WOLF RPG v3.x, porter le modèle WolfTL (flat,
  Command::Init générique + indent) plutôt que d'adapter une crate tierce au modèle
  arborescent incompatible.
  ```
- [ ] `ROADMAP.md` : cocher les 3 items F5 listés en intro.

**Test de validation (gate complet) :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml
pnpm typecheck
```

Commit message : `docs(wolf): v3.x (Inko) resolved via v3_format — remove spike, update compatibility docs, lessons`

---

## Hors scope (futur — plan séparé)

Si `v3_format/` s'avère solide et bien testé sur Inko, **évaluer séparément** (nouveau
plan, pas dans celui-ci) le portage du chemin v2.x (Honoka) vers le même modèle :
- v2.x = même format `Command::Init` mais `s_v35 = false` (pas de `v35Unknown`), encodage
  Shift-JIS, pas de LZ4.
- Bénéfice : suppression complète de la dépendance externe `wolfrpg-map-parser` (plus de
  `[patch.crates-io]` git/branch fragile).
- Risque : Honoka fonctionne déjà à 100% (437 segments, 0 erreur) — toute régression sur
  ce chemin est un **downgrade** pour les utilisateurs actuels. À ne tenter qu'après
  validation longue de `v3_format/` sur Inko, et avec des tests de non-régression stricts
  (diff segment-par-segment vs. extraction actuelle).

---

## Verification gate (à chaque phase)

```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml
```

## Commandes git

```bash
git checkout -b feat/f5-01-wolf-v3-mps-parser
# ... commits par phase ...
git checkout main
git merge --no-ff feat/f5-01-wolf-v3-mps-parser -m "feat(f5-01): Wolf RPG v3.x (Inko) — in-house v3_format parser, replaces wolfrpg-map-parser for v3.x"
git push origin main
git branch -d feat/f5-01-wolf-v3-mps-parser
```

## Mise à jour après complétion

- `ROADMAP.md` : items F5 "Wolf v3/WolfX" cochés pour `.mps`/`CommonEvent.dat` Inko
- `CHANGELOG.md` : entrée `[Unreleased]` — "Wolf RPG v3.x (Inko) maps + CommonEvent.dat
  fully supported via in-house v3_format parser"
- `docs/engines.md` : section format v3.x documentée
- Mémoire session : Inko `.mps` + `CommonEvent.dat` résolus, `wolf_mps_spike.rs` supprimé,
  piste v2.x → wolfrpg-map-parser-free évaluée séparément
