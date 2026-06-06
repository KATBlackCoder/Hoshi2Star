# Plan F4-01 — Wolf RPG : Fondations

## Objectif

Poser les fondations du module Wolf RPG : ajout des dépendances Cargo, création du module
`engines/wolf/`, détection du moteur dans `detector.rs`, et extension de `llm/tokenizer.rs`
avec les patterns de placeholders spécifiques à Wolf RPG.

## Statut : [ ] À faire

## Prérequis

- F3 complet (v0.3.2 taggé) — `[x]` Fait
- `detector.rs` existant (Engine::MvMz + VxAce désactivé) ✅
- `tokenizer.rs` existant (Engine::MvMz + MzOnly) ✅
- `docs/plans/wolf-rpg-approach.md` validé ✅
- Aucune autre tâche F4 active

## Estimation

7 steps · ~1–2 jours

## Items ROADMAP concernés

```
F4 — Engine Layer — Wolf RPG v1/v2 :
  [ ] src-tauri/src/engines/wolf/extractor.rs
  [ ] src-tauri/src/engines/wolf/decryptor.rs
  [ ] src-tauri/src/engines/wolf/injector.rs
```

---

## Steps

---

### Step 1 — Vérifier versions crates.io + ajouter dépendances Cargo.toml

**Objectif :** Ajouter `encoding_rs` (Shift-JIS ↔ UTF-8) et `wolfrpg-map-parser` (parsing .mps)
à `src-tauri/Cargo.toml`. Vérifier les versions actuelles avant d'écrire.

**Fichiers touchés :**
- `src-tauri/Cargo.toml`

**Dépend de :** *(aucun)*

Tâches :
- [ ] Rechercher la version actuelle de `encoding_rs` sur crates.io
  - Attendu : `0.8.x` (Mozilla, MIT/Apache-2.0, très stable)
  - Commande : `cargo search encoding_rs` OU consulter crates.io
- [ ] Rechercher la version actuelle de `wolfrpg-map-parser` sur crates.io
  - Attendu : `0.6.x` (MIT, G1org1owo, ~38 dl/mois)
  - ⚠️ Adoption faible — noter la version exacte dans ce plan une fois vérifiée
- [ ] Ajouter dans `[dependencies]` de `Cargo.toml` :
  ```toml
  encoding_rs = "0.8"
  wolfrpg-map-parser = "0.6"
  ```
- [ ] Lancer `cargo check` pour vérifier que les deux crates se résolvent sans conflit

**Test de validation :**
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Résultat attendu : compile sans erreur, `Cargo.lock` mis à jour avec les deux nouvelles dépendances

Commit message : `chore(deps): add encoding_rs + wolfrpg-map-parser for Wolf RPG F4`

---

### Step 2 — Créer src-tauri/src/engines/wolf/mod.rs

**Objectif :** Créer le module `wolf` dans la couche engines, en suivant exactement le même
pattern que `engines/mv_mz/mod.rs`. Déclarer les 3 sous-modules (même s'ils sont vides).

**Fichiers touchés :**
- `src-tauri/src/engines/wolf/mod.rs` ← créer
- `src-tauri/src/engines/mod.rs` ← ajouter `pub mod wolf;`

**Dépend de :** *(aucun)*

**Note :** Vérifier d'abord si `engines/mod.rs` existe ou si les modules engines sont déclarés
directement dans `lib.rs`. Adapter en conséquence (ne pas créer de doublon).

Tâches :
- [ ] Créer `src-tauri/src/engines/wolf/mod.rs` :
  ```rust
  pub mod decryptor;
  pub mod extractor;
  pub mod injector;
  ```
- [ ] Créer `src-tauri/src/engines/wolf/decryptor.rs` (stub) :
  ```rust
  // Wolf RPG DXA decryptor — implementation in F4-02
  ```
- [ ] Créer `src-tauri/src/engines/wolf/extractor.rs` (stub) :
  ```rust
  // Wolf RPG text extractor — implementation in F4-03
  ```
- [ ] Créer `src-tauri/src/engines/wolf/injector.rs` (stub) :
  ```rust
  // Wolf RPG binary injector — implementation in F4-04
  ```
- [ ] Trouver où `engines/mv_mz` est déclaré (`mod mv_mz;`) et ajouter `pub mod wolf;` au même endroit

**Test de validation :**
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Résultat attendu : compile sans erreur (stubs vides acceptés par Rust)

Commit message : `feat(engines): scaffold wolf/ module — empty stubs for decryptor/extractor/injector`

---

### Step 3 — Ajouter Engine::Wolf dans l'enum detector.rs

**Objectif :** Étendre l'enum `Engine` avec la variante `Wolf` et ajouter les fonctions
helper de détection sans encore modifier `detect_engine()` (ça, c'est Step 4).

**Fichiers touchés :**
- `src-tauri/src/engines/detector.rs`

**Dépend de :** Step 2

Tâches :
- [ ] Ajouter `Wolf` à l'enum `Engine` :
  ```rust
  pub enum Engine {
      MvMz,
      VxAce,
      Wolf,
  }
  ```
- [ ] Créer la fonction helper pure `is_wolf_game_dir(game_dir: &Path) -> bool` :
  ```rust
  // Returns true if the directory looks like a Wolf RPG game root.
  // Criteria:
  //   - Game.exe OR Game.ini present at root
  //   - AND (BasicData/ directory OR Data/ with *.wolf OR Data/ with *.mps)
  pub fn is_wolf_game_dir(game_dir: &Path) -> bool {
      let has_launcher = game_dir.join("Game.exe").exists()
          || game_dir.join("Game.ini").exists();
      if !has_launcher {
          return false;
      }
      game_dir.join("BasicData").is_dir()
          || has_wolf_archives(game_dir)
          || has_mps_files(game_dir)
  }

  fn has_wolf_archives(game_dir: &Path) -> bool {
      let data_dir = game_dir.join("Data");
      if !data_dir.is_dir() { return false; }
      std::fs::read_dir(&data_dir)
          .ok()
          .map(|entries| entries
              .filter_map(|e| e.ok())
              .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("wolf")))
          .unwrap_or(false)
  }

  fn has_mps_files(game_dir: &Path) -> bool {
      let map_dir = game_dir.join("Data").join("MapData");
      if map_dir.is_dir() {
          return std::fs::read_dir(&map_dir)
              .ok()
              .map(|entries| entries
                  .filter_map(|e| e.ok())
                  .any(|e| e.path().extension().and_then(|x| x.to_str()) == Some("mps")))
              .unwrap_or(false);
      }
      false
  }
  ```
- [ ] Corriger le(s) `match engine` existant(s) dans `detector.rs` qui ne couvre plus tous les cas
  — ajouter `Engine::Wolf => unreachable!()` provisoirement si nécessaire

**Test de validation :**
```bash
cargo check --manifest-path src-tauri/Cargo.toml
```
Résultat attendu : compile sans erreur, clippy vert

Commit message : `feat(engines): add Engine::Wolf + is_wolf_game_dir helper (detection helpers)`

---

### Step 4 — Brancher Engine::Wolf dans detect_engine() + tests

**Objectif :** Modifier `detect_engine()` pour appeler `is_wolf_game_dir()` après MV/MZ (en
position 2). Ajouter les tests unitaires pour la détection Wolf. Corriger tous les `match
engine` impactés dans le projet (compiler les erreurs sont les guides).

**Fichiers touchés :**
- `src-tauri/src/engines/detector.rs`
- `src-tauri/src/commands/project.rs` ← mise à jour `match engine` provisoire

**Dépend de :** Step 3

Tâches :
- [ ] Modifier `detect_engine()` — insérer le check Wolf **après** MV/MZ, **avant** VX Ace
  (désactivé) :
  ```rust
  // 1. MV/MZ (inchangé)
  if let Some(data_dir) = find_data_dir(game_dir) { ... }

  // 2. Wolf RPG — Game.exe/Game.ini + BasicData/ or .wolf/.mps files
  if is_wolf_game_dir(game_dir) {
      return Ok(Engine::Wolf);
  }

  // 3. VX Ace (disabled)
  // if let Some(vx_dir) = ...
  ```
- [ ] Mettre à jour `commands/project.rs` : ajouter un bras `Engine::Wolf` provisoire dans le
  `match engine` de `open_project` :
  ```rust
  Engine::Wolf => {
      return Err("Wolf RPG extraction not yet implemented (F4-03)".to_string());
  }
  ```
  — Cela évite une erreur de compilation sans implémenter prématurément.
  Même chose pour l'engine_str :
  ```rust
  Engine::Wolf => "wolf",
  ```
- [ ] Tests unitaires dans `detector.rs` :
  - `test_detect_wolf_with_game_exe_and_basic_data` : `Game.exe` + `BasicData/` → `Engine::Wolf`
  - `test_detect_wolf_with_game_exe_and_wolf_archives` : `Game.exe` + `Data/*.wolf` → `Engine::Wolf`
  - `test_detect_wolf_with_game_ini` : `Game.ini` + `BasicData/` → `Engine::Wolf`
  - `test_detect_wolf_no_launcher` : `BasicData/` seul sans `Game.exe` → `UnknownEngine`
  - `test_detect_mv_not_confused_with_wolf` : `data/System.json` avec `gameTitle` → `MvMz` (pas Wolf)
  - `test_detect_wolf_not_confused_with_mv` : `Game.exe` + `BasicData/` mais PAS `data/System.json` → `Wolf`

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::detector::tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 6+ nouveaux tests Wolf passent, tous les tests existants encore verts

Commit message : `feat(engines): detect_engine() — add Wolf RPG detection (step 2, after MV/MZ)`

---

### Step 5 — WolfVersion struct + find_wolf_data_dir()

**Objectif :** Définir `WolfVersion { major, minor }` et `find_wolf_data_dir()` (analogue de
`find_data_dir()` pour MV/MZ). La version déterminera l'encodage utilisé lors de
l'extraction et de l'injection.

**Fichiers touchés :**
- `src-tauri/src/engines/detector.rs`

**Dépend de :** Step 3

**Note :** La détection de version réelle (depuis l'en-tête DXA CodePage) sera finalisée en
F4-02. Ici on définit les types et une heuristique simple basée sur la structure de fichiers.

Tâches :
- [ ] Définir la struct :
  ```rust
  #[derive(Debug, Clone, PartialEq, Eq)]
  pub struct WolfVersion {
      pub major: u8,
      pub minor: u8,
  }

  impl WolfVersion {
      pub fn is_utf8(&self) -> bool {
          self.major >= 3
      }
  }
  ```
- [ ] Créer `find_wolf_data_dir(game_dir: &Path) -> Option<PathBuf>` :
  - Cherche `Data/` (capital D, Windows) puis `data/` (lowercase, Linux)
  - Note : Wolf RPG utilise `Data/` et non `data/` comme MV/MZ
  ```rust
  pub fn find_wolf_data_dir(game_dir: &Path) -> Option<std::path::PathBuf> {
      let candidates = [game_dir.join("Data"), game_dir.join("data")];
      candidates.into_iter().find(|p| p.is_dir())
  }
  ```
- [ ] Créer `guess_wolf_version_from_structure(game_dir: &Path) -> WolfVersion` :
  - Heuristique : si `Data/MapData/*.mps` sont lisibles comme UTF-8 → v3+ ; sinon v2
  - Pour F4, une implémentation simple qui retourne `WolfVersion { major: 2, minor: 0 }`
    par défaut est acceptable — la version exacte sera lue depuis le DXA CodePage en F4-02
  ```rust
  pub fn guess_wolf_version_from_structure(_game_dir: &Path) -> WolfVersion {
      // TODO(F4-02): read exact version from DXA header CodePage field.
      // Default to v2 (Shift-JIS) — safe conservative assumption.
      WolfVersion { major: 2, minor: 0 }
  }
  ```
- [ ] Tests unitaires :
  - `test_wolf_version_is_utf8` : `WolfVersion { major: 3, minor: 0 }.is_utf8()` → `true`
  - `test_wolf_version_is_shiftjis` : `WolfVersion { major: 2, minor: 0 }.is_utf8()` → `false`
  - `test_find_wolf_data_dir_capital_d` : `Data/` exists → retourne `Data/`
  - `test_find_wolf_data_dir_lowercase_fallback` : `data/` only → retourne `data/`

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml engines::detector::tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 4 nouveaux tests verts, clippy vert

Commit message : `feat(engines): WolfVersion struct + find_wolf_data_dir + version heuristic`

---

### Step 6 — Engine::Wolf dans tokenizer.rs — patterns regex

**Objectif :** Ajouter le mode `Engine::Wolf` dans `llm/tokenizer.rs` avec une `Regex`
dédiée couvrant tous les placeholders Wolf RPG documentés dans `wolf-rpg-approach.md`.
Les patterns doivent être ordonnés pour éviter les correspondances partielles.

**Fichiers touchés :**
- `src-tauri/src/llm/tokenizer.rs`

**Dépend de :** Step 1 (aucune dépendance externe — la regex est pure Rust)

**Règles de priorité dans la regex :**
1. `\r[Base,Ruby]` — PREMIER (contient virgule, sinon capturé partiellement)
2. DB refs 3 params `\udb[:]`, `\cdb[:]`, `\sdb[:]` — avant les codes simples
3. `\sysS[n]` — avant `\sys[n]` (préfixe plus long d'abord)
4. `\cself[n]` — avant `\self[n]`
5. `\space[n]` — avant `\sp[n]`
6. `\v?[n]` — le `?` est littéral ici (pas quantificateur regex)
7. Codes standard `\v[n]`, `\c[n]`, `\s[n]`, `\f[n]`, `\i[n]`
8. `\m[n]` — séparé (m absent du groupe ci-dessus pour éviter la duplication avec `\f[n]`)
9. Codes multi-char `\mx[n]`, `\my[n]`, `\ax[n]`, `\ay[n]`, `\sp[n]`, `\-[n]`, `\font[n]`
10. Alignement `<L>`, `<C>`, `<R>`
11. Codes sans argument `\E`, `\N`, `\A+`, `\A-`, `\\`, `\!`, `\.`, `\^`, `\>`, `\<`
    ⚠️ `\!`, `\.`, `\^`, `\>`, `\<` : contrôle d'affichage Wolf (pause, attente, fin forcée,
    affichage instantané) — présents dans wolf-rpg-research.md §4, absents de MV/MZ RE_WOLF
    car RE_WOLF est standalone (pas extension de RE_MVMZ)
12. Newline littéral `\n`

Tâches :
- [ ] Ajouter la variante `Wolf` à l'enum `Engine` dans `tokenizer.rs` :
  ```rust
  pub enum Engine {
      MvMz,
      MzOnly,
      Wolf,
  }
  ```
- [ ] Créer `static RE_WOLF: LazyLock<Regex>` :
  ```rust
  static RE_WOLF: LazyLock<Regex> = LazyLock::new(|| {
      Regex::new(
          r"(?x)
            \\r\[[^\[\],]+,[^\[\]]*\]           # \r[Base,Ruby] — ruby opaque (PREMIER)
          | \\(?:udb|cdb|sdb)\[\d+:\d+:\d+\]   # DB refs 3 params (avant codes simples)
          | \\sysS\[\d+\]                        # system string (avant \sys)
          | \\cself\[\d{1,2}\]                   # common-event self (avant \self)
          | \\self\[\d\]                         # self variable événement
          | \\sys\[\d+\]                         # system variable
          | \\space\[\d+\]                       # line height (avant \sp)
          | \\v\?\[\d+\]                         # reserve variable \v?[n]
          | \\(?:sp|mx|my|ax|ay)\[\d+\]          # speed/offset codes
          | \\-\[\d+\]                           # pixel spacing
          | \\font\[\d\]                         # sub-font
          | \\[vcsfiVCSFI]\[\d+\]                # standard codes v/c/s/f/i (maj+min)
          | \\m\[\d+\]                           # max line (\m[n] — m absent du groupe ci-dessus)
          | <[LCR]>                              # alignment tags
          | \\A[+\-]                             # anti-aliasing
          | \\[EN\\!.^><]                        # no-arg codes: \E \N \\ \! \. \^ \> \<
          | \n                                   # literal newline
          ",
      )
      .expect("RE_WOLF regex must compile")
  });
  ```
- [ ] Brancher `Engine::Wolf` dans `Tokenizer::tokenize()` :
  ```rust
  Engine::Wolf => &*RE_WOLF,
  ```
- [ ] Brancher `Engine::Wolf` dans `core/qa.rs` → `Tokenizer::tokenize(source, TokEngine::MvMz)`
  — pour l'instant garder `MvMz` pour le check QA Wolf (étendre en F4-05)

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml llm::tokenizer::tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : tests existants verts, clippy vert, 0 compilation error

Commit message : `feat(tokenizer): add Engine::Wolf mode with Wolf RPG placeholder patterns`

---

### Step 7 — Tests unitaires tokenizer Wolf

**Objectif :** Valider chaque nouveau pattern Wolf individuellement + tester la non-régression
des patterns MV/MZ existants. Round-trip tokenize→restore pour chaque famille de patterns.

**Fichiers touchés :**
- `src-tauri/src/llm/tokenizer.rs` ← section `#[cfg(test)]`

**Dépend de :** Step 6

Tâches :
- [ ] Test `test_wolf_ruby_opaque` : `\r[魔法,まほう]` → 1 token, round-trip OK
- [ ] Test `test_wolf_ruby_with_text` : `\r[魔法,まほう]が使えます` → 1 token, texte `が使えます` préservé
- [ ] Test `test_wolf_db_refs` : `\udb[0:1:2]`, `\cdb[3:4:5]`, `\sdb[6:7:8]` → 3 tokens, round-trip OK
- [ ] Test `test_wolf_sysS_before_sys` : `\sysS[10]` → 1 token `\sysS[10]` (pas `\sys[10]`)
- [ ] Test `test_wolf_cself_before_self` : `\cself[99]` → 1 token `\cself[99]` (pas `\self[9]`)
- [ ] Test `test_wolf_reserve_variable` : `\v?[30]` → 1 token (le `?` est littéral dans Wolf)
- [ ] Test `test_wolf_alignment_tags` : `<L>テキスト<R>` → 2 tokens, texte `テキスト` préservé
- [ ] Test `test_wolf_standard_codes` : `\v[5]\c[3]\s[10]` → 3 tokens, round-trip OK
- [ ] Test `test_wolf_no_arg_codes` : `\E\N\A+\A-` → 4 tokens, round-trip OK
- [ ] Test `test_wolf_multiline` : `テキスト\nつぎの行` → 1 token (newline), round-trip OK
- [ ] Test `test_wolf_no_interference_with_mvmz` : tokeniser `\V[12]` avec `Engine::MvMz`
  → 1 token (non-régression, MV/MZ inchangé)

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml llm::tokenizer::tests::test_wolf
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 11 nouveaux tests Wolf verts, tous les 14 tests MV/MZ existants encore verts

Commit message : `test(tokenizer): Wolf RPG placeholder patterns — 11 tests (ruby, DB refs, alignment, etc.)`

---

## Tests obligatoires avant push GitHub

```bash
# Rust — tous les tests
cargo test --manifest-path src-tauri/Cargo.toml
# Attendu : 100% verts (anciens + nouveaux)

# Rust — qualité code
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
# Attendu : 0 warning, 0 erreur

# Rust — formatage
cargo fmt --manifest-path src-tauri/Cargo.toml
# Attendu : aucun fichier modifié

# TypeScript — types (aucun fichier TS modifié en F4-01)
pnpm typecheck
# Attendu : 0 erreur
```

## Commandes git

```bash
git checkout -b feat/f4-01-wolf-foundations
# ... commits intermédiaires par step ...
git checkout main
git merge --no-ff feat/f4-01-wolf-foundations -m "feat(f4-01): Wolf RPG foundations — Engine::Wolf, detector, tokenizer patterns"
git push origin main
git branch -d feat/f4-01-wolf-foundations
```

## Mise à jour après complétion

- `ROADMAP.md` : démarrer les cases F4 Wolf RPG (à cocher au fur et à mesure)
- `CHANGELOG.md` : section `[Unreleased]` → entrée `Added` Wolf RPG F4-01 fondations
- `docs/journal/` : nouvelle entrée `YYYY-MM-DD-f4-01-wolf-foundations.md`
- Mettre à jour la mémoire session : F4-01 terminé, passer à F4-02
