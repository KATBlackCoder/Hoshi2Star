# Wolf RPG — Compatibilité des formats binaires

**Jeux testés :**
- `test/月咲流ホノカver1.03/` — Wolf RPG Editor v2.225 (Honoka) → **extraction/injection fonctionnelles** via `wolfrpg_map_parser` (fork)
- `test/Densyanai Inko ver2.0/` — Wolf RPG Editor v3.x (Inko) → **extraction/injection fonctionnelles** via le parser maison `v3_format/`

---

## État actuel (post F5-01)

```
Honoka maps (.mps, v2.225)        437 segments, 0 erreur     ✅ wolfrpg_map_parser
Honoka CommonEvent.dat (v2.225)   2195 segments, 0 erreur    ✅ wolfrpg_map_parser (fork)
Inko maps (.mps, v3.x)            8 segments (TitleMap), 0 erreur ✅ v3_format (maison)
Inko CommonEvent.dat (v3.x)       1585 segments, 0 erreur    ✅ v3_format (maison)
```

Wolf RPG Editor v3.x (Inko) utilise un format binaire entièrement différent de
v2.x (Honoka) : header LZ4-compressé + modèle de commandes "plat" (pas de
conteneurs récursifs). Le détail de ce format et la solution retenue sont
documentés ci-dessous (sections "Inko / v3.x").

Le reste de ce document (sections suivantes) couvre l'historique de
l'investigation v2.225 vs v2.292/v3.x via `wolfrpg-map-parser` — conservé pour
mémoire, mais **la conclusion finale pour le v3.x est différente** : un parser
maison (`src-tauri/src/engines/wolf/v3_format/`), pas un fork de la crate.

---

## Pourquoi Honoka (v2.225) fonctionne pour les maps

### Structure binaire d'un `.mps` v2.225

```
[  0.. 19] Signature 20 bytes : 00 00 00 00 00 00 00 00 00 00  W  O  L  F  M 00 00 00 00 00
[ 20.. 24] 5 bytes inconnus   : 64 00 00 00 65  (Honoka Map001)
[ 25.. 28] skippable (u32 LE) : 05 00 00 00  →  5
[ 29.. 33] skip 5 bytes       : 00 82 c8 82 b5
[ 34.. 37] tileset (u32 LE)   : 01 00 00 00  →  tileset 1
[ 38.. 41] width   (u32 LE)   : 1e 00 00 00  →  30
[ 42.. 45] height  (u32 LE)   : 16 00 00 00  →  22
[ 46.. 49] event_count        : 01 00 00 00  →  1 event
[ 50..    ] layer1 (width*height*4 bytes), layer2, layer3
[    ..   ] events
[    ..   ] 0x66  (map end)
```

La crate vérifie `magic == MAP_SIGNATURE` en dur. Honoka correspond exactement.

### Problème résiduel : commandes `CallCommonEvent` inconnues

La crate connaît les signatures D2 suivantes :

```rust
CallEvent1           = 0x06D20000
CallEvent2           = 0x05D20000
CallEvent3           = 0x07D20000
CallEventByVariable1 = 0x03D20000
CallEventByVariable2 = 0x0BD20000
ReserveEvent         = 0x03D30000
```

Les `.mps` de Honoka contiennent `0x04D20000` et `0x09D20000` — le `match` dans `Command::parse` tombe sur `_ => panic!("Unknown command code {:08x}", sig)`.

**Fix appliqué dans Hoshi2Star** (`extractor.rs` → `normalize_wolf_command_signatures`) :  
Avant d'appeler `Map::parse`, on remplace les bytes `04 D2 00 00` et `09 D2 00 00` par `06 D2 00 00`. Toutes les variantes D2 partagent le même layout binaire (le nombre d'arguments est lu depuis les données, pas depuis le nibble de signature), donc le remap est sans perte.

**Pourquoi on ne peut pas faire la même chose pour `CommonEvent.dat`** :  
Le pattern `09 D2 00 00` apparaît aussi *à l'intérieur* d'un `CommonEvent` déjà parsé comme :
```
[arg_count = 0x09]  [option_byte0 = 0xD2]  [option_byte1 = 0x00]  [option_byte2 = 0x00]
```
Patcher `0x09` → `0x06` change `argument_count` de 9 à 6. Le parseur consomme alors 12 bytes de moins depuis `number_arguments`, désalignant toute la suite → panic `Invalid common event signature: 06`.  
→ **La normalisation byte-level est non-fiable sur `CommonEvent.dat`.**

---

## Inko (v3.x) — la vraie cause : LZ4 + modèle de commandes plat

Une première investigation (conservée ci-dessus pour mémoire) avait conclu à
"6 incompatibilités structurelles" entre Honoka (v2.225) et Inko (v2.292) en
comparant les fichiers `.mps`/`CommonEvent.dat` bruts à des constantes
magiques codées en dur dans `wolfrpg-map-parser` (signature `0x55` vs `0x00`,
`skippable = 1913`, event signature `0x22` vs `0x00`, magic `CommonEvent.dat`
`0x93` vs `0x8F`, etc.).

**Cette analyse comparait des octets dans le mauvais référentiel.** Les
fichiers Inko (`.mps` et `CommonEvent.dat`) ne sont **pas** des variantes
"v2.292" du format v2.225 — ce sont des fichiers **Wolf RPG Editor v3.x**,
**compressés en LZ4** avec un header versionné inspiré de WolfTL
(github.com/Sinflower/WolfTL). Tous les octets "incompatibles" observés
faisaient partie du header LZ4/version, pas du contenu réel — une fois
décompressé, le payload utilise un modèle de commandes **plat**
(`Command::Init`, sans conteneurs récursifs `Page`/`EventCommand` imbriqués
comme en v2.x), totalement différent du modèle que `wolfrpg-map-parser` sait
parser.

### Format `.mps` v3.x (header LZ4, 25 octets)

```
[ 0..20] magic 20 bytes (WOLFMAP + flags), byte 16 = flag UTF-8 (0x00/0x55)
[20..25] version (u32 LE) + unknown2 (u8)
[25..29] dec_size (u32 LE) — taille décompressée du payload
[29..33] enc_size (u32 LE) — taille du bloc LZ4
[33..  ] bloc LZ4 brut (enc_size octets)
```

### Format `CommonEvent.dat` v3.5 (header LZ4, 11 octets)

```
[0]      0x00 (leading byte)
[1..10]  magic 9 octets : 57 00 00 4F 4C [00|55] 46 43 00  (byte 6 = flag UTF-8)
[10]     version (0x93 ou 0xCC → mode v3.5, LZ4 "Unpack")
[11..15] dec_size (u32 LE)
[15..19] enc_size (u32 LE)
[19..  ] bloc LZ4 brut (enc_size octets)
```

Décompressé = `header[0..11] ++ payload(dec_size octets)`, le payload commence
par `eventCnt: u32` puis N événements.

### Solution retenue : parser maison `v3_format/`

Plutôt que de patcher `wolfrpg-map-parser` (conçu pour le modèle récursif
v2.x), Hoshi2Star implémente un module dédié,
`src-tauri/src/engines/wolf/v3_format/`, porté byte-pour-byte depuis le
modèle plat de WolfTL (MIT) :

- `compression.rs` — décompression/recompression LZ4 générique (header
  variable selon `.mps` ou `CommonEvent.dat`)
- `command.rs` — `Command::Init`/`Dump` plat, partagé entre maps et common
  events
- `coder.rs` — primitives de lecture/écriture (strings UTF-8/Shift-JIS,
  entiers, etc.)
- `map.rs` — `.mps` v3.x (header + events + commandes plates)
- `common_events.rs` — `CommonEvent.dat` v3.5 (header LZ4 11 octets +
  événements + commandes plates via `command.rs`)

Chaque module est validé par un round-trip byte-exact
(`parse(bytes).dump() == bytes`) sur les fichiers réels d'Inko avant d'être
câblé dans `extractor.rs`/`injector.rs`.

v2.x (Honoka) continue d'utiliser `wolfrpg_map_parser` (fork) sans
changement — les deux formats restent gérés par des parsers séparés, voir
[`tasks/lessons.md`](../../tasks/lessons.md) (entrée 2026-06-11).

---

## État actuel dans Hoshi2Star

- `normalize_wolf_command_signatures` — appliquée dans `extract_map_segments` pour `.mps` Honoka (v2.x) ✅
- Tests d'intégration réels (`cargo test test_real_`) :
  - `test_real_honoka_maps` → **437 segments, 0 erreur** ✅ (`wolfrpg_map_parser`)
  - `test_real_honoka_common_events` → **2195 segments, 0 erreur** ✅ (`wolfrpg_map_parser`, fork)
  - `test_real_inko_maps_v3_round_trip` / `test_real_inko_maps_v3` → ✅ (`v3_format::map`, byte-exact round-trip)
  - `test_real_inko_common_events_v3_round_trip` / `test_real_inko_common_events_v3` → **1585 segments** depuis 1251 events, 0 erreur ✅ (`v3_format::common_events`, byte-exact round-trip)
