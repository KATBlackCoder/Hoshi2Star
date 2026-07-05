# Journal — 2026-07-05 — Fix version-detection + encoding mismatch for unpacked Wolf v3.x games

**Phase** : F5 (Wolf RPG) — bugfix critique (corruption de données), suite directe du fix LZ4 (2026-07-04)
**Statut** : ✅ Complété, vérifié end-to-end (export réel)

---

## Contexte

Après le fix LZ4 de la veille, l'utilisateur restaure une copie de secours de
`DataBase.dat`/`CDataBase.dat`, puis tente un nouvel export depuis Hoshi2Star.
Nouvelle erreur : « Erreur export : dat parse error: encoding error: invalid
utf-8 sequence of 1 bytes from index 14 ».

## Diagnostic

Reproduction directe avec `test_real_inko_database_segments` : le fichier
restauré échoue à l'identique, AVANT tout changement de code — la corruption
est bien dans le fichier restauré, pas un artefact de la session en cours.

Localisation exacte (réplication manuelle en Python de `parse_project`/
`parse_dat_types` sur le payload décompressé) : `type[0]` (技能/Skills),
`entry[7]` (アイスエッジ/IceEdge), champ `使用時文章[戦闘]` (message de
combat). Valeur stockée dans la base SQLite de Hoshi2Star :
`cast Ice．Edge!` (「．」= U+FF0E, point plein cadratin) — texte valide.

Comparaison bit à bit : l'UTF-8 de ce texte donne
`...49636 5 efbc8e 4564676521`, mais le fichier `.dat` contenait
`...49636 5 8144 4564676521` — exactement l'encodage **Shift-JIS** de la même
chaîne (「．」→ `81 44` en SJIS). Le texte a donc été écrit en Shift-JIS dans
un fichier dont l'en-tête déclare UTF-8.

Cause racine, en deux temps :
1. `detector.rs::guess_wolf_version_from_structure` ne sait détecter la
   version Wolf RPG qu'en lisant le champ CodePage d'une archive `.wolf` —
   or ce jeu est distribué **sans archive** (dossier `Data/` en clair). Sans
   archive à sonder, la fonction retombe sur le défaut codé en dur
   `WolfVersion { major: 2, minor: 0 }` (Shift-JIS), alors que ses fichiers
   `.dat` sont réellement en v3.5/UTF-8 (magic `0x55` confirmé).
2. `injector.rs::serialize_dat_type` recevait pourtant déjà le bon signal
   par fichier (`dat.is_utf8`, dérivé du magic du `.dat` lui-même, utilisé
   correctement pour l'en-tête) — mais il l'ignorait (`_is_utf8` non utilisé)
   et encodait le contenu des chaînes via `encode_for_wolf(s, version)`, où
   `version` est ce même défaut erroné v2.0 global. Résultat : en-tête UTF-8
   correct, contenu Shift-JIS incorrect, dès qu'un caractère non-ASCII
   apparaît dans une traduction de base de données.

`CommonEvent.dat` (v3.5/LZ4) était épargné car son chemin dédié
(`inject_common_events_v3`) ne consulte jamais `version` pour l'encodage —
toujours UTF-8 par construction du format v3_format.

## Correctif (3 volets)

1. **`detector.rs`** : `guess_wolf_version_from_structure` tente d'abord
   l'archive `.wolf` (inchangé), puis un nouveau fallback
   `guess_wolf_version_from_loose_dat` qui sonde le magic de
   `Data/BasicData/SysDatabase.dat` (toujours présent, signal fiable),
   et seulement en dernier recours le défaut v2.0.
2. **`injector.rs`** : `encode_for_wolf` prend désormais un `bool` (`is_utf8`)
   plutôt qu'un `&WolfVersion` ; `serialize_dat_type` utilise le `is_utf8`
   par fichier (déjà calculé, auparavant ignoré) au lieu du `version`
   externe. `inject_dat` garde son paramètre `_version` (renommé, inutilisé)
   uniquement pour conserver la même signature que `inject_map`/
   `inject_common_events` dans la boucle de dispatch `inject_all`.
3. **`dat_parser.rs`** : `read_wolf_string` retente en Shift-JIS quand un
   décodage UTF-8 échoue sur une base déclarée UTF-8 — corrige exactement
   cette erreur précise sans risque de masquer une vraie corruption
   (`decode_shiftjis` échoue lui aussi sur des octets réellement invalides).
   Effet : les fichiers déjà corrompus par ce bug redeviennent lisibles, et
   le tout prochain export réussi les réécrit correctement en UTF-8 — la
   base se répare d'elle-même dès le premier export sain.

## Vérification

- Gate complet : `pnpm typecheck` ✅ · 52 Vitest ✅ · `cargo clippy -D
  warnings` ✅ · **394 tests Rust ✅ (+6 vs 0.4.7)**, y compris
  `test_real_inko_database_segments` — qui échouait avant ce fix sur le
  fichier réel de l'utilisateur, et passe maintenant.
- **Vérification live end-to-end** (`pnpm tauri:linux` + MCP Tauri, projet
  réel Densyanai_Inko) : ouverture du projet → « Tout exporter » (4 fichiers,
  1948 segments) → **aucune erreur** (le dialogue « appliquer taille de
  police » ignoré, non lié au bug) → `hoshi2star.zip` généré. Extraction du
  zip + re-décodage du `DataBase.dat` frais via `parse_database` : le champ
  `使用時文章[戦闘]` de l'entrée アイスエッジ vaut bien `"cast Ice．Edge!"` —
  le point plein cadratin est correctement UTF-8, plus de mojibake.

## Fichiers modifiés

- `src-tauri/src/engines/detector.rs` — fallback loose-`.dat`, 4 tests
- `src-tauri/src/engines/wolf/injector.rs` — `encode_for_wolf(bool)`,
  `serialize_dat_type` sur `is_utf8` par fichier, 1 test
- `src-tauri/src/engines/wolf/dat_parser.rs` — fallback SJIS dans
  `read_wolf_string`, 1 test

## Note pour l'utilisateur

Ce fix répare les fichiers déjà partiellement corrompus **à la lecture** —
mais seulement s'ils restent des fichiers LZ4 structurellement valides (ce
qui est le cas de la copie de secours restaurée). Aucune action de
récupération de fichier supplémentaire n'est nécessaire cette fois : un
simple nouvel export depuis Hoshi2Star (avec ce correctif) suffit à produire
un patch propre.

---
*Généré par Claude Code — Hoshi2Star*
