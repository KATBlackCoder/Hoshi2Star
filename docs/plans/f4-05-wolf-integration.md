# Plan F4-05 — Wolf RPG : Intégration pipeline + UI + tests end-to-end

## Objectif

Intégrer le moteur Wolf RPG dans le pipeline complet de Hoshi2Star :
commandes Tauri (`open_project`, `export_project`), QA engine, message WolfX,
icônes FileTree, documentation architecture. Puis valider sur les deux jeux de test locaux :
月咲流ホノカ ver1.03 (clé directe, DXA v8 monolithique) et Densyanai Inko ver2.0 (GuessKeyV6,
13 archives).

C'est l'étape de soudure — le moteur Wolf est techniquement complet (F4-01 à 04),
ici on l'expose via l'IPC et on valide que l'app fonctionne de bout en bout.

## Statut : [x] COMPLET — 2026-06-07

## Prérequis

- F4-01 complet (fondations + Engine::Wolf stub dans project.rs) — `[ ]` À faire
- F4-02 complet (décrypteur) — `[ ]` À faire
- F4-03 complet (extracteur) — `[ ]` À faire
- F4-04 complet (injecteur + round-trip) — `[ ]` À faire

## Estimation

8 steps · ~2–3 jours

## Items ROADMAP concernés

```
F4 — Engine Layer — Wolf RPG v1/v2 :
  [ ] Tests round-trip Wolf v1/v2
F4 — Monétisation :
  (Aucun item F4-05 directement lié — préparation infrastructure)
```

---

## Steps

---

### Step 1 — commands/project.rs — open_project Wolf branch

**Objectif :** Remplacer le bras provisoire `Engine::Wolf => Err(...)` dans `open_project`
par la vraie intégration : détection version, extraction complète, insertion en DB.
Adapter le `dispatch_extract` pattern de MV/MZ pour Wolf.

**Fichiers touchés :**
- `src-tauri/src/commands/project.rs`

**Dépend de :** F4-03 Step 8 (`extract_all_wolf`)

Tâches :
- [ ] Ajouter l'import Wolf :
  ```rust
  use crate::engines::wolf::{extractor as wolf_extractor, detector::detect_wolf_version};
  ```
- [ ] Mettre à jour `engine_str` pour Wolf :
  ```rust
  Engine::Wolf => "wolf",
  ```
- [ ] Mettre à jour `find_data_dir` / data directory lookup pour Wolf :
  ```rust
  Engine::Wolf => find_wolf_data_dir(game_dir)
      .ok_or_else(|| "Cannot find Data/ directory in Wolf RPG game folder".to_string())?,
  ```
- [ ] Lire le titre du jeu Wolf (depuis `Game.ini` section `[GameTitle]` ou fallback nom dossier) :
  ```rust
  Engine::Wolf => read_wolf_game_title(game_dir).unwrap_or_else(|| folder_name),
  ```
  ⚠️ `read_wolf_game_title` : lire `Game.ini` s'il existe (format `.ini` Windows), sinon
  retourner `None` et utiliser le nom du dossier.
- [ ] Remplacer le bras Wolf dans le `match engine { ... }` de la boucle d'insertion :
  ```rust
  Engine::Wolf => {
      let wolf_version = detect_wolf_version(game_dir);
      let entries = wolf_extractor::extract_all_wolf(game_dir, &wolf_version)
          .map_err(|e| e.to_string())?;
      for (file_name, file_path, segments) in &entries {
          // INSERT source_files + segments (même logique que MV/MZ)
      }
  }
  ```
- [ ] Tests (manuels — pas de test unitaire automatisé pour la command Tauri complète) :
  - Ouvrir un dossier Wolf synthétique → pas d'erreur → projet créé en DB

**Test de validation :**
```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : compile sans erreur, clippy vert

Commit message : `feat(commands): open_project — integrate Engine::Wolf extraction (F4-03)`

---

### Step 2 — commands/export.rs — export_project Wolf

**Objectif :** Étendre `export_project` pour gérer `Engine::Wolf` — appeler l'injecteur Wolf
avec les traductions depuis la DB.

**Fichiers touchés :**
- `src-tauri/src/commands/export.rs`

**Dépend de :** Step 1, F4-04 Step 5 (`inject_all`)

Tâches :
- [ ] Lire le code de `export_project` dans `export.rs` pour comprendre le pattern existant
  - Pour MV/MZ : lit les segments traduits depuis DB, recharge les JSON originaux, injecte, écrit
  - Pour Wolf : même logique mais appelle `wolf::injector::inject_all()`
- [ ] Ajouter dans le `match engine` de `export_project` :
  ```rust
  Engine::Wolf => {
      // 1. Grouper les segments traduits par fichier source
      // 2. Détecter la version Wolf
      // 3. Appeler inject_all(game_dir, &translations_by_file, &version)
      // 4. Retourner le compte de fichiers exportés
  }
  ```
- [ ] Gérer le cas des segments non traduits (status = 'untranslated') :
  - Comportement cohérent avec MV/MZ : skip les segments non traduits, garder le texte source
- [ ] Test manuel : export_project sur un projet Wolf extrait → fichiers .mps/.dat créés dans Data/

**Test de validation :**
```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : compile, clippy vert

Commit message : `feat(commands): export_project — Engine::Wolf calls inject_all (Option A)`

---

### Step 3 — core/qa.rs — LineWidthConfig Wolf

**Objectif :** Étendre le QA engine pour Wolf RPG. Wolf a ses propres limites de boîte
de dialogue (différentes de MV/MZ). Mettre à jour la fonction `check()` pour utiliser
la configuration Wolf quand le moteur est Wolf.

**Fichiers touchés :**
- `src-tauri/src/core/qa.rs`

**Dépend de :** F4-01 Step 6 (Engine::Wolf dans tokenizer)

**⚠️ Limites de boîte Wolf RPG à investiguer :**
Wolf RPG utilise une boîte de dialogue dont la taille est configurable dans l'éditeur.
La valeur par défaut n'est pas documentée précisément dans les sources disponibles.
Valeur conservative estimée : ~40–45 caractères demi-largeur (à vérifier dans WolfTL
ou dans les fichiers de config d'un jeu Wolf v2 freeware).

Tâches :
- [ ] Ajouter une config Wolf dans `LineWidthConfig` ou une constante séparée :
  ```rust
  impl LineWidthConfig {
      /// Default config for Wolf RPG message boxes.
      /// Box width is typically ~520 px with default font (Wolf v2 default settings).
      /// ⚠️ This value is a conservative estimate — verify from Wolf RPG editor defaults.
      pub fn wolf_default() -> Self {
          Self {
              box_width_px: 520.0,   // estimated Wolf default
              fullwidth_char_px: 26.0,
              halfwidth_char_px: 13.0,
              max_lines: 4,
          }
      }
  }
  ```
- [ ] Modifier `check()` pour accepter un `Option<&LineWidthConfig>` OU ajouter un paramètre engine :
  - Solution la plus simple pour F4 : ajouter un paramètre `engine: TokEngine` à `check()` et
    utiliser la config appropriée en interne
  - Mettre à jour tous les appels existants à `check()` avec `TokEngine::MvMz`
- [ ] Mettre à jour le check placeholder dans `check()` pour utiliser `engine` au lieu de `TokEngine::MvMz` hardcodé
- [ ] Tests :
  - `test_wolf_line_too_long_with_wolf_config` : ligne de 30 kanji = 60 units > 40 max Wolf → erreur
  - `test_wolf_line_ok` : ligne de 20 kanji = 40 units ≤ 40 max Wolf → pas d'erreur
  - Non-régression : tests MV/MZ existants inchangés

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml core::qa::tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 2 nouveaux tests verts + tous les tests QA existants verts, clippy vert

Commit message : `feat(qa): LineWidthConfig::wolf_default() + Engine::Wolf in check() placeholder scan`

---

### Step 4 — Message d'erreur WolfX/v3.5+

**Objectif :** Détecter la version WolfX (v3.5+) au moment de l'ouverture du projet et
retourner une erreur claire à l'utilisateur. L'app ne doit jamais silencieusement échouer
sur un jeu WolfX — le message doit être actionnable.

**Fichiers touchés :**
- `src-tauri/src/engines/detector.rs` ← détecter WolfX
- `src-tauri/src/commands/project.rs` ← vérifier avant d'extraire

**Dépend de :** Step 1, F4-02 Step 5 (CodePage detection)

**Comment détecter WolfX :**
- Si `Game.exe` existe et version PE FileVersion ≥ 3.5
- OU si le décrypteur retourne `Err(DecryptorError::UnsupportedChaCha20)` lors de l'ouverture
  (il faudra ajouter cette variante d'erreur en F4-02 si ce n'est pas fait)
- Pour F4 : si la version détectée via DXA CodePage échoue avec les clés XOR connues
  et GuessKey → probable WolfX → retourner l'erreur

Tâches :
- [ ] Ajouter `DecryptorError::PossibleWolfX` dans F4-02 (ou l'ajouter ici si F4-02 est terminé) :
  ```rust
  #[error("archive may use WolfX encryption (v3.5+) which is not supported in F4")]
  PossibleWolfX,
  ```
- [ ] Dans `commands/project.rs`, dans le bras `Engine::Wolf` de `open_project` :
  ```rust
  if let Err(e) = result {
      if matches!(e, DecryptorError::PossibleWolfX) {
          return Err(
              "This game uses Wolf RPG v3.5+ (WolfX) which is not yet supported. \
               Support is planned for a future version (v0.5.0).".to_string()
          );
      }
  }
  ```
- [ ] Toast UI : ce message d'erreur sera affiché par le frontend dans `useProjectStore` →
  déjà géré par le `invoke` error handling existant dans `openProject` thunk
- [ ] Tests :
  - `test_wolfx_error_message` : projet WolfX → `Err` avec le message exact attendu
    (test via `assert!(err.contains("WolfX"))`)

**Test de validation :**
```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : compile, clippy vert, message d'erreur dans le code

Commit message : `feat(commands): WolfX/v3.5+ detection — explicit error with v0.5.0 roadmap message`

---

### Step 5 — FileTree — icônes spécifiques Wolf

**Objectif :** Afficher des icônes distinctives pour les fichiers `.mps` et `.dat` Wolf RPG
dans le FileTree. Utiliser une couleur amber (ou une nouvelle couleur) distincte de MV/MZ.

**Fichiers touchés :**
- `src/components/editor/FileTree.tsx` ← ajouter détection Wolf par extension
- `src/lib/format.ts` ← mettre à jour `engineLabel()` pour Wolf

**Dépend de :** *(aucune dépendance Rust — pur TypeScript)*

Tâches :
- [ ] Dans `FileTree.tsx`, ajouter la détection du type de fichier Wolf :
  - `.mps` → icône carte (ex: `Map` de lucide) + couleur `text-amber-500`
  - `.dat` → icône base de données (ex: `Database`) + couleur `text-amber-400`
  - `.wolf` → icône archive (ex: `Archive`) + couleur `text-amber-300` (si jamais affiché)
- [ ] Mettre à jour `engineLabel()` dans `format.ts` :
  ```typescript
  case 'wolf': return 'Wolf RPG'
  ```
- [ ] Vérifier que `engine: 'wolf'` depuis la DB est affiché correctement dans le badge
  projet dans `AppToolbar`
- [ ] Tests TypeScript :
  ```bash
  pnpm typecheck
  pnpm lint
  ```

**Test de validation :**
```bash
pnpm typecheck
pnpm lint
```
Résultat attendu : 0 erreur TS, 0 erreur lint

Commit message : `feat(ui): FileTree — Wolf RPG icons (.mps amber, .dat amber) + engineLabel`

---

### Step 6 — docs/architecture.md — mise à jour Wolf RPG

**Objectif :** Documenter le module `engines/wolf/` dans `docs/architecture.md`. Mettre à
jour la table des moteurs supportés et le diagramme d'architecture.

**Fichiers touchés :**
- `docs/architecture.md`

**Dépend de :** *(aucune dépendance code)*

Tâches :
- [ ] Mettre à jour la section `engines/` du tableau de l'architecture :
  | Fichier / Dossier | Rôle |
  | `wolf/decryptor.rs` | Déchiffrement DXA XOR 12 octets. Clés hardcodées v2.01/v2.10/v2.20/no_key. GuessKeyV6. |
  | `wolf/extractor.rs` | Extraction texte : .mps via wolfrpg-map-parser, .dat via portage WolfTL. Encodage SJIS/UTF-8. |
  | `wolf/injector.rs` | Injection binaire .mps et .dat. Stratégie Option A (fichiers déchiffrés). |
  | `wolf/encoding.rs` | decode_shiftjis / encode_shiftjis via encoding_rs. Critique pour v2. |
  | `wolf/dat_parser.rs` | Parser binaire .dat databases + CommonEvents. |
- [ ] Mettre à jour le diagramme "Vue d'ensemble" pour inclure `wolf/`
- [ ] Mettre à jour la section "Ce qui n'est PAS dans cette version" (retirer Wolf RPG)
- [ ] Ajouter une note sur WolfX/v3.5+ : non supporté, prévu en F5
- [ ] Mettre à jour la date "Dernière mise à jour" en haut du fichier

**Test de validation :**
```bash
# Vérification manuelle : lire docs/architecture.md et vérifier la cohérence
```
Résultat attendu : toutes les sections Wolf sont documentées, aucune référence obsolète

Commit message : `docs(architecture): add Wolf RPG engine layer — wolf/mod, decryptor, extractor, injector`

---

### Step 7 — Tests end-to-end sur les jeux de test locaux

**Objectif :** Valider le pipeline complet sur les deux vrais jeux Wolf RPG disponibles dans
`docs/games/`. Ce test est **manuel** — il ne peut pas être automatisé.

**Jeux de test disponibles localement :**
- `docs/games/月咲流ホノカver1.03/` — Wolf v2.255, DXA v8, clé connue, archive monolithique
- `docs/games/Densyanai Inko ver2.0/` — Wolf v2 probable, DXA v8, clé inconnue (GuessKeyV6), 13 archives

⚠️ Ces jeux sont exclus du repo via `docs/games/*` dans `.gitignore`. Ne jamais commiter.

**Fichiers touchés :** *(aucun fichier code — validation manuelle)*

**Dépend de :** Step 1, Step 2 (open_project + export_project Wolf opérationnels)

**Procédure de test — 月咲流ホノカ (clé directe) :**

1. Lancer `pnpm tauri dev`
2. "Open Project" → sélectionner `docs/games/月咲流ホノカver1.03/`
3. Vérifier :
   - [ ] Projet s'ouvre sans erreur (clé v2.255 trouvée dans `WOLF_KEYS`)
   - [ ] FileTree affiche les fichiers `.mps` et `.dat` avec icônes Wolf amber
   - [ ] Cliquer un fichier `.mps` → segments japonais visibles dans SegmentGrid
   - [ ] Cliquer un fichier `.dat` → noms/descriptions japonais visibles
   - [ ] Un segment traduit manuellement → QA score affiché
4. "Export All" :
   - [ ] Aucune erreur d'export
   - [ ] Fichiers `.mps`/`.dat` créés dans `Data/MapData/` et `Data/BasicData/`
   - [ ] Lancer le jeu → démarre sans crash

**Procédure de test — Densyanai Inko (GuessKeyV6) :**

1. "Open Project" → sélectionner `docs/games/Densyanai Inko ver2.0/`
2. Vérifier :
   - [ ] GuessKeyV6 trouve la clé automatiquement (aucune clé hardcodée pour ce jeu)
   - [ ] Les 13 archives `.wolf` sont toutes décryptées
   - [ ] `BasicData.wolf` (10KB) → segments de base de données extraits correctement
   - [ ] `MapData.wolf` → segments de carte (`.mps`) visibles
3. "Export All" :
   - [ ] Fichiers créés dans `Data/MapData/` et `Data/BasicData/`
   - [ ] Lancer le jeu → démarre sans crash

**Test de validation :** (manuel — pas de `cargo test`)

Documenter les résultats dans `docs/journal/YYYY-MM-DD-f4-wolf-e2e.md` :
- Jeu 1 (Honoka) : segments extraits, clé utilisée, export OK/KO
- Jeu 2 (Densyanai Inko) : clé GuessKeyV6, archives traitées, export OK/KO
- Problèmes rencontrés et corrections

Commit message : `test(wolf): e2e validation — 月咲流ホノカ (direct key) + Densyanai Inko (GuessKeyV6)`

---

### Step 8 — Mise à jour ROADMAP + validation GuessKeyV6 sur Densyanai Inko

**Objectif :** Cocher tous les items F4 Wolf RPG dans `ROADMAP.md`. Valider que GuessKeyV6
fonctionne en ajoutant un test unitaire automatisé qui utilise le header réel de
`Densyanai Inko/Data/BasicData.wolf` comme fixture.

**Fichiers touchés :**
- `ROADMAP.md`
- `CHANGELOG.md`
- `src-tauri/src/engines/wolf/decryptor.rs` ← test fixture réel

**Dépend de :** Step 7 (e2e passé), tous les F4-01 à F4-04 complets

**Test GuessKeyV6 sur vrai header :**
- `BasicData.wolf` de Densyanai Inko = 10KB → extraire les 64 premiers octets comme fixture
- Ces octets peuvent être inclus dans le test en dur (pas de fichier de jeu dans le repo)
- Vérifier que `guess_key_v6(header)` retourne une clé non-nulle ET que `extract_all()` avec
  cette clé déchiffre correctement un fichier synthétique à partir de ce vrai header

Tâches :
- [ ] Extraire manuellement les 64 premiers octets de `BasicData.wolf` et les hardcoder dans
  un test comme vecteur de test :
  ```rust
  #[test]
  fn test_guess_key_v6_real_header() {
      let header: [u8; 64] = [/* octets Densyanai Inko BasicData.wolf 0x00..0x40 */];
      let key = guess_key_v6(&header);
      assert!(key.is_some(), "GuessKeyV6 should recover a key from a real Wolf v2 archive");
  }
  ```
- [ ] Cocher dans ROADMAP.md :
  - `[x] src-tauri/src/engines/wolf/extractor.rs`
  - `[x] src-tauri/src/engines/wolf/decryptor.rs`
  - `[x] src-tauri/src/engines/wolf/injector.rs`
  - `[x] Tests round-trip Wolf v1/v2`
- [ ] Mettre à jour `CHANGELOG.md` : section `[Unreleased]` → entrée complète F4 Wolf RPG

**Test de validation :**
```bash
cargo test --manifest-path src-tauri/Cargo.toml
pnpm typecheck
```
Résultat attendu : 100% tests verts (dont `test_guess_key_v6_real_header`), 0 erreur TS

Commit message : `chore(f4): mark Wolf RPG F4 complete — ROADMAP + CHANGELOG + GuessKeyV6 real-header test`

---

## Tests obligatoires avant push GitHub (gate F4 complet)

```bash
# Gate Rust
cargo test --manifest-path src-tauri/Cargo.toml
# Attendu : 100% verts (fondations F4-01 + decryptor F4-02 + extractor F4-03 + injector F4-04)

cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
# Attendu : 0 warning, 0 erreur

cargo fmt --manifest-path src-tauri/Cargo.toml
# Attendu : aucun fichier modifié

# Gate TypeScript
pnpm typecheck
# Attendu : 0 erreur

pnpm lint
# Attendu : 0 erreur ESLint

# Gate manuel (obligatoire pour merger F4)
# [ ] 月咲流ホノカ s'ouvre (clé v2.255 directe) + segments visibles + export OK
# [ ] Densyanai Inko s'ouvre (GuessKeyV6) + 13 archives traitées + export OK
# [ ] Les deux jeux démarrent après export (même sans traduction)
```

## Commandes git

```bash
git checkout -b feat/f4-05-wolf-integration
# ... commits intermédiaires par step ...
git checkout main
git merge --no-ff feat/f4-05-wolf-integration -m "feat(f4-05): Wolf RPG full integration — commands, QA, UI, e2e tests"
git push origin main
git branch -d feat/f4-05-wolf-integration
```

## Mise à jour après complétion

- `ROADMAP.md` : tous les items F4 Wolf cochés, statut F4 `[x] Complet`
- `CHANGELOG.md` : entrée `[0.4.0]` avec tout F4 Wolf RPG
- `docs/journal/` : entrée `YYYY-MM-DD-f4-wolf-integration.md` + entrée `YYYY-MM-DD-f4-wolf-e2e.md`
- `docs/architecture.md` : mis à jour (Step 6)
- Mémoire session : F4 complet, préparer release v0.4.0
