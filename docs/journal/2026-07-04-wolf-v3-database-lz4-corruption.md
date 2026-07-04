# Journal — 2026-07-04 — Fix corruption LZ4 des bases Wolf RPG v3.5 à l'export

**Phase** : F5 (Wolf RPG) — bugfix critique (corruption de données)
**Statut** : ✅ Complété

---

## Contexte

L'utilisateur a traduit intégralement le projet Wolf RPG « Densyanai_Inko_ver2.0 »
(v3.5, UTF-8), exporté (`export_project` → `hoshi2star.zip`), copié-collé le
résultat par-dessus le dossier du jeu, puis lancé `Game.exe` : Wolf RPG Editor
affiche « データベースファイル BasicData/DataBase が破損しているか、バージョンが
古い可能性があります。» (fichier de base de données corrompu ou version trop
ancienne) et le jeu ne démarre pas.

## Diagnostic

Inspection binaire directe des fichiers du dossier `test/Densyanai_Inko_ver2.0/
Data/BasicData/` :

```
DataBase.dat  : 00 57 00 00 4f 4c 55 46 4d 00 c4 1e000000 feffffff 00
CDataBase.dat : 00 57 00 00 4f 4c 55 46 4d 00 c4 28000000 feffffff 00
```

Octet 10 = `0xC4` → l'en-tête déclare le format LZ4 (Wolf RPG v3.x) : les 8
octets suivants doivent être `decompressed_size:u32 + compressed_size:u32`
puis un bloc LZ4 brut. Or les 4 octets à l'offset 15 (`fe ff ff ff`) sont
exactement `DAT_TYPE_SEPARATOR` — la valeur qui apparaît en clair dans le
format **non compressé**. Preuve directe que le fichier a été sérialisé en
clair sous un en-tête qui prétend être du LZ4.

Cause : `injector.rs::serialize_dat` — utilisée par `inject_dat` pour
réinjecter les traductions dans `.dat` de base de données — préservait
l'octet de version d'origine (`dat_original_header[10]`) mais écrivait
toujours le payload au format v2.x en clair, sans jamais recompresser en
LZ4. `parse_database` (lecture) gère pourtant très bien la décompression
LZ4 depuis F5-01 — seule l'écriture (`serialize_dat`) n'avait jamais été
mise à jour pour le cas v3.5. `CommonEvent.dat`, lui, était déjà correct :
`inject_common_events` a son propre chemin v3.5 (`inject_common_events_v3`)
qui décompresse puis recompresse via `v3_format`.

Confirmation indépendante : le test existant `test_real_inko_database_segments`
(qui lit directement `test/Densyanai_Inko_ver2.0/Data/BasicData/`, la copie
de jeu de l'utilisateur) échouait déjà, avant tout changement de code, avec
`LZ4 .dat file is shorter than compressed_size` — exactement la même
corruption que celle vue par Wolf RPG Editor.

## Correctif

- `dat_parser.rs` : `DAT_VERSION_LZ4` passé `pub(crate)` ; nouvelle fonction
  `compress_lz4_dat(payload) -> Vec<u8>`, symétrique de `decompress_lz4_dat`
  (compresse le payload plain avec `lz4_flex::block::compress`, préfixe
  `decompressed_size` + `compressed_size`).
- `injector.rs::serialize_dat` : sépare désormais header (indicateur+magic+
  version) et payload (type_count + types + terminateur) ; si
  `version_byte == DAT_VERSION_LZ4`, le payload est recompressé via
  `compress_lz4_dat` avant d'être concaténé au header — sinon écrit en clair
  comme avant (comportement v2.x inchangé).
- Nouveau test `test_inject_dat_lz4_v3_recompresses` : construit un `.dat`
  LZ4 synthétique (magic SJIS pour rester cohérent avec le `.project` de
  test existant), injecte une traduction, vérifie que l'en-tête reste `0xC4`
  **avec un vrai bloc LZ4** derrière (pas `DAT_TYPE_SEPARATOR` en clair), et
  que la re-parse via `parse_database` (même chemin que le runtime Wolf)
  retrouve la traduction.

## Vérification

- Gate complet : typecheck ✅ · 52 Vitest ✅ · clippy 0 ✅ · **388/389** tests
  Rust (le seul rouge, `test_real_inko_database_segments`, échoue de façon
  identique avec et sans le correctif — confirmé par `git stash` — car il
  lit la copie de jeu déjà corrompue sur disque, que le code ne peut pas
  réparer rétroactivement).
- Pas de vérif live possible : réparer véritablement le jeu de l'utilisateur
  nécessite un `DataBase.dat`/`CDataBase.dat` **propres** (non traduits) en
  remplacement — Hoshi2Star ne conserve pas de copie binaire de l'original,
  seul le texte des segments est en base SQLite, donc `inject_dat` n'a plus
  de base valide à partir de laquelle réinjecter.

## Recommandation transmise à l'utilisateur

1. Restaurer une copie propre de `DataBase.dat` + `CDataBase.dat` (téléchargement
   d'origine, sauvegarde, réinstallation) dans `Data/BasicData/` — uniquement
   ces 2 fichiers, le reste du dossier (déjà traduit) reste intact.
2. Une fois le correctif en place, relancer `export_project` sur ce projet :
   `hoshi2star.zip` recompressera correctement les deux fichiers.
3. Recopier le nouveau zip par-dessus la copie restaurée.

## Prochaine session

- Aucun follow-up de code identifié : le correctif couvre le seul point
  d'écriture `.dat` manquant (`serialize_dat`). Les chemins `.mps` (maps) et
  `CommonEvent.dat` géraient déjà la recompression v3.5.

---
*Généré par Claude Code — Hoshi2Star*
