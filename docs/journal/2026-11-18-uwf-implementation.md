# Journal — 2026-11-18 — UWF : déchiffrer un jeu Wolf dans l'app avec unwolf (UWF-1 à UWF-6)

**Phase** : F5
**Durée estimée** : session cloud
**Statut** : 🔄 En cours
**Outil** : Claude (session cloud)
**Branche et commit de départ** : `feat/wolf-unwolf` depuis `main` @ `76f3d5a` (« docs: code map in docs/architecture.md (DD V8) »)
**Objectif / plan / story** : étapes UWF-1 à UWF-6 du « Plan — UWF » (en tête de `tasks/todo.md`), une PR vers
`main`, sans fusion. UWF-7 (retrait de `decrypt/`) et UWF-V (essai réel) hors session. `tasks/todo.md` et
`tasks/handoff.md` ne sont pas modifiés par cette session.

---

## Reprise

- Lus : `tasks/handoff.md`, `tasks/lessons.md`, `CONTEXT.md`, `docs/architecture.md`, le plan UWF en entier,
  ADR-008, `docs/engines/wolf-uberwolf.md`.
- Préalables vérifiés sur `main` @ `76f3d5a` : `src/lib/api/`, `src-tauri/src/core/text_fit/` et
  `src-tauri/src/db/project_context.rs` présents. ADR-009 existe : ADR-010 est libre.
- unwolf 0.2.0 publié sur crates.io (API crates.io : versions `0.2.0`, `0.1.2`, `0.1.1`, `0.1.0`). Sources de la
  0.2.0 téléchargées depuis crates.io dans le dossier temporaire de la session pour lire l'API (lecture seule,
  hors dépôt).
- Environnement cloud : `rustc 1.97.0` (≥ 1.85), Node 22, pnpm 10.28. Les bibliothèques système de Tauri
  manquaient (`gdk-sys` ne compilait pas) : installées comme dans `.github/workflows/release.yml`
  (`libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`). `lindera.dev` accessible : le
  dictionnaire japonais se télécharge, aucun test en échec attendu.

## Gate de référence sur `main` @ `76f3d5a` (avant tout changement)

| Contrôle | Résultat |
|---|---|
| `cargo test --locked --no-fail-fast` | 850 réussis, 0 échec, 15 ignorés |
| `pnpm typecheck` | OK |
| `pnpm lint` | 0 erreur, 10 avertissements |
| `pnpm test` (Vitest) | 44 fichiers, 201 tests réussis |

(`cargo fmt --check` et `cargo clippy --all-targets` passés pour la première fois avec UWF-1, ci-dessous.)

## UWF-1 — Dépendance

**Fait.**
- `src-tauri/Cargo.toml` : `unwolf = { version = "0.2", default-features = false }` (UWF-D8), depuis crates.io,
  sans chemin ni Git. `Cargo.lock` : une seule entrée ajoutée, `unwolf 0.2.0` (dépendances `encoding_rs`, `log`,
  `lz4_flex`, `memmap2`, `serde`, `serde_json`, `thiserror 2.0.20`, toutes déjà présentes) ; aucune autre version
  changée (`diff` du lockfile vérifié).
- Mentions de licence : la recherche `rg -i 'third.party|NOTICE|licen'` montre que les avis tiers vivent dans
  `src-tauri/resources/licenses/` (avis de `lindera-ipadic`), inclus dans les bundles par `bundle.resources` de
  `src-tauri/tauri.conf.json`. Même mécanisme pour unwolf : `src-tauri/resources/licenses/unwolf-NOTICE.txt`
  (texte `LICENSE` MIT et `NOTICE` de la crate 0.2.0, à l'identique) ajouté à `bundle.resources`. Pas de
  `THIRD_PARTY_NOTICES.md` à la racine, puisqu'un emplacement existait.

**Écart au plan.** Aucun (`rustc 1.97.0`, publication vérifiée par l'API crates.io plutôt que `cargo search`).

**Gate (UWF-1).** `cargo fmt --check` OK ; `cargo clippy --all-targets --locked -D warnings` OK ;
`cargo test --locked --no-fail-fast` 850 réussis, 0 échec, 15 ignorés ; `pnpm typecheck` OK ; `pnpm lint`
0 erreur (10 avertissements, inchangés) ; Vitest 201/201.

## UWF-2 — Module `engines/wolf/unpack.rs`

**Fait.**
- `src-tauri/src/engines/wolf/unpack.rs` (nouveau), seul fichier de l'app qui importe `unwolf::`
  (`rg 'unwolf::' src-tauri/src` : ce seul fichier). Interface : `unpack_game(game_dir, config_path,
  on_progress, cancel) -> Result<UnpackReport, UnpackError>`. `UnpackReport` (`serde`, `camelCase`) : archives
  extraites, déjà extraites, ignorées par unwolf, en échec (archives sans texte seulement), état Pro
  (`notPro`, `alreadyUnprotected`, `removedV35`), fichiers déprotégés, archives déplacées, avertissements.
  `UnpackError` (`thiserror`) : chaque message commence par un marqueur stable `wolf_unpack_<cas>`
  (`cancelled`, `no_launcher`, `not_writable`, `data_folder_present`, `destination_exists`, `pro_unsupported`,
  `extraction_failed`, `protection_failed`, `unprotect_failed`, `not_readable`, `move_failed`) pour le front.
- Ordre : archives de texte (`sources::text_archives`, inchangé) ; préalables sans extraction (écriture possible,
  destinations libres dans `Hoshi2Star-archives/`, `Data.wolf` + `Data/` non vide ⇒ refus) ; premier lanceur de
  `WOLF_LAUNCHERS` (passé en `pub(crate)`) donné à `init_game` ; extraction des archives contenant `BasicData`
  (`Data/BasicData.wolf`, ou `Data.wolf` entier par `unpack_data_with_progress`) ; état Pro par
  `session.protection_status()` ; extraction du reste ; levée V35 ; `Session` relâchée ; contrôle « lisible par
  l'app » (`is_wolf_game_dir`, une base qui déclare l'encodage, au moins un `.mps` dans `Data/MapData/`) ;
  déplacement un à un par `rename`, avec remise en place des archives déjà déplacées si l'un échoue.
- `engines/detector.rs` : la lecture de l'encodage de `guess_wolf_version_from_structure` est sortie dans
  `wolf_database_version(game_dir) -> Option<WolfVersion>` (`pub(crate)`), pour contrôler le jeu tant que les
  archives sont encore là ; `guess_wolf_version_from_structure` = refus des archives puis cette fonction :
  comportement identique (tests existants inchangés et verts).
- 10 tests unitaires (archives fabriquées par `unwolf::dxa::pack_directory` et `default_modes()[3]`) : jeu par
  dossiers lisible, archives déplacées, archive d'images ni extraite ni déplacée, `refuse_text_archives` vide,
  `detect_engine` = Wolf, aucun dossier temporaire laissé ; `Data.wolf` racine extrait en entier puis déplacé ;
  `Data.wolf` + `Data/` non vide ⇒ refus sans aucune écriture (arborescence identique) ; archive de texte
  corrompue ⇒ erreur, rien déplacé ; annulation au premier fichier ⇒ `Cancelled`, rien déplacé, pas de sortie
  partielle ; destination déjà prise ⇒ refus avant extraction ; fichier libre préexistant intact
  (`AlreadyUnpacked`) ; jeu sans carte après extraction ⇒ `NotReadable`, archives en place ; jeu sans lanceur ⇒
  refus ; deuxième passage ⇒ rapport vide, arborescence inchangée.

**Écarts au plan (la 0.2 fait foi).**
- Avertissements collectés par `Arc<Mutex<Vec<String>>>`, pas `Rc<RefCell<…>>` : le gestionnaire
  d'événements d'unwolf 0.2 doit être `Send` (`EventHandler`).
- État Pro lu par `session.protection_status()` (`NotPro`, `Unprotected`, `Protected(V35)` ⇒ levée,
  `Protected(_)` et toute valeur future ⇒ refus) ; aucun test `pro_generation` refait dans l'app (UWF-D5).
- `Event`, `Outcome`, `ProtectionStatus`, `ProVersion`, `FileStatus` sont `#[non_exhaustive]` : chaque `match`
  a un bras générique (refus ou échec, jamais « succès par défaut »).
- Précision de UWF-D4 : le refus `Data.wolf` + `Data/` ne vaut que si `Data/` contient quelque chose. Un `Data/`
  vide est extrait normalement ; c'est aussi la règle d'unwolf (`AlreadyUnpacked` = dossier de sortie non vide).
- Les chemins sont comparés après `std::path::absolute(game_dir)` : unwolf rend les archives en chemin absolu
  (son `init_game` fait de même).

**Limites (non testables par archives fabriquées, à voir en UWF-V).**
- Branches Pro (dérivation de clé, levée V35, refus V1/V2) : unwolf ne permet pas de fabriquer ces cas ;
  couvertes par UWF-0 (partie Claude) et à vérifier en UWF-V.
- Remise en place après un déplacement en échec : non testée (un `rename` ne peut pas être mis en échec de façon
  fiable sous Linux en root ; le cas visé est un fichier verrouillé sous Windows).
- Après un refus Pro V1/V2, le `Data/BasicData/` extrait pour lire `Game.dat` reste dans le dossier du jeu ; les
  archives ne bougent pas, le jeu continue donc de lire ses archives. Rien n'est supprimé par l'app.

**Gate (UWF-2).** Premier passage : clippy rouge (`drop` d'une fermeture, lint `drop_non_drop`), ligne retirée ;
tests déjà verts. Second passage : `cargo fmt --check` OK ; clippy OK ; `cargo test` 860 réussis (+10), 0 échec,
15 ignorés ; typecheck OK ; lint 0 erreur (10 avertissements) ; Vitest 201/201.

**Incident d'environnement.** Disque de session plein pendant la compilation (`src-tauri/target` à 29 Go) :
`target` supprimé, puis compilation sans cache incrémental ni informations de débogage
(`CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`). Sans effet sur le code ni sur
les résultats des tests.

## UWF-3 — Détection de l'état empaqueté

**Avant de coder (leçon du 2026-10-28).** `rg -l 'Game\.ini|Game\.exe|GamePro|MapData|BasicData'
src-tauri/tests src-tauri/src` : 23 fichiers. Ceux qui fabriquent un jeu avec une archive `.wolf` puis appellent
la détection ou l'ouverture (`detector.rs` `test_detect_wolf_packed_game_is_not_detected` et
`test_guess_wolf_version_reports_text_archives_first`, `sources.rs`, `injector.rs`, `extractor.rs`
`test_real_honoka_archive_is_refused`, `e2e_project_flow.rs` `wolf_game_with_a_text_archive_is_refused_at_open`)
passent par `detect_engine`, `guess_wolf_version_from_structure`, `WolfSources` ou l'injecteur, que UWF-3 ne
change pas : seul le message d'erreur d'`open_project` sur `UnknownEngine` change, et aucun test ne l'attendait
pour un jeu empaqueté (`rg 'could not identify game engine'` : seulement le front et ses tests, qui simulent
l'erreur). Aucun test existant modifié.

**Fait.**
- `engines/wolf/sources.rs` : `WOLF_PACKED = "wolf_packed"`, variante `SourcesError::Packed { archives }`
  (message `wolf_packed: <archives>: …`), `PackedState { Packed, Loose, NotWolf }`, `packed_state(game_dir)`
  (lanceur + au moins une entrée de `text_archives` + `is_wolf_game_dir` faux ⇒ `Packed` ; noms de fichiers
  seulement, aucune archive ouverte) et `packed_error(game_dir)`.
- `commands/project.rs` `open_project` : dans le `spawn_blocking` de détection, `UnknownEngine` devient
  l'erreur `wolf_packed` quand `packed_state` vaut `Packed` ; sinon le message reste celui d'`UnknownEngine`.
  `detect_engine` inchangé (`test_detect_wolf_packed_game_is_not_detected` toujours vert). Le cas « fichiers
  libres + archives » reste `wolf_archive_present`.
- Tests : `sources::tests::test_packed_state` (par dossiers, `Data.wolf` racine avec `GamePro.exe`, sans lanceur,
  archive d'images seule, jeu déchiffré, jeu déchiffré + archive) ; e2e
  `wolf_packed_game_is_refused_at_open_with_the_packed_marker` (marqueur `wolf_packed`, archives citées, pas de
  « could not identify game engine », dossier sans jeu ⇒ message inchangé, aucun projet créé).

**Écart au plan.** `packed_error` ajouté à côté de `packed_state` pour garder le message dans `sources.rs`.

**Gate (UWF-3).** fmt OK ; clippy OK ; `cargo test` 862 réussis (+2), 0 échec, 15 ignorés ; typecheck OK ; lint
0 erreur (10 avertissements) ; Vitest 201/201.
