# Plan F4-04 — Wolf RPG : Injecteur binaire

## Objectif

Réécrire les fichiers binaires Wolf RPG (`.mps` et `.dat`) avec les traductions.
Mirror de l'extracteur (F4-03) : pour chaque clé de segment, localiser la position
binaire et remplacer le texte. Gérer le re-encodage UTF-8 → Shift-JIS pour Wolf v2.

Stratégie d'export pour F4 : **Option A — fichiers déchiffrés dans Data/**
(le moteur Wolf RPG lit `Data/` en priorité sur les archives `.wolf`).
Pas de re-chiffrement DXA en F4 — Option B reportée en F5.

## Statut : [ ] À faire

## Prérequis

- F4-03 complet (extracteur + clés de segments) — `[ ]` À faire
- F4-02 complet (décrypteur, pour comprendre la structure binaire) — `[ ]` À faire
- `src-tauri/src/engines/wolf/injector.rs` stub créé (F4-01 Step 2) ✅
- `wolf/encoding.rs` avec `encode_shiftjis` disponible (F4-03 Step 2) ✅

## Estimation

6 steps · ~2–3 jours

## Items ROADMAP concernés

```
F4 — Engine Layer — Wolf RPG v1/v2 :
  [ ] src-tauri/src/engines/wolf/injector.rs
  [ ] Tests round-trip Wolf v1/v2
```

---

## Steps

---

### Step 1 — Structs + erreurs injector

**Objectif :** Définir les types de données et erreurs du module injector Wolf.
Pattern identique à `mv_mz/injector.rs` adapté pour le binaire Wolf.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/injector.rs`

**Dépend de :** F4-01 Step 2

**⚠️ Tâche préalable obligatoire — à faire AVANT Step 2 :**
- [ ] Investiguer l'API de `wolfrpg-map-parser` : lire la doc crates.io et les structs publiques.
  `Map::parse()` expose-t-elle des champs `offset`/`position` dans les types retournés ?
  Les deux sources disponibles (docs/wolf-rpg-research.md §5, docs/plans/wolf-rpg-approach.md)
  décrivent l'API comme "structs Rust + sortie JSON" — aucune mention d'offsets binaires.
  Probabilité forte : les offsets ne sont PAS exposés.
  → Si offsets **confirmés exposés** : Approche A valide en Step 2.
  → Si offsets **absents** (défaut attendu) : Approche B par défaut en Step 2.

Tâches :
- [ ] Définir les erreurs :
  ```rust
  #[derive(Debug, thiserror::Error)]
  pub enum InjectorError {
      #[error("key not found in Wolf data: {0}")]
      KeyNotFound(String),
      #[error("encoding error (UTF-8 to Shift-JIS failed): {0}")]
      Encoding(String),
      #[error("string length changed: key {key}, original {original} bytes, new {new} bytes")]
      LengthChanged { key: String, original: usize, new: usize },
      #[error("I/O error: {0}")]
      Io(#[from] std::io::Error),
  }
  ```
- [ ] Définir la struct de traduction (input de l'injection) :
  ```rust
  /// One (key, translated_text) pair to inject.
  pub struct WolfTranslation {
      pub key: String,
      pub text: String,
  }
  ```
- [ ] Définir `pub struct InjectionResult { pub file_path: PathBuf, pub updated_count: usize }`

**Test de validation :**
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Résultat attendu : compile sans erreur

Commit message : `feat(wolf/injector): define InjectorError, WolfTranslation, InjectionResult`

---

### Step 2 — Injecteur .mps (réécriture carte)

**Objectif :** Réécrire un fichier `.mps` avec les traductions. Utiliser les clés générées
par l'extracteur pour localiser exactement les positions binaires à modifier.
**Contrainte absolue : préserver tous les octets non-texte exactement.**

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/injector.rs`

**Dépend de :** Step 1, F4-03 Step 3

**⚠️ Pattern de réécriture — décision conditionnelle à la tâche préalable de Step 1 :**
Deux approches pour retrouver les positions binaires des strings dans le .mps :
- **A** : Stocker les offsets byte exacts dans `ExtractedSegment` — possible SEULEMENT si
  `wolfrpg-map-parser` expose les offsets dans ses structs (à confirmer en Step 1 préalable).
  Avantage : rapide à l'injection. Inconvénient : couplage fort extraction/injection.
- **B** : Re-parser le .mps pour retrouver les positions — indépendant de l'API, plus robuste.
  Avantage : aucune modification de `ExtractedSegment`. Inconvénient : re-lecture des bytes.

**Recommandation : Approche B par défaut.** Les sources disponibles décrivent l'API
wolfrpg-map-parser comme "structs Rust + sortie JSON" sans mention d'offsets binaires.
Adopter A uniquement si Step 1 préalable confirme explicitement les offsets.

Tâches :
- [ ] **(Approche A — seulement si Step 1 préalable confirme les offsets)**
  Modifier `ExtractedSegment` (ou l'équivalent Wolf) pour inclure `byte_offset: u64` et `byte_len: usize`
  — si `ExtractedSegment` est partagé avec MV/MZ, utiliser `Option<u64>` pour backward compat
- [ ] **(Approche A — conditionnel)** Mettre à jour `extract_map()` (F4-03 Step 3) pour stocker
  les offsets binaires si l'API wolfrpg-map-parser les expose
- [ ] **(Approche B — défaut)** Implémenter `fn locate_string_in_mps(bytes: &[u8], key: &str) -> Option<(u64, usize)>` :
  Re-parser les bytes bruts du `.mps` pour retrouver la position et longueur de la string
  correspondant à la clé donnée. Utiliser le même algorithme de traversal que `extract_map()`
  mais ne retourner que l'offset + longueur, sans allouer de texte.
- [ ] Implémenter `pub fn inject_map(bytes: &mut Vec<u8>, translations: &[WolfTranslation], version: &WolfVersion) -> Result<InjectionResult, InjectorError>` :
  1. Pour chaque `WolfTranslation` : localiser la string (via offsets Approche A ou
     `locate_string_in_mps` Approche B)
  2. Re-encoder le texte traduit en Shift-JIS si v2, UTF-8 si v3+
  3. Si la longueur change : ⚠️ les strings dans `.mps` sont à préfixe u32 — réécriture
     complète du buffer nécessaire (pas d'injection in-place pour F4)
- [ ] Tests :
  - `test_inject_map_identity` : traduire avec le texte original → bytes identiques (round-trip identité)
  - `test_inject_map_shorter` : texte plus court → fichier valide (parseable par wolfrpg-map-parser)
  - `test_inject_map_longer` : texte plus long → fichier valide
  - `test_inject_map_wrong_key` : clé inexistante → `Err(KeyNotFound)`

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::injector::tests::test_inject_map
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 4 tests verts, clippy vert

Commit message : `feat(wolf/injector): inject_map — binary .mps rewrite with translation`

---

### Step 3 — Re-encodage UTF-8 → Shift-JIS + gestion d'erreur

**Objectif :** Wrapper dédié pour l'encodage à l'injection. Erreur explicite si un caractère
Unicode ne peut pas être encodé en Shift-JIS (Wolf v2 crasherait avec du texte non-SJIS).

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/injector.rs`

**Dépend de :** F4-03 Step 2 (wolf/encoding.rs)

**⚠️ Critique pour la stabilité des jeux Wolf v2 :**
Si le texte traduit contient des caractères hors Shift-JIS (ex: é, ñ, Ü),
le jeu crashera ou affichera de la bouillie. L'erreur doit être claire et actionnable.

Tâches :
- [ ] Implémenter `fn encode_for_wolf(text: &str, version: &WolfVersion) -> Result<Vec<u8>, InjectorError>` :
  ```rust
  fn encode_for_wolf(text: &str, version: &WolfVersion) -> Result<Vec<u8>, InjectorError> {
      if version.is_utf8() {
          Ok(text.as_bytes().to_vec())
      } else {
          encoding::encode_shiftjis(text)
              .map_err(|e| InjectorError::Encoding(e.to_string()))
      }
  }
  ```
- [ ] Tester les cas d'erreur :
  - `test_encode_french_accents_in_v2` : "café" → `Err(InjectorError::Encoding)` (Wolf v2 = Shift-JIS)
  - `test_encode_french_accents_in_v3` : "café" → OK (Wolf v3+ = UTF-8)
  - `test_encode_ascii_both_versions` : "Hello" → OK dans les deux versions

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::injector::tests::test_encode_for_wolf
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 3 tests verts, clippy vert

Commit message : `feat(wolf/injector): encode_for_wolf — Shift-JIS guard for v2, UTF-8 for v3+`

---

### Step 4 — ⚠️ Injecteur .dat (réécriture database)

**Objectif :** Réécrire les fichiers `.dat` databases avec les traductions. Mirror de
`dat_parser.rs` — lire la structure, remplacer les valeurs texte, réécrire le binaire.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/injector.rs`
- `src-tauri/src/engines/wolf/dat_parser.rs` ← peut nécessiter d'exposer des offsets

**Dépend de :** Step 1, Step 3, F4-03 Step 6

**⚠️ Complexité des longueurs de champs :**
Les strings dans les `.dat` Wolf peuvent avoir :
- Longueur préfixée `u32` : un champ plus long → modifier le préfixe ET décaler tout ce qui suit
- Null-terminated : plus simple, mais moins courant dans Wolf
⚠️ Vérifier dans WolfTL source quel format est utilisé avant d'implémenter.
Si longueur préfixée : l'injection nécessite une re-sérialisation complète du fichier.

Tâches :
- [ ] Implémenter `pub fn inject_dat(bytes: &[u8], translations: &[WolfTranslation], version: &WolfVersion) -> Result<(Vec<u8>, InjectionResult), InjectorError>` :
  1. Parser le .dat avec `dat_parser::parse_dat_file()`
  2. Pour chaque traduction : trouver le champ par clé, remplacer la valeur texte
  3. Re-sérialiser tout le `.dat` (re-écrire complètement — plus simple que modification in-place)
  4. Re-encoder chaque string (Shift-JIS ou UTF-8 selon version)
- [ ] Implémenter `fn serialize_dat(dat: &DatFile, version: &WolfVersion) -> Result<Vec<u8>, InjectorError>` :
  - Sérialiser en binaire dans le même format que l'original
  - Doit produire un fichier identique à l'original si aucune traduction n'est changée
- [ ] Tests :
  - `test_inject_dat_identity` : parse → serialize sans traduction → bytes identiques (round-trip)
  - `test_inject_dat_name` : modifier un champ "name" → nouveau binaire parseable
  - `test_inject_dat_wrong_key` : clé inexistante → `Err(KeyNotFound)`

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::injector::tests::test_inject_dat
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 3 tests verts, clippy vert

Commit message : `feat(wolf/injector): inject_dat — full re-serialization with translated fields`

---

### Step 5 — Stratégie export Option A + inject_all()

**Objectif :** Orchestrateur d'injection qui écrit les fichiers modifiés dans Data/ du jeu
(stratégie Option A : déchiffrés en place). Le moteur Wolf RPG lit Data/ en priorité sur
les archives `.wolf`.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/injector.rs`

**Dépend de :** Step 2, Step 4

**Stratégie Option A (F4) :**
- Ne pas re-chiffrer en `.wolf`
- Écrire les `.mps`/`.dat` traduits directement dans `Data/MapData/` et `Data/BasicData/`
- Si le jeu avait des `.wolf` : les `.wolf` restent sur le disque (le jeu lit Data/ en priorité)
- Documenter dans un commentaire que Option B (re-chiffrement DXA) est prévu pour F5

Tâches :
- [ ] Implémenter `pub async fn inject_all(game_dir: &Path, translations_by_file: &HashMap<String, Vec<WolfTranslation>>, version: &WolfVersion) -> Result<Vec<InjectionResult>, InjectorError>` :
  1. Pour chaque fichier dans `translations_by_file` :
     - Si `.mps` : charger les bytes originaux, appeler `inject_map()`, écrire dans `Data/MapData/`
     - Si `.dat` : charger, `inject_dat()`, écrire dans `Data/BasicData/`
  2. Créer les répertoires `Data/MapData/` et `Data/BasicData/` si inexistants
  3. ⚠️ Ne jamais écraser les `.wolf` originaux — écrire uniquement les `.mps`/`.dat` déchiffrés
  4. Retourner la liste des résultats d'injection
- [ ] Tests :
  - `test_inject_all_creates_files` : injection dans un dossier temporaire → fichiers créés
  - `test_inject_all_does_not_overwrite_wolf` : archive `.wolf` présente → non modifiée

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::injector::tests::test_inject_all
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 2 tests verts, clippy vert

Commit message : `feat(wolf/injector): inject_all() — Option A export (decrypted files in Data/)`

---

### Step 6 — Tests round-trip complet extract → inject

**Objectif :** Valider le round-trip complet : extraire les segments d'un fichier Wolf
synthétique, "traduire" (conserver le texte original), injecter, et vérifier que les
bytes produits sont identiques aux bytes d'origine.

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/` ← tests d'intégration

**Dépend de :** Step 2, Step 4, F4-03 Step 3

Tâches :
- [ ] Test `test_round_trip_mps_identity` :
  1. Créer un `.mps` synthétique avec du texte japonais
  2. `extract_map()` → liste de segments
  3. Créer les `WolfTranslation` avec le texte source (identité)
  4. `inject_map()` → nouveaux bytes
  5. Re-parser avec `wolfrpg-map-parser` → même texte → OK
  6. Bytes identiques si aucun changement de longueur
- [ ] Test `test_round_trip_dat_identity` :
  1. Créer un `.dat` synthétique
  2. `parse_dat_file()` → extraire segments
  3. `inject_dat()` avec textes originaux → bytes identiques
- [ ] Test `test_round_trip_mps_translation` :
  1. `.mps` synthétique avec texte japonais
  2. Extraire, traduire en anglais (chaîne de même longueur ou différente)
  3. Injecter, re-parser → texte anglais présent
- [ ] Documentation dans le fichier des limitations connues :
  - Texte plus long que l'original : re-sérialisation complète (pas d'injection in-place)
  - Wolf v2 : seuls les caractères Shift-JIS sont supportés dans la traduction

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::injector::tests
cargo test --manifest-path src-tauri/Cargo.toml engines::wolf::extractor::tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : tous les tests injector verts (au moins 12), tous les tests extractor verts, clippy vert

Commit message : `test(wolf): round-trip extract→inject for .mps and .dat — identity + translation`

---

## Tests obligatoires avant push GitHub

```bash
# Rust — tous les tests (injector + extractor + decryptor + fondations)
cargo test --manifest-path src-tauri/Cargo.toml
# Attendu : 100% verts

# Rust — qualité code
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
# Attendu : 0 warning, 0 erreur

# Rust — formatage
cargo fmt --manifest-path src-tauri/Cargo.toml
# Attendu : aucun fichier modifié

# TypeScript — aucun fichier TS modifié
pnpm typecheck
# Attendu : 0 erreur
```

## Commandes git

```bash
git checkout -b feat/f4-04-wolf-injector
# ... commits intermédiaires par step ...
git checkout main
git merge --no-ff feat/f4-04-wolf-injector -m "feat(f4-04): Wolf RPG injector — .mps + .dat rewrite, Option A export, round-trip tests"
git push origin main
git branch -d feat/f4-04-wolf-injector
```

## Mise à jour après complétion

- `ROADMAP.md` : cocher `src-tauri/src/engines/wolf/injector.rs`
- `CHANGELOG.md` : entrée `Added` Wolf injector + round-trip validation
- `docs/journal/` : nouvelle entrée `YYYY-MM-DD-f4-04-wolf-injector.md`
- `docs/engines.md` : documenter la limitation Option A (pas de re-chiffrement en F4)
- Mettre à jour la mémoire session : F4-04 terminé, passer à F4-05
