# F4-01 Wolf RPG Foundations — 2026-06-06

Session d'implémentation complète du plan `docs/plans/f4-01-wolf-foundations.md`.
7 steps exécutés, 7 commits intermédiaires, merge `--no-ff` vers `main`.

---

## Ce qui a été fait

### Step 1 — Dépendances Cargo
- `encoding_rs = "0.8"` (v0.8.35 disponible sur crates.io) — déjà transitif, maintenant déclaré explicitement
- `wolfrpg-map-parser = "0.6"` (v0.6.0, seule version disponible, MIT, G1org1owo)
- `cargo check` propre, Cargo.lock mis à jour

### Step 2 — Module wolf/ scaffoldé
- `src-tauri/src/engines/wolf/mod.rs` — déclare `decryptor`, `extractor`, `injector`
- 3 stubs vides (commentaires F4-02/03/04)
- `pub mod wolf;` ajouté dans `src-tauri/src/engines/mod.rs` (même endroit que `mv_mz`)

### Step 3 — Engine::Wolf dans detector.rs
- Variante `Wolf` ajoutée à l'enum `Engine` (entre `MvMz` et `VxAce`)
- `is_wolf_game_dir()` : Game.exe ou Game.ini + (BasicData/ ou Data/*.wolf ou Data/MapData/*.mps)
- `has_wolf_archives()` et `has_mps_files()` helpers privés
- 4 match arms non-exhaustifs dans `commands/project.rs` corrigés avec bras provisoires :
  - `engine_str` : `Engine::Wolf => "wolf"`
  - `data_dir` : `Engine::Wolf => return Err("Wolf RPG extraction not yet implemented (F4-03)")`
  - `game_title` : `Engine::Wolf => unreachable!()`
  - extraction match : `Engine::Wolf => unreachable!()`

### Step 4 — detect_engine() branché
- Wolf inséré en position 2 dans `detect_engine()` (après MV/MZ, avant VX Ace désactivé)
- 6 tests unitaires : `game_exe_basic_data`, `wolf_archives`, `game_ini`, `no_launcher`,
  `mv_not_confused_with_wolf`, `wolf_not_confused_with_mv`
- Résultat : 18/18 tests détecteur verts

### Step 5 — WolfVersion + find_wolf_data_dir
- `WolfVersion { major: u8, minor: u8 }` avec `is_utf8() -> bool` (`major >= 3`)
- `find_wolf_data_dir()` : cherche `Data/` puis `data/` (fallback Linux)
- `guess_wolf_version_from_structure()` : retourne `WolfVersion { major: 2, minor: 0 }` par défaut
  — commentaire `TODO(F4-02): read exact version from DXA header CodePage field`
- 4 tests unitaires : `is_utf8`, `is_shiftjis`, `capital_d`, `lowercase_fallback`
- Résultat : 22/22 tests verts

### Step 6 — RE_WOLF dans tokenizer.rs
- `Engine::Wolf` ajouté à l'enum `Engine` du tokenizer
- `static RE_WOLF: LazyLock<Regex>` avec 14 alternatives ordonnées :
  - `\r[Base,Ruby]` opaque (PREMIER — contient virgule)
  - `\udb/\cdb/\sdb[\d+:\d+:\d+]` DB refs
  - `\sysS[n]` avant `\sys[n]`
  - `\cself[n]` avant `\self[n]`
  - `\self[n]`, `\sys[n]`, `\space[n]`
  - `\v?[n]` (? littéral Wolf)
  - `\sp/mx/my/ax/ay[n]`, `\-[n]`, `\font[n]`
  - `\v/c/s/f/i[n]` (maj+min)
  - `\m[n]` (séparé — f exclu pour éviter duplication)
  - `<L>/<C>/<R>` alignement
  - `\A+/\A-` anti-aliasing
  - `\E\N\\\!\.\^\>\<` codes no-arg (correction journal 2026-06-06)
  - `\n` newline littéral
- Clippy : 0 warnings sur la regex (validation `clippy::invalid_regex`)
- 15 tests MV/MZ existants toujours verts

### Step 7 — 11 tests tokenizer Wolf
Tous 11 verts :
- `test_wolf_ruby_opaque` — `\r[魔法,まほう]` → 1 token
- `test_wolf_ruby_with_text` — texte après ruby préservé
- `test_wolf_db_refs` — `\udb/\cdb/\sdb` → 3 tokens, round-trip OK
- `test_wolf_sysS_before_sys` — `\sysS[10]` → 1 token (pas `\sys[10]`)
- `test_wolf_cself_before_self` — `\cself[99]` → 1 token (pas `\self[9]`)
- `test_wolf_reserve_variable` — `\v?[30]` → 1 token
- `test_wolf_alignment_tags` — `<L>テキスト<R>` → 2 tokens
- `test_wolf_standard_codes` — `\v[5]\c[3]\s[10]` → 3 tokens
- `test_wolf_no_arg_codes` — `\E\N\A+\A-` → 4 tokens
- `test_wolf_multiline` — `\n` → 1 token
- `test_wolf_no_interference_with_mvmz` — MV/MZ inchangé

---

## Chiffres finaux

| Métrique | Valeur |
|---------|-------|
| Tests Rust total | 192 (+ 2 ignored VxAce) |
| Nouveaux tests Wolf detector | 6 |
| Nouveaux tests WolfVersion | 4 |
| Nouveaux tests tokenizer Wolf | 11 |
| Tests MV/MZ préservés | 15 |
| Clippy warnings | 0 |
| Commits intermédiaires | 7 |

---

## Décisions prises

- `wolfrpg-map-parser = "0.6"` : seule version sur crates.io (0.6.0), adoption faible (38 dl/mois)
  — utilisé comme accélérateur pour F4-03, fallback manuel prévu si la crate est abandonnée
- `encoding_rs = "0.8"` : déjà transitif (via reqwest), déclaré explicitement pour clarté
- `wolf/mod.rs` déclaré dans `engines/mod.rs` (pas dans `lib.rs`) — même pattern que `mv_mz`
- 4 match arms dans `commands/project.rs` : `data_dir` retourne Err provisoire (le plus propre),
  `game_title` et extraction match utilisent `unreachable!()` (dead code — Wolf sort avant)
- Correction regex appliquée (journal corrections) : `\\m\[\d+\]` seul (pas `\\[fm]`),
  `\\[EN\\!.^><]` avec les 5 codes de contrôle d'affichage manquants

---

## Tâches ROADMAP cochées

- [x] F4-01 : Engine::Wolf détection + WolfVersion + RE_WOLF tokenizer + engines/wolf/ scaffold
- F4 statut : `[ ] À démarrer` → `[~] En cours`

---

## Prochaine session

F4-02 — `engines/wolf/decryptor.rs` :
- Lire l'en-tête DXA (signature "DX", version 5/6/8)
- Table de clés hardcodée (Wolf v2.01, v2.10, v2.20, v3.x)
- Algorithme KeyConv XOR 12 octets (depuis GARbro ArcDX.cs)
- GuessKeyV6 (attaque texte clair connu)
- Plan : `docs/plans/f4-02-wolf-decryptor.md`
