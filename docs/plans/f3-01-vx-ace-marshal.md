# Plan F3-01 — VX Ace Engine (marshal-rs, Temps 1)

## Objectif

Implémenter le support RPG Maker VX Ace pour les projets non-packagés (`.rvdata2` en clair).
Utiliser le crate `marshal-rs` pour parser/sérialiser le format Ruby Marshal sans Ruby installé.
Même architecture que le moteur MV/MZ : `extractor.rs` + `injector.rs` + `detector.rs` étendu.

## Statut : [ ] À faire

## Prérequis

- [x] F1 complet (engines/mv_mz/ implémenté, patterns établis)
- [x] F2 complet (commands/project.rs avec dispatch Engine, DB stable)
- [ ] marshal-rs disponible sur crates.io ← à vérifier au Step 1

## Estimation

10 steps · ~2h–2h30 · 3 steps ⚠️ à risque

## Items ROADMAP concernés

- `F3 > Engine Layer — RPG Maker VX Ace` — items 1 à 4 (hors `.rgss3a` → f3-02)

---

### Step 1 — Ajouter marshal-rs dans Cargo.toml

**Objectif :** Ajouter la dépendance et valider que `cargo build` compile.
**Fichiers touchés :** `src-tauri/Cargo.toml`
**Dépend de :** rien

Tâches :
- [ ] Ajouter `marshal-rs = "0"` dans `[dependencies]` (ou la version exacte trouvée sur crates.io)
- [ ] Vérifier le nom d'import Rust : `use marshal_rs::{load_utf8, dump};` (tiret → underscore)
- [ ] `cargo build` sans erreur de compilation

Test de validation :
```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | tail -5
```
Résultat attendu : `Finished` sans erreur · `marshal-rs` apparaît dans la liste des dépendances compilées.

Commit message : `build(deps): add marshal-rs for VX Ace .rvdata2 parsing`

---

### Step 2 — ⚠️ Investiguer la structure Value produite par marshal-rs

**Objectif :** Comprendre exactement ce que `load_utf8()` produit pour chaque type de fichier VX Ace.
**Fichiers touchés :** `src-tauri/src/engines/vx_ace/extractor.rs` (tests seulement, pas de code prod)
**Dépend de :** Step 1

> ⚠️ **Risque** : Le format exact de `serde_json::Value` produit par marshal-rs pour les objets Ruby
> (`RPG::Actor`, `RPG::Map`, etc.) est inconnu. Les JSON Pointer keys dépendent entièrement de cette
> structure. Si marshal-rs encode les Ruby Objects différemment des Hash Ruby, toute l'extraction
> doit s'adapter.
>
> **Hypothèse de travail** (à vérifier) : marshal-rs encode les Ruby Objects et Hash comme
> `serde_json::Value::Object`. Les fields Ruby snake_case (`game_title`) restent snake_case dans le Value.
> Les Array Ruby → `Value::Array`. nil Ruby → `Value::Null`.
>
> **Question critique sur MapInfos** : En VX Ace, `MapInfos.rvdata2` est un Ruby Hash
> `{1 => RPG::MapInfo, ...}` (clés Integer). marshal-rs encode les clés entières comme strings JSON
> (`"1"`, `"2"`, ...) → JSON Pointer `/1/name` (à confirmer).

Tâches :
- [ ] Créer un test `#[test] fn inspect_marshal_structure()` dans `extractor.rs`
  qui construit un `Value` JSON minimal représentant un acteur VX Ace, le `dump()`
  en marshal, puis le recharge avec `load_utf8()` et affiche la structure avec `dbg!()`
- [ ] Tester sur les types clés : Array d'objets (Actors), Hash (MapInfos), objet Map imbriqué
- [ ] Documenter dans ce fichier (section **Résultats investigation**) la structure exacte
  des JSON Pointer paths pour : `Actors`, `Items`, `Map001`, `MapInfos`, `System`
- [ ] Si la structure diffère de l'hypothèse, adapter les Steps 4 et 6 avant de les implémenter

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml inspect_marshal_structure -- --nocapture
```
Résultat attendu : `dbg!()` affiche la structure Value ; les paths JSON Pointer sont identifiés.

Commit message : `test(vx_ace): investigate marshal-rs Value structure for rvdata2 files`

#### Résultats investigation (complétés 2026-05-26)

**API réelle marshal-rs 2.0.1 (différente de la doc initiale) :**
```rust
// load_utf8 retourne Result<marshal_rs::Value, _>, pas serde_json::Value directement
// dump prend marshal_rs::Value + Option<&str>, pas serde_json::Value
// Conversion bidirectionnelle via Into : serde_json::Value ↔ marshal_rs::Value

fn to_json(bytes: &[u8]) -> serde_json::Value {
    let mv: marshal_rs::Value = load_utf8(bytes, None).unwrap();
    mv.into()
}
fn to_bytes(v: serde_json::Value) -> Vec<u8> {
    let mv: marshal_rs::Value = v.into();
    dump(mv, None)  // second arg = instance_var_prefix, None pour jeux standard
}
```

**JSON Pointer paths confirmés (tous opérationnels) :**
```
// Actors.rvdata2 → Array [null, {name, nickname, description, id}, ...]
// Pointeurs : /1/name ✓  /1/nickname ✓  /1/description ✓

// MapInfos.rvdata2 → Object {"1": {name, parent_id, order}, "2": {...}, ...}
// Pointeurs : /1/name ✓  /2/name ✓  (clés string, pas integer)

// Map001.rvdata2 → Object {display_name, events: Object {"1": {pages:[{list:[...]}]}}}
// Pointeurs : /events/1/pages/0/list/0/parameters/0 ✓  (events = Hash clés string)
//             /events/1/pages/0/list/1/parameters/0/0 ✓ (choices)

// System.rvdata2 → Object {game_title, currency_unit, terms: {basic:[], commands:[], params:[], messages:{}}}
// Pointeurs : /game_title ✓  /currency_unit ✓  /terms/basic/0 ✓  /terms/messages/key ✓
//             SNAKE_CASE confirmé (game_title, pas gameTitle — différence MV/MZ)

// Idempotence Value : load_utf8(dump(load_utf8(bytes))) == load_utf8(bytes) ✓
```

**Adaptations pour Steps 4 et 6 :**
- `extract_from_bytes` : `load_utf8(bytes, None).map(Into::into)` pour obtenir `serde_json::Value`
- `inject_and_serialize` : inject sur `serde_json::Value` → `.into()` → `dump(mv, None)`
- `extract_map` : `events` est un Object (pas Array) → itérer sur `as_object()`, trier clés par parse::<u32>()
- `extract_map_infos` : json est un Object avec clés string → itérer sur `as_object()`
- `extract_system` : utiliser `game_title` (snake_case), pas `gameTitle`

---

### Step 3 — Créer la structure du module vx_ace/

**Objectif :** Scaffolder les fichiers vides, déclarer dans engines/mod.rs.
**Fichiers touchés :**
- `src-tauri/src/engines/vx_ace/mod.rs` (nouveau)
- `src-tauri/src/engines/vx_ace/extractor.rs` (nouveau, vide)
- `src-tauri/src/engines/vx_ace/injector.rs` (nouveau, vide)
- `src-tauri/src/engines/mod.rs` (ajouter `pub mod vx_ace;`)
**Dépend de :** Step 2 (les paths sont connus)

Tâches :
- [ ] Créer `src-tauri/src/engines/vx_ace/` avec les 3 fichiers
- [ ] `mod.rs` : `pub mod extractor; pub mod injector;` (pas de decryptor pour Temps 1)
- [ ] `extractor.rs` : module doc + `use marshal_rs::load_utf8;` + stubs vides
- [ ] `injector.rs` : module doc + `use marshal_rs::dump;` + stubs vides
- [ ] Ajouter `pub mod vx_ace;` dans `engines/mod.rs`
- [ ] `cargo build` sans erreur

Test de validation :
```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep -E "(error|warning.*unused)" | head -10
```
Résultat attendu : build propre, warnings `unused` acceptables pour les stubs.

Commit message : `feat(engines): scaffold vx_ace module structure (extractor, injector)`

---

### Step 4 — ⚠️ Implémenter extractor.rs

**Objectif :** Extraire tous les segments traduisibles depuis les `.rvdata2` VX Ace.
**Fichiers touchés :** `src-tauri/src/engines/vx_ace/extractor.rs`
**Dépend de :** Step 2 (structure JSON Pointer confirmée), Step 3

> ⚠️ **Risque** : Les structures Ruby divergent de MV/MZ sur plusieurs points clés :
> - `MapInfos` : Ruby Hash (clés entières) vs MV/MZ Array → JSON Pointer `/1/name` vs `/{i}/name`
> - `System` : fields snake_case (`game_title`) vs MV/MZ camelCase (`gameTitle`)
> - `Map*.rvdata2` : `events` est un Ruby Hash `{Integer → RPG::Event}` vs Array en MV/MZ
> - Code 101 VX Ace : **pas de speaker name** (params[3] = position, pas de params[4])
> - `Troops` : même pattern que MV/MZ (Array → pages → list)

Tâches :
- [ ] Copier/adapter `SegmentKind` et `ExtractedSegment` depuis `mv_mz/extractor.rs`
  (duplication intentionnelle — pas d'abstraction prématurée)
- [ ] Implémenter `pub fn extract_from_bytes(file_name: &str, bytes: &[u8]) -> Vec<ExtractedSegment>`
  qui appelle `load_utf8(bytes)` puis dispatch vers la bonne fonction
- [ ] `extract_actors(value: &Value)` — `[null, {name, nickname, description}, ...]`
- [ ] `extract_items(value: &Value)` — `[null, {name, description}, ...]`
- [ ] `extract_weapons(value: &Value)` — idem Items
- [ ] `extract_armors(value: &Value)` — idem Items
- [ ] `extract_skills(value: &Value)` — `name`, `description`, `message1`, `message2`
- [ ] `extract_states(value: &Value)` — `name`, `message1`–`message4`
- [ ] `extract_enemies(value: &Value)` — `name` uniquement
- [ ] `extract_troops(value: &Value)` — pages → list (codes 401, 102 identiques VX Ace)
- [ ] `extract_map_infos(value: &Value)` — Hash `{"1": {name}, "2": {name}, ...}`
- [ ] `extract_common_events(value: &Value)` — `[null, {name, list}, ...]`
- [ ] `extract_map(value: &Value)` — `events` Hash → pages → list (codes 401, 102, **pas de 101 speaker**)
- [ ] `extract_system(value: &Value)` — `game_title`, `currency_unit`, `terms`
  (attention : fields snake_case Ruby, pas camelCase)
- [ ] Filtre `is_empty()` / `trim().is_empty()` sur chaque texte (identique MV/MZ)
- [ ] **Réutiliser** `is_placeholder_only()` de MV/MZ directement — VX Ace utilise les mêmes
  placeholders (`\V[n]`, `\N[n]`, `\C[n]`, etc.) — tokenizer existant OK

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml vx_ace::extractor -- --nocapture 2>&1 | tail -20
```
Résultat attendu : 0 test failures (tests écrits au Step 5).

Commit message : `feat(vx_ace): implement extractor for all rvdata2 file types`

---

### Step 5 — Tests unitaires extractor.rs

**Objectif :** Couvrir chaque fonction d'extraction avec des données marshal synthétiques.
**Fichiers touchés :** `src-tauri/src/engines/vx_ace/extractor.rs` (bloc `#[cfg(test)]`)
**Dépend de :** Step 4

> **Stratégie** : Construire les `serde_json::Value` de test directement (pas de fichiers .rvdata2
> sur disque). Utiliser `serde_json::json!()` avec la structure confirmée au Step 2.
> Le round-trip complet (bytes → Value → extract → inject → dump → bytes) est testé au Step 7.

Tâches :
- [ ] `test_extract_actors_basic` — 2 acteurs, vérifier name/nickname/description
- [ ] `test_extract_actors_skips_null` — premier élément null ignoré
- [ ] `test_extract_items_name_and_description`
- [ ] `test_extract_skills_with_messages` — message1 rempli, message2 vide → 3 segments
- [ ] `test_extract_states_four_messages`
- [ ] `test_extract_map_infos_hash_keys` — clés `"1"`, `"2"` → JSON Pointer `/1/name`, `/2/name`
- [ ] `test_extract_common_events_name_and_dialogue`
- [ ] `test_extract_map_dialogue_code_401`
- [ ] `test_extract_map_choices_code_102`
- [ ] `test_extract_map_no_speaker_in_code_101` — VX Ace : code 101 sans speaker name
- [ ] `test_extract_system_game_title_snake_case` — field `game_title` (pas `gameTitle`)
- [ ] `test_extract_skips_empty_strings`
- [ ] `test_extract_skips_whitespace_only`

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml vx_ace::extractor
```
Résultat attendu : tous les tests verts (aucun `FAILED`).

Commit message : `test(vx_ace): unit tests for all extractor functions`

---

### Step 6 — Implémenter injector.rs

**Objectif :** Réécrire les segments traduits dans un `serde_json::Value` marshal, puis `dump()` en bytes.
**Fichiers touchés :** `src-tauri/src/engines/vx_ace/injector.rs`
**Dépend de :** Step 4

> **Différence clé vs MV/MZ** : L'injector MV/MZ reçoit un `&mut serde_json::Value` déjà parsé.
> Pour VX Ace, il faut aussi exposer la sérialisation `dump()` → `Vec<u8>`.
> Le module expose donc deux fonctions publiques : `inject()` (même API que MV/MZ) + `serialize()`.

Tâches :
- [ ] Reprendre `InjectorError` (KeyNotFound, NotAString) — identique MV/MZ
- [ ] `pub fn inject(value: &mut Value, translations: &[(&str, &str)]) -> Result<(), InjectorError>`
  — exactement la même implémentation que `mv_mz/injector.rs` (pointer_mut)
- [ ] `pub fn serialize(value: Value) -> Vec<u8>` — appelle `marshal_rs::dump(value)`
- [ ] `pub fn inject_and_serialize(bytes: &[u8], translations: &[(&str, &str)]) -> Result<Vec<u8>, InjectorError>`
  — fonction convenience : `load_utf8(bytes)` → `inject()` → `serialize()`

Test de validation :
```bash
cargo build --manifest-path src-tauri/Cargo.toml 2>&1 | grep "^error" | wc -l
```
Résultat attendu : `0` erreur de compilation.

Commit message : `feat(vx_ace): implement injector (pointer_mut + marshal dump)`

---

### Step 7 — Tests unitaires injector.rs

**Objectif :** Valider le round-trip extract → inject → contenu identique.
**Fichiers touchés :** `src-tauri/src/engines/vx_ace/injector.rs` (bloc `#[cfg(test)]`)
**Dépend de :** Step 5 + Step 6

> **Note round-trip bytes** : `dump(load_utf8(bytes)) != bytes` est attendu pour les jeux
> japonais Shift-JIS (UTF-8 est différent au niveau binaire). Le test pertinent est :
> `load_utf8(dump(load_utf8(bytes))) == load_utf8(bytes)` (Value idempotent, pas bytes).

Tâches :
- [ ] `test_inject_actor_name` — injecter une traduction sur `/1/name`, vérifier le Value
- [ ] `test_inject_error_key_not_found` — clé absente → InjectorError::KeyNotFound
- [ ] `test_inject_error_not_a_string` — cible non-string → InjectorError::NotAString
- [ ] `test_round_trip_actors` — extract → inject (source comme target) → Value identique
- [ ] `test_round_trip_map` — extract_map → inject → Value identique
- [ ] `test_serialize_produces_valid_marshal` — `load_utf8(serialize(value))` == value original
  (idempotence du Value, pas des bytes)

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml vx_ace::injector
```
Résultat attendu : tous verts.

Commit message : `test(vx_ace): round-trip and error tests for injector`

---

### Step 8 — ⚠️ Étendre detector.rs pour VX Ace

**Objectif :** Ajouter `Engine::VxAce` et sa détection sans régresser MV/MZ.
**Fichiers touchés :** `src-tauri/src/engines/detector.rs`
**Dépend de :** Step 3

> ⚠️ **Risque Linux / sensibilité à la casse** : VX Ace utilise `Data/` (D majuscule) sur Windows.
> Sur Linux (CachyOS), le filesystem est case-sensitive. `data/` ≠ `Data/`.
> Stratégie : chercher `Data/` EN PREMIER (exact case), puis fallback `data/` pour les jeux
> extraits sur Linux sans préservation de casse. Documenter ce comportement.
>
> **Ordre de détection** : MV/MZ EN PREMIER (System.json dans data/), puis VX Ace.
> Critères VX Ace : `Data/System.rvdata2` existe OU (`Data/*.rvdata2` présent ET `Game.ini` présent).
> Absence de `www/data/` n'est PAS nécessaire (MV/MZ détecté avant grâce à System.json).

Tâches :
- [ ] Ajouter `VxAce` à `enum Engine`
- [ ] Implémenter `find_vx_ace_data_dir(game_dir: &Path) -> Option<PathBuf>`
  — cherche `Data/` puis `data/` (fallback case-insensitive Linux)
- [ ] Implémenter `is_vx_ace_data_dir(dir: &Path) -> bool`
  — vérifie `System.rvdata2` présent dans le dossier
- [ ] Dans `detect_engine()` : ajouter la branche VX Ace APRÈS la branche MV/MZ existante
- [ ] Tests unitaires (tempdir) :
  - `test_detect_vx_ace_data_dir` — `Data/System.rvdata2` présent
  - `test_detect_vx_ace_fallback_lowercase` — `data/System.rvdata2` (Linux)
  - `test_detect_mv_mz_not_confused_with_vx_ace` — `data/System.json` → MvMz (pas VxAce)
  - `test_detect_unknown_neither` — vide → UnknownEngine

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml detector
```
Résultat attendu : tous les tests detector verts (anciens MV/MZ + nouveaux VX Ace).

Commit message : `feat(detector): add Engine::VxAce detection (Data/*.rvdata2 + Game.ini)`

---

### Step 9 — Mettre à jour open_project command

**Objectif :** Dispatcher vers vx_ace::extractor quand `Engine::VxAce` est détecté.
**Fichiers touchés :** `src-tauri/src/commands/project.rs`
**Dépend de :** Step 4, Step 6, Step 8

Tâches :
- [ ] Ajouter `use crate::engines::vx_ace::{extractor as vx_extractor, injector as vx_injector};`
- [ ] Dans `open_project` : ajouter `Engine::VxAce => "vx_ace"` dans le match `engine_str`
- [ ] Implémenter `collect_rvdata2_files(data_dir: &Path) -> Result<Vec<(String, String, String, Vec<u8>)>, io::Error>`
  — analogue à `collect_json_files()` mais retourne `Vec<u8>` (binaire, pas string)
  — filtre les extensions `.rvdata2` uniquement
  — pas de parsing JSON (le parsing est délégué à `extract_from_bytes`)
- [ ] Implémenter `classify_vx_ace_file(file_name: &str) -> &'static str`
  — `Actors.rvdata2` → `"vx_actors"`, `Map001.rvdata2` → `"vx_map"`, etc.
  — utiliser des types distincts `"vx_*"` pour différencier dans le FileTree
- [ ] Dans `open_project` : branche `Engine::VxAce` utilise `collect_rvdata2_files()` et
  `vx_extractor::extract_from_bytes(file_name, &bytes)` pour chaque fichier
- [ ] Dans `export_project` : branche `vx_ace` utilise `vx_injector::inject_and_serialize()`
  et écrit `Vec<u8>` (pas string) avec `std::fs::write(path, bytes)`
- [ ] Tests unitaires :
  - `test_classify_vx_ace_map_files` — `Map001.rvdata2` → `"vx_map"`
  - `test_classify_vx_ace_data_files` — `Actors.rvdata2` → `"vx_actors"`, etc.

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml commands::project
```
Résultat attendu : tous les tests project verts (anciens MV/MZ + nouveaux VX Ace).

Commit message : `feat(commands): dispatch Engine::VxAce in open_project and export_project`

---

### Step 10 — Mettre à jour l'UI FileTree

**Objectif :** Ajouter des icônes distinctes pour les types de fichiers VX Ace (`vx_*`).
**Fichiers touchés :** `src/components/editor/FileTree.tsx`
**Dépend de :** Step 9

> Les types `"vx_actors"`, `"vx_map"`, etc. (Step 9) arrivent via `get_source_files()`.
> Le FileTree doit les afficher avec des icônes visuellement distinctes des types MV/MZ.
> Stratégie : même icône logique (Map → Map icon) mais couleur Amber/Teal pour VX Ace
> vs Blue/Green pour MV/MZ — distinction visuelle sans complexité excessive.

Tâches :
- [ ] Dans `fileIcon()`, ajouter les cas `"vx_map"`, `"vx_actors"`, `"vx_armors"`, `"vx_weapons"`,
  `"vx_skills"`, `"vx_items"`, `"vx_enemies"`, `"vx_classes"`, `"vx_common_events"`,
  `"vx_map_infos"`, `"vx_system"`, `"vx_states"`, `"vx_troops"`
- [ ] Couleur distincte : `text-amber-400` pour les types VX Ace (vs couleurs actuelles MV/MZ)
- [ ] `pnpm typecheck` sans erreur

Test de validation :
```bash
pnpm typecheck 2>&1 | grep -c "^error"
```
Résultat attendu : `0`

Commit message : `feat(ui): add VX Ace file type icons in FileTree (amber color scheme)`

---

## Tests obligatoires avant push GitHub

```bash
# 1. Tous les tests Rust (anciens + nouveaux VX Ace)
cargo test --manifest-path src-tauri/Cargo.toml
# Résultat attendu : 0 FAILED (anciens tests MV/MZ doivent rester verts)

# 2. Clippy strict
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
# Résultat attendu : 0 warnings, 0 errors

# 3. Fmt
cargo fmt --manifest-path src-tauri/Cargo.toml --check
# Résultat attendu : exit 0 (code déjà formatté)

# 4. TypeScript
pnpm typecheck
# Résultat attendu : 0 erreurs

# 5. Test manuel (si accès à un jeu VX Ace non-packagé)
# - Ouvrir un dossier jeu VX Ace via l'UI
# - Vérifier que les segments s'affichent dans la grille
# - Vérifier que les noms de fichiers ont les icônes amber dans le FileTree
# - Exporter et vérifier que les .rvdata2 sont bien réécrits
```

## Commandes git de push final

```bash
# Vérifier l'état après tous les commits des 10 steps
git log --oneline -15

# S'assurer que tous les tests passent une dernière fois
cargo test --manifest-path src-tauri/Cargo.toml && pnpm typecheck

# Push vers main (ou PR si branche de feature)
git push origin main
```

## Mise à jour après complétion

- [ ] Cocher dans ROADMAP.md :
  - `[ ] Intégration marshal-rs` → `[x]`
  - `[ ] src-tauri/src/engines/vx_ace/extractor.rs` → `[x]`
  - `[ ] src-tauri/src/engines/vx_ace/injector.rs` → `[x]`
  - Laisser `[ ] Support archive .rgss3a` pour f3-02
  - Laisser `[ ] Tests round-trip VX Ace` → `[x]`
- [ ] Mettre à jour CHANGELOG.md (skill `update-changelog`)
- [ ] Créer journal `docs/journal/YYYY-MM-DD-f3-01-vx-ace.md`
- [ ] Statut de ce plan → `[x] Complété`
- [ ] Créer `docs/plans/f3-02-vx-ace-rgss3a.md` pour le Temps 2 (archives `.rgss3a` + zlib)
