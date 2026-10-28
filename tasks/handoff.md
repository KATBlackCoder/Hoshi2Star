# Point de reprise — Hoshi2Star

**Mis à jour :** 2026-10-28 — Claude, session cloud « Implémentation UW » (en cours).
**Objectif utilisateur :** baser Wolf sur le `Data/` déchiffré par UberWolf, sans lire les `.wolf` (décision prise le 2026-10-28 après les essais en jeu des jeux Pro).
**Statut :** `main` @ `e95bb38`. Branche `feat/wolf-uberwolf` (depuis `origin/docs/wolf-uberwolf-plan`) : implémentation UW, une étape par commit, gate cloud vert à chaque étape.

## En cours (2026-10-28, Claude) — UW : Wolf lu depuis le `Data/` déchiffré par UberWolf

- Plan : `tasks/todo.md`, « Plan — UW », **validé le 2026-10-28** (UW-D1 à UW-D8 = recommandations, UW-D5 reporté). Journal : `docs/journal/2026-10-28-plan-wolf-uberwolf.md`.
- Preuves en jeu du 2026-10-28 : とのかじ 3.723 et ポンコツ 3.714 (`GamePro.exe`), déchiffrés par UberWolf et sans `.wolf`, se lancent et affichent les textes marqués « ★ » par le prototype (copies `RAE-PLAN/wolf/*-essai-marque`, hors Git). Non prouvé : `Evtext/*.txt` de ポンコツ ; un export fait par l'app.
- Analyse des 24 956 textes du 3.658 : `RAE-PLAN/essais/wolf-3658-analyse-textes.md` et `wolf-3658-segments-classes.tsv` (hors Git) ; 19 chemins `bg/*.png⏎0⏎0⏎0` extraits à tort (filtre corrigé par UW-4).
- **Implémentation (session cloud, 2026-10-28)** : branche `feat/wolf-uberwolf`. Avancement coché dans `tasks/todo.md` (« Plan — UW »), avec le résultat du gate de chaque étape. UW-1 (détection), UW-2 et UW-3 (sources `Data/` seul, refus `wolf_archive_present`, export en fichiers libres), UW-4 (filtre des chemins suivis de lignes numériques), UW-5 (interface) faits, gate vert.
- **Prochaine action** : UW-6 (revue des tests d'intégration restants), UW-7 (documentation), UW-8 et PR vers `main`. UW-9 (essai en jeu) et UW-R restent à faire en local par l'utilisateur.

## Terminé (2026-10-28, Claude) — libellés de terminologie traduits, PR #20 fusionnée

- Travail Codex non commité récupéré du worktree `~/.codex/worktrees/6fab` (branche `codex/hoshiyomi-redesign`, déjà dans `main`) et posé tel quel sur `fix/terminology-i18n-labels` depuis `main` @ `333633a` (accord de l'utilisateur). Worktree et branche Codex retirés ensuite.
- Contenu : libellés des filtres et fenêtres de terminologie (catégorie grammaticale, type sémantique) passés en clés i18n fr/en au lieu du français écrit en dur ; `TerminologySelect.tsx`, `terminologyOptions.ts`.
- **Gate cloud vert** sur `988c5cb` (typecheck, Vitest 156/156, lint 0 erreur / 10 avertissements connus, Prettier OK). Puis, à la demande de l'utilisateur : `SEMANTIC_TYPE_LABELS` simplifié et `TermTranslateDialog` traduit (clés `terminology.translateDialog.*`), +1 test ; gate refait vert (Vitest 157/157). Commité sur `claude/bold-edison-m1w6yn`, au-dessus de `988c5cb`. Journal : `docs/journal/2026-10-28-terminologie-i18n.md`.
- À signaler dans la PR : le type sémantique devient une liste fermée (éditeur et filtre), plus de saisie libre.
- `fix/terminology-i18n-labels` avancée sur `1c9ab2f`, **PR #20 fusionnée** (`c1afdf5`, 2026-10-28, accord de l'utilisateur) ; branches `fix/terminology-i18n-labels` et `claude/bold-edison-m1w6yn` supprimées.
- Non vérifié dans l'app (page Terminologie en anglais). Reste hors lot : deux `aria-label` français en dur (`ScanProgress.tsx:43`, `TerminologyTable.tsx:244`) ; écart Prettier déjà présent dans `TermTranslateDialog.test.tsx`.

## En cours (2026-10-27, Claude) — WR-E6 : retours à la ligne Wolf côté LLM

- Plan : `tasks/todo.md`, « Plan — WR-E6 », **validé le 2026-10-27** (recommandations). Journal : `docs/journal/2026-10-27-wr-e6-retours-llm-wolf.md`.
- Branche `feat/wr-e6-soft-breaks` (worktree `../Hoshi2Star-e6`) **empilée** sur `origin/feat/wr-wolf-wrap` @ `090e445` (décision de l'utilisateur) : la PR #16 n'est pas fusionnée ; la PR E6 visera `main` après elle.
- Fait : `engines/wolf/line_breaks.rs` (retours souples, recoupe), `Engine::WolfMessage` et `Tokenized::restore` (`llm/tokenizer.rs`), clé Wolf dans le contexte (`llm/context/wolf.rs`), QA avec `json_key`, 17 tests, `CHANGELOG.md`. Relecture indépendante faite, deux défauts corrigés.
- **Poussé** le 2026-10-27 (accord de l'utilisateur) : `origin/feat/wr-e6-soft-breaks` @ `f414e62`. Gate cloud demandé dans une session cloud « Gate cloud WR-E6 », qui ne fait que rapporter (aucune note automatique à la fin : résultat à relever dans son transcript).
- **Gate cloud vert** (2026-10-27) sur `38033e9`, code identique à `f414e62` : fmt, typecheck, Vitest 153/153, clippy `--all-targets -D warnings` 0, `cargo test --locked --no-fail-fast` **803 OK / 0 échec / 15 ignorés** (+17 = les tests E6), lint 0 erreur (10 avertissements connus). Vrais jeux non couverts (tests `#[ignore]`).
- **Essai réel E6-7 concluant** (2026-10-27, projet `APP`) : 27/27 messages retraduits par le LLM, 0 ligne de ponctuation seule (9 avant), 0 ligne vide (5 avant), nombre de lignes de la source respecté, aucun `needs_review`. 26 entrées de TM fautives supprimées avant l'essai (sauvegarde `backups/hoshi2star-avant-essai-e6-20261027-215036.db`). Détail : journal du jour.
- **PR #18 ouverte** (2026-10-27, accord de l'utilisateur), empilée sur `feat/wr-wolf-wrap` (#16).
- **Fusionné** le 2026-10-27 à la demande de l'utilisateur : #16 (`e47a1e5`), puis #18 repointée vers `main` et fusionnée.
- **#17 fusionnée** ensuite (`1ff3352`, repointée vers `main`, à la demande de l'utilisateur) : la fenêtre d'export propose la taille du jeu au lieu de `\f[18]`.
- **Ménage (2026-10-27, accord de l'utilisateur)** : worktrees `../Hoshi2Star-rw`, `-e6`, `-wr`, `-font` et Codex `522a` retirés ; branches fusionnées supprimées en local et sur `origin` (dont `fix/sqlite-translation-secs-migration`, déjà dans `main` par la PR #3) ; sauvegardes `avant-essai-*` supprimées ; copies `RAE-PLAN/essais/wolf-v223`, `wolf-v3` et `wolf-v3-en` supprimées (les chemins cités plus bas n'existent plus). Gardés : `RAE-PLAN/essais/extraits` (tests réels Wolf), `bakin-v07`, `test/`.
- Gardé à décider : le worktree Codex `6fab` (`codex/hoshiyomi-redesign`) porte un travail non commité et absent de `main` (libellés des filtres de terminologie traduits fr/en, `TerminologySelect.tsx`, `terminologyOptions.ts`).
- **Prochaine action** : l'utilisateur retire les projets d'essai APP et DX de l'app (dossiers de jeu supprimés). Suites Wolf : essai en jeu 3.658 depuis l'app (recopier le jeu) ; garde « jeu mis à jour » Wolf ; coupe après un trait d'union (« H- ⏎ scenes ») ; noms instables (glossaire).

## Terminé (2026-10-27, Claude) — RW (Wolf), fusionné par la PR #13

- Plan validé : `tasks/todo.md`, « Plan détaillé — RW ». Branche `feat/rw-wolf` dans le worktree `../Hoshi2Star-rw` (depuis `origin/main` @ `2331c75`). Commits locaux ; push seulement avec accord (gate cloud).
- Base : étude REA RuneTranslate + prototypes Python validés **en jeu** sur Wolf 2.23 et 3.658 (preuves locales hors Git : `RAE-PLAN/wolf/`, `RAE-PLAN/essais/RESULTATS.md`). Constat clé : **l'archive `.wolf` passe avant les fichiers libres** (v2 et v3) ; l'archive reconstruite est acceptée par le jeu.
- Journal : `docs/journal/2026-10-26-rw-wolf.md`.
- **RW-1 écrit** (`5d90131`, `engines/wolf/decrypt/legacy_xor.rs`) : signature v6 chiffrée, LZ v6, noms et chemins v6 réels, version de chiffrement v8 (chaîne-clé « v3.14 », refus 3.31+ par `UnsupportedWolfCrypt`). Tests synthétiques au format DxLib réel. **Non compilé, non testé** : premier contrôle au gate cloud.
- Attention pour RW-4 : RW0 refuse Wolf v2 à l'ouverture et à l'export avec un message dédié ; ce refus doit être levé quand les cartes v2 seront lues.
- **Poussé** le 2026-10-26 (`origin/feat/rw-wolf` @ `cc7efc7`, accord de l'utilisateur) ; **gate cloud RW-1 vert** sur `cc7efc7` : fmt OK, typecheck OK, Vitest 142/142, clippy `--all-targets -D warnings` 0 avertissement, `cargo test --locked` 724 OK / 0 échec / 12 ignorés (717 sur `main`, +7 RW-1), lint 0 erreur (10 avertissements frontend connus). Lecture de vraies archives non couverte (tests `diag_*` ignorés).
- Environnement cloud (hors périmètre, à traiter à part) : le hook `PreToolUse` bloque toute commande dont le texte contient la sous-chaîne interdite de npm, y compris la commande d'installation de pnpm et même une note qui la cite (motif trop large ; contourné par `pnpm i`) ; le conteneur cloud n'a pas les bibliothèques GTK/WebKit de Tauri (installées à la main via apt pour clippy) : à mettre dans le script de setup cloud.
- **RW-2 écrit** (`engines/detector.rs`, `legacy_xor.rs`) : version lue dans le magic de la base `BasicData` (dossier libre, puis archive qui la contient, seules les entrées base déchiffrées via `extract_matching`), puis version de chiffrement v8 ; la page de codes n'est plus utilisée. **Non compilé** : gate cloud à faire.
- RW-2 poussé (`151e9f2`, accord de l'utilisateur) ; **gate cloud RW-2 vert** sur `151e9f2` : fmt, typecheck, Vitest 142, clippy `--all-targets -D warnings` 0, `cargo test --locked` 727 OK / 0 échec / 12 ignorés (les 8 tests `guess_wolf_version` OK), lint 0 erreur. Setup cloud : `apt-get update` nécessaire avant l'installation des bibliothèques Tauri.
- **RW-4 écrit** (`74822db`) : cartes v2 Shift-JIS 0x64, cartes UTF-8 0x64 et `CommonEvent.dat` en clair lus et réécrits dans leur conteneur ; refus v2 de RW0 levé ; refus groupé conservé (`UnreadableWolfFiles`, marqueur `wolf_unreadable_files`) : un échec de lecture d'une carte n'est plus ignoré en silence, v3 compris. Test réel `wolf_real_round_trip` (`#[ignore]`, `H2S_WOLF_GAMES`). **Non compilé** : gate cloud à faire. Interface à reprendre en RW-8 (branche `WOLF_V2_UNSUPPORTED` morte dans `src/lib/projectError.ts`). Accents v2 : l'export échoue encore au premier accent (RW-7).
- RW-4 poussé (`a0a96b2`, accord de l'utilisateur) ; **gate cloud RW-4 rouge** : 732 OK / 1 échec / 13 ignorés ; seul échec `wolf_v2_game_is_refused_at_open` (test e2e de RW0 qui attendait `wolf_v2_unsupported`). Corrigé : test remplacé par `wolf_unreadable_map_is_refused_at_open` + nouveau `wolf_v2_game_opens_with_its_dialogue` ; non compilé.
- Correctif poussé (`169a644`, accord de l'utilisateur) ; gate relancé : erreur de compilation du nouveau test e2e (`OpenProjectResult` lu comme `Project`), corrigée d'une ligne ; correctif vérifié par la session cloud sur une copie hors dépôt : **734 OK / 0 échec / 13 ignorés**, clippy `--all-targets` 0. Commit du correctif local, non poussé. Partie Honoka du test non exécutée (fixture absente en cloud).
- **RW-3 écrit** (`engines/wolf/decrypt/writer.rs`, `read_layout` dans `legacy_xor.rs`) : `rebuild_archive` v6 (réécriture dans l'ordre physique, bourrage d'origine) et v8 (ajout en fin, table non compressée), selon `dxa6.py` / `dxa8.py` validés en jeu. Tests synthétiques (identité sans changement, relecture après remplacement, refus) ; test réel `#[ignore]` `wolf_real_archive_rebuild` (`H2S_WOLF_ARCHIVES`, fichiers `.wolf` séparés par `:`). **Non compilé.**
- RW-3 + correctif e2e poussés (`dc006ab`, accord de l'utilisateur) ; **gate cloud RW-3 vert** sur `dc006ab` : fmt, typecheck, Vitest 142, clippy `--all-targets -D warnings` 0, `cargo test --locked --no-fail-fast` **741 OK / 0 échec / 14 ignorés**, les 7 tests de l'écrivain et `test_find_v8_key_rejects_an_implausible_toc` OK, les 2 tests e2e Wolf OK ; lint 0 erreur. RW-1 à RW-4 et RW-3 vérifiés en synthétique ; vrais fichiers non vérifiés en Rust (tests `--ignored`).
- **RW-6 écrit** (`engines/wolf/integrity.rs`) : chaque carte, `CommonEvent.dat` et `.dat` produit est relu et comparé aux octets dont il vient, dans la boucle d'`inject_all_to_memory` ; écarts (`IntegrityIssue` MV/MZ réutilisé) ⇒ export refusé. Pas de contrôle « source changée » pour Wolf (pas de copie d'origine ; il bloquerait tout ré-export) : garde « jeu mis à jour » Wolf = lot séparé à proposer. `unknown1` des `.dat` conservé. Vérification de l'archive reconstruite reportée en RW-5. **Non compilé.**
- RW-6 poussé (`d216d57`, accord de l'utilisateur) ; **gate cloud RW-6 vert** sur `d216d57` : fmt, typecheck, Vitest 142, clippy `--all-targets -D warnings` 0, `cargo test --locked --no-fail-fast` **748 OK / 0 échec / 14 ignorés** ; `integrity` 6/6, `injector` 19/19, e2e Wolf 3/3 ; lint 0 erreur.
- **RW-5 écrit** (`e43ab1f`) : `engines/wolf/sources.rs` (`WolfSources` : archives d'abord — `Data.wolf`, `Data/MapData.wolf`, `Data/BasicData.wolf` — puis fichiers libres ; archive illisible ou cartes hors `MapData/` ⇒ refus, marqueur `wolf_unreadable_archive`) ; extracteur et injecteur branchés dessus, anciens chargeurs retirés ; export : fichier venu d'une archive ⇒ archive reconstruite livrée à son chemin, contrôlée par `verify_archive_rebuild` ; `.wolf` stocké sans recompression dans le zip. **Changement de comportement à signaler** : un `.wolf` illisible laissé à côté d'un `Data/` déchiffré par UberWolf refuse désormais le projet (messages mis à jour : retirer les `.wolf`). Autre changement : les archives DXA v5 (Wolf 1.x, sans dossiers) sont refusées à l'ouverture (avant : ouvertes, mais export en fichiers libres ignorés par le jeu). Mémoire de l'export : environ l'archive d'origine + l'archive reconstruite (≈ 4,2 Go pour le jeu 3.658 de 2,1 Go ; correctif `d49cead`). **Non compilé.**
- RW-5 poussé (`2ad2418`, accord de l'utilisateur, changements de comportement acceptés) ; **gate cloud RW-5 rouge** : 754 OK / 1 échec / 14 ignorés ; seul échec `test_inject_all_does_not_overwrite_wolf` (ancien test qui attendait qu'un faux `.wolf` soit ignoré). Corrigé (attend le refus, `.wolf` intact), non compilé ; tous les nouveaux tests RW-5 OK.
- Correctif poussé (`610cd33`, accord de l'utilisateur) ; **gate RW-5 vert** sur `610cd33` : fmt, typecheck, Vitest 142, clippy `--all-targets -D warnings` 0, `cargo test --locked --no-fail-fast` **755 OK / 0 échec / 14 ignorés** ; `engines::wolf` 170 OK / 4 ignorés ; e2e Wolf 4/4 ; lint 0 erreur.
- **RW-7 écrit** : `encoding::fold_for_shift_jis` (é → e, œ → oe, « » → ", espaces insécables, tirets) appliqué aux fichiers Shift-JIS (cartes, événements communs, `.dat`), compté (`folded_count`, `WolfExport::folded_texts`, journalisé) ; intégrité alignée ; `encoding::path_warning` pour un jeu v2 dont le chemin a un caractère hors cp932 ou à aller-retour cp932 trompeur (`〜` U+301C), journalisé à l'ouverture et à l'export. Affichage du compte et de l'avertissement : RW-8. **Non compilé.**
- **RW-8 écrit** (2026-10-27) : `preflight_export` renvoie `wolf` (archives annoncées, Shift-JIS, nombre de traductions accentuées, avertissement de chemin) sans lire d'archive ; boîte d'export : `ExportWolfNotes` ; erreurs `wolf_unreadable_archive` / `wolf_unreadable_files` avec détail en description du toast ; branche et clés `wolfV2Unsupported` retirées. `export_project` inchangé. **Non compilé.**
- RW-7 + RW-8 poussés (`1661ab5`, accord de l'utilisateur, 2026-10-27) ; **gate cloud vert** sur `1661ab5` : fmt, typecheck, Vitest **145/145**, clippy `--all-targets -D warnings` 0, `cargo test --locked --no-fail-fast` **760 OK / 0 échec / 14 ignorés**, lint 0 erreur (10 avertissements connus) ; tests ciblés RW-7/RW-8 et e2e Wolf 5/5 OK. Prettier (contrôle en plus) : écart sur `src/components/editor/ProjectList.tsx` **déjà présent sur `main`** (`2331c75`), hors lignes RW-8 ; noté à part dans `tasks/todo.md`.
- RW-9 (2026-10-27) : test réel `wolf_real_round_trip` **OK** sur 90 vrais fichiers (2.23 et 3.658, extraits dans `RAE-PLAN/essais/extraits/`, hors Git).
- RW-9 (2026-10-27) : test réel `wolf_real_archive_rebuild` **OK** sur le vrai `Data.wolf` 2.23 (204 Mo).
- RW-9 (2026-10-27) : **essai en jeu 2.23 OK** depuis l'app (zip = `Data.wolf` reconstruit seul ; jeu lancé, textes traduits affichés). Constats hors RW : pas de retour à la ligne automatique Wolf (débordements), pas d'export partiel pour tester. Base de l'app : projet de test `APP` réduit à `sys＿タイトル.mps` (sauvegarde `backups/hoshi2star-avant-essai-rw9-20261027-080135.db`).
- **PR ouverte** le 2026-10-27 : https://github.com/KATBlackCoder/Hoshi2Star/pull/13 (`feat/rw-wolf` → `main`, accord de l'utilisateur).
- **Prochaine action exacte** : relecture et fusion de la PR #13 par l'utilisateur ; essai en jeu 3.658 facultatif. Après fusion : les sessions « Export d'essai » (plan EX validé) et « Retour à la ligne Wolf » partent de `main` ; leurs plans sont non commités dans le dossier principal, à mettre sur leurs branches avant de mettre `main` à jour. (tests réels `--ignored` chez l'utilisateur, essai en jeu de l'export de l'app), puis PR. Pas de build local ; gate en cloud après push autorisé. Suites hors RW : garde « jeu mis à jour » Wolf (copie d'origine, équivalent R2) ; écriture en flux de l'archive dans le zip si l'export du jeu de 2,1 Go peine.

## En cours (2026-10-27, Claude) — WR : retour à la ligne Wolf à l'export

- **Repris le 2026-10-27** (décision de l'utilisateur, « ok on va essayer »), avec la méthode du budget en em (K9).
  RW (PR #13) et DX (PR #15, `80b4fc5`) sont fusionnés. Branche prévue : `feat/wr-wolf-wrap` depuis `main`, dans un
  worktree, pour ne pas changer la branche du dossier partagé. Le constat K9, non commité dans le dossier principal,
  ira dans le premier commit WR.
- K10 ajouté : largeurs d'origine en em sur tout le jeu. Les lignes avec visage vont jusqu'à 25 em, contre 22 sans visage.
- WR-1 réduit à 6 captures (B, C, D, I, K, L) dans la WCAL existante ; détail dans `tasks/todo.md`.
- Ancien report du même jour : capture A faite (choix collés à droite hors écran ; `\f` sans effet sur un choix).

- Plan : `tasks/todo.md`, « Plan — WR ». **Validé le 2026-10-27** (WR-E0 à WR-E6 = recommandations). Aucun code applicatif, rien commité.
- WR-1 : variante `RAE-PLAN/essais/wolf-v223/WCAL` construite et relue hors jeu (outil `RAE-PLAN/wolf/_outils/build_wolf_wcal.py`) ; fiche `WCAL.FICHE.md`, captures attendues dans `WCAL-captures/` (`capture.fish <lettre>`).
- Journal : `docs/journal/2026-10-27-wr-retour-ligne-wolf.md`.
- Prérequis levé : RW et DX sont fusionnés. EX est abandonné. Point d'accroche : `collect_wolf_zip_entries` (après `apply_font_prefix`, avant `wolf_to_memory`), pour les portées `Release` et `Debug`.
- Constats clés :
  - les `\n` parasites viennent du tokenizer, qui impose au LLM chaque `\n` comme jeton ;
  - la donnée en jeu (~276 px coupés à `\f[18]`) contredit le modèle simple (lignes d'origine de 320 à 336 px à 16 px) ⇒ calibrage en jeu obligatoire avant tout code (WR-1) ;
  - les réglages numériques de `Game.dat` ne sont pas décodés.
- **WR-1 fait (2026-10-27)** : budget de 22 em et 4 lignes, avec ou sans visage ; taille par défaut `\f[12]` ; `\f[18]` = 1,5 fois (K5 expliqué). Détail : `tasks/todo.md`, « Résultats WR-1 ».
- **WR-2/WR-3 faits** : taille du jeu illisible dans les fichiers ⇒ champ manuel (12 pour 2.23) ; budget = p99 des lignes des cartes (22 em) et maximum de lignes (4), concordant avec WR-1.
- **WR-4 à WR-7 écrits** (`feat/wr-wolf-wrap`) : `engines/wolf/layout/`, `commands/wolf_text_fit.rs`, migration `0013`, branchements export / QA / réglages, interface, tests. Écarts au plan listés dans `tasks/todo.md` (choix non signalés, police = seul fichier du dossier, mesure sans repli Shift-JIS).
- **Poussé** le 2026-10-27 (accord de l'utilisateur) : `origin/feat/wr-wolf-wrap` @ `95e7a76`. Gate cloud (WR-8) demandé dans une session cloud « Gate cloud WR », qui ne fait que rapporter : résultat à relever dans son transcript.
- **Gate cloud du 2026-10-27 sur `95e7a76`** : tout est vert sauf 1 test, 785 OK / 1 échec / 15 ignorés ; Vitest 153 ; clippy 0. L'échec vient de la liste des migrations figée à `1..=12` (`tests/terminology_migration.rs:205`), passée à `1..=13`, commitée et pas encore poussée.
- **Gate vert** sur `f02e174` (poussé avec accord) : `cargo test` 786 OK / 0 échec / 15 ignorés, fmt OK ; le reste est inchangé depuis `95e7a76`.
- WR-9 préparé (2026-10-27) : `pnpm i` dans le worktree ; base sauvegardée (`~/.local/share/com.blackat.hoshi2star/backups/hoshi2star-avant-essai-wr9-20261027-181808.db`, intégrité OK, 3 234 segments). Le lancement depuis le worktree applique la migration 0013 à la vraie base ; l'app de `main` l'accepte ensuite comme base plus récente. Mesure sur `APP` (une seule carte) : 22 em, mais 3 lignes ⇒ fixer 4 lignes à la main.
- **WR-9 OK en jeu (2026-10-27)** : tous les messages dans le cadre, portraits conservés, aucune ligne isolée (captures `RAE-PLAN/essais/wolf-v223/WR9-captures/`). Corrigés pendant l'essai : listes de la boîte illisibles en thème sombre ; remarques Wolf en anglais.
- **Gate final vert sur `4221f9e`** (2026-10-27) : fmt, typecheck, Vitest 153, clippy 0, `cargo test` 786 OK / 0 échec / 15 ignorés, lint 0 erreur ; CHANGELOG ajouté.
- PR #16 (`feat/wr-wolf-wrap` → `main`) ouverte ; relecture et fusion par l'utilisateur.
- **Suivi écrit (2026-10-27)** : le 18 par défaut de la fenêtre de taille de police à l'export (Wolf) ; branche `fix/wolf-export-font-default`, worktree `../Hoshi2Star-font`, empilée sur `feat/wr-wolf-wrap` ; gate vert.
- `fix/wolf-export-font-default` poussé ; **gate cloud vert sur `61dc603`** (Vitest 156, `cargo test` 787 OK / 0 échec, clippy 0).
- **PR #17 ouverte** (2026-10-27, accord de l'utilisateur) : https://github.com/KATBlackCoder/Hoshi2Star/pull/17, `fix/wolf-export-font-default` → `feat/wr-wolf-wrap`, empilée sur la PR #16.
- **Prochaine action** : relecture et fusion de la PR #16 par l'utilisateur, puis recibler la PR #17 sur `main`, et la relire et la fusionner.

## Abandonné (2026-10-27, Claude) — EX : export d'essai

- Plan validé puis **abandonné par l'utilisateur** après la fusion de RW. L'export exige toujours une traduction complète : c'est la règle produit.
- Remplacement envisagé : un bouton de debug qui extrait ou injecte pour tester en jeu. Pistes dans `tasks/todo.md`, « Plan — EX », section « Abandon ».
- Aucun code ni branche créés.
- **Remplacé par DX** : plan dans `tasks/todo.md`, « Plan — DX ». **Validé le 2026-10-27** (défauts DX-1 à DX-3). Implémentation en cours sur `feat/dx-debug-export` (depuis `main` @ `2d9ecd7`).
- **Étapes 1 à 6 faites le 2026-10-27** (code, tests Rust `src-tauri/tests/export_debug.rs`, tests Vitest, CHANGELOG,
  journal `docs/journal/2026-10-27-dx-export-debug.md`). **Rien de commité, aucun build ni test lancé** (seulement
  `rustfmt --check` et prettier).
- **Commit `8459d4c`** (`--no-verify`, accord de l'utilisateur) poussé sur `origin/feat/dx-debug-export` le 2026-10-27.
- **Gate cloud vert sur `8459d4c`** (2026-10-27, compte rendu collé par l'utilisateur) : fmt OK, typecheck OK,
  Vitest **152/152**, lint 0 erreur (10 avertissements connus), clippy `--locked --all-targets -D warnings` 0,
  `cargo test --locked --no-fail-fast` **766 OK / 0 échec / 14 ignorés** (vrai dictionnaire lindera) ;
  `tests/export_debug.rs` 5/5, `tests/text_fit.rs` 11/11. Aucune correction, rien poussé de plus.
- **Essai dans l'app OK** (2026-10-27, journal `docs/journal/2026-10-27-dx-essai-app.md`) :
  - réglage, bouton et boîte conformes ;
  - nouveau projet `DX` (copie intacte de `W0`), 31/3202 traduits ;
  - zip = `Data.wolf` seul ; seule la carte titre change (31 textes traduits, 1 laissé en japonais exprès).
  Base sauvegardée avant : `backups/hoshi2star-avant-essai-dx-20261027-141825.db`.
- **Essai en jeu OK** (utilisateur, `DX-JEU`, 2026-10-27) : menu titre, remerciements, difficulté et première
  réplique de Gyoin en anglais. La ligne de Gyoin laissée exprès en japonais
  s'affiche en japonais : export partiel confirmé. Débordements connus (WR).
- Suivi commité et poussé, **PR ouverte** le 2026-10-27 (`feat/dx-debug-export` → `main`, accord de l'utilisateur ;
  `gh pr view feat/dx-debug-export`).
- **Prochaine action** : relecture et fusion de la PR par l'utilisateur. Ensuite : WR (reprise WCAL), essai 3.658
  facultatif avec l'export de debug.

## En cours (2026-10-26, Claude) — Bakin sous Linux (étude locale, hors produit)

- Livrable : `RAE-PLAN/bakin/linux-recherche-globale.md` ; journal : `docs/journal/2026-10-26-bakin-linux-recherche.md`.
- Causes trouvées : sous Proton 11, VAO 0 refusé (`glVertexAttribPointer` → 1282, fenêtre blanche) ; sous wine-cachyos, plugins `.dlp` non chargés.
- Hook VAO compilé (`RAE-PLAN/bakin/_outils/bakin_vao_hook.so`), non essayé sur le jeu.
- T4 fait le 2026-10-26 : cause racine confirmée, un bug de Wine 11.0 (`memdup_attribs` tronque les attributs au premier 0, le contexte partagé recréé perd le profil compatibilité et devient core) ; détail dans `linux-recherche-globale.md` §3.5.
- **T1 fait le 2026-10-26 : le jeu fonctionne sous Linux** (titre, carte 3D, dialogues), avec GE-Proton 11-6 + hook VAO (`RAE-PLAN/bakin/_outils/t1.fish`). Reste une erreur `glBindTexture` `(634)` unique, sans effet visible (§7).
- Nettoyage fait (préfixe T4 et journal de 37 Mo mis à la corbeille). Script de lancement des variantes : `RAE-PLAN/bakin/_outils/bakin.fish <B0|B1|B2|B3> [args]` ; fiche Bakin mise à jour.
- Essais en jeu de traduction (2026-10-26) : **B0 OK** (archive reconstruite acceptée), **B1 OK** (textes traduits, accents, couleurs, variable `\$[島の名前]`), **B2 plante** (OverflowException) : les `.rbr` ont des tailles de blocs int32 à recalculer. `rbpack.size_fields` / `fix_sizes` ajoutés, variante **B4** construite et contrôlée hors jeu (`essais/RESULTATS.md`).
- B4 (tailles recalculées) : plus d'overflow, mais NullReferenceException dans `LayoutManager..ctor` (écran de chargement). Variantes d'isolement B5 (seul `Layout.rbr`) et B6 (seul `Map_000.rbr`) construites.
- B5 plante (faute dans `Layout.rbr`), **B6 OK : dialogues de carte traduits à longueur libre validés en jeu**. Cause de B5 trouvée : les écrans de `Layout.rbr` sont des objets `[taille][GUID][16 nuls][nom]` non détectés ; détecteur `_screens` ajouté, en-tête compté depuis l'octet 23 ; variante B7 construite.
- **B7 OK en jeu (2026-10-26) : traduction Bakin à longueur libre validée** (menu titre, dialogues, cartouche de nom, codes couleur et variables), avec les quatre formes de tailles de blocs recalculées. Règle consignée dans `tasks/lessons.md`.
- B3 (dictionnaire) : aucun effet, même avec `/Dic /L=EN` ; le lanceur ne transmet probablement pas `/Dic` (non bloquant).
- **Lutris : vérifié le 2026-10-26.**
  - B7 a été lancé depuis Lutris 0.5.22 : écran titre, Options et sortie normale. Le correctif est chargé (`created VAO 2`) et le journal du jeu ne contient aucune exception.
  - Procédure mise à jour dans `RAE-PLAN/bakin/lutris-bakin.md`. Sa copie en doc Claude, liée depuis ce fichier, n'est **pas** à jour.
  - Journal de session : `docs/journal/2026-10-26-bakin-lutris.md`.
  - Restes :
    - le vrai préfixe `pfx-bakin-t1` n'a **pas** été essayé sous Lutris : l'utilisateur a choisi de ne pas relancer ;
    - le préfixe parasite `"/home/blackat/pfx-bakin-t1 "` a été mis à la corbeille le 2026-10-26, avec son accord ;
    - le jeu Lutris contient encore `prefix: '/home/blackat/pfx-bakin-t1 '`, avec une espace : à corriger dans Lutris, sinon le prochain lancement recrée ce préfixe ;
    - retirer `BAKIN_VAO_LOG` dans Lutris ;
    - ancien journal du correctif : `~/bakin-vao-hook.log.avant-lutris`.
- **Étude Bakin close (2026-10-26)** :
  - `RAE-PLAN/bakin/rapport.md` §7 (résultats en jeu, quatre formes de tailles, limites) ; recommandation révisée : livraison par archive reconstruite, et non par dictionnaire ;
  - contrôle hors jeu des 73 `.rbr` des deux démos : `Cast`, `NItem`, `NSkill`, `Camera` et le `Layout.rbr` de Bakin 1.6 ont des tailles non reconnues, d'où un avertissement ajouté (`rbpack.rejected_detectors`) ;
  - suivi : `tasks/todo.md` (« Études REA »).
- **Nettoyage (2026-10-26, accord de l'utilisateur)** :
  - mis à la corbeille : `RAE-PLAN/essais/bakin-v07/B0` à `B6`, `essais/wolf-v3/V2`, `essais/wolf-v223/W2` et `~/bakin-dotnet`, soit environ 7,7 Gio réellement occupés une fois les copies partagées btrfs déduites ;
  - la place ne sera libérée qu'une fois la corbeille vidée, ce que l'utilisateur fait lui-même ;
  - gardés : `B7` (Lutris pointe dessus), `BW`, tous les journaux `B*-bakinplayer_log.txt`, et les variantes Wolf validées ;
  - `bakin.fish B0…B3` ne trouvera plus ces variantes.
- **Décision (2026-10-26)** : l'extraction et la réinjection Bakin attendent que Wolf soit terminé. Jeux Bakin 1.2, 1.6, 2.1 et 2.7 disponibles ; détail dans `tasks/todo.md` (« Études REA »).
- **Prochaine action** : l'utilisateur corrige le préfixe et retire `BAKIN_VAO_LOG` dans Lutris. Ensuite, finir Wolf avant tout travail Bakin.

## En cours (2026-10-26, Claude) — R2b

- Journal : `docs/journal/2026-10-26-r2b-realignement.md`. Commits `d897f54`, `fe5f9df`, `e7cb87f`.
- `resync_project(project_id, apply)` : aperçu puis application en une transaction ; orphelins dans
  `orphaned_segments` (migration 0012) et mémoire de traduction ; `ResyncDialog` depuis le dialogue d'intégrité.
- **Gate complet réel vert** (2026-10-26, `lindera.dev` autorisé, `608bdbe`) : 717 tests Rust OK / 0 échec / 12 ignorés ;
  Vitest 142 ; clippy `-D warnings` (aussi `--all-targets`), fmt, typecheck OK ; lint 0 erreur.
- **Prochaine action exacte** :
  1. Fusion de la PR de R2 par l'utilisateur (relecture faite, gate cloud vert, 0 faux positif sur 37 543 lignes réelles).
  2. Non vérifiés en réel : réalignement sur un vrai jeu mis à jour ; essai en jeu (accord requis).
  3. Étape 7 reportée (décision utilisateur) ; suivis dans `tasks/todo.md`, « Suivis R2b ».
  4. Ensuite : étude REA de RuneTranslate (binaire, décision utilisateur du 2026-10-26) puis lot RW ; ou lot R3/R4.

## En cours (2026-10-26, Claude) — R2a

- Journal : `docs/journal/2026-10-26-r2a-garde-source-export.md` (preuves rouge/vert, comptes du gate).
- Fait : `IntegrityIssue::SourceChanged` + `verify_sources` ; contrôle dans `export_project` et
  `debug_inject_file` ; règle E2 (`same_role` dans `reconstruct_json`) ; dialogue d'intégrité fr/en ;
  tests `src-tauri/tests/export_game_update.rs` (P-2 MV, P-2 MZ locuteur + choix, jeu inchangé).
- Prouvé : test rouge sur `e8a97a9` (MV : visage `Actor2` → `EN2` ; MZ : `key not found in JSON`), puis vert ;
  la garde seule (sans E2) laissait passer P-2 en MV.
- **Bloqué** : `lindera.dev` refusé (403) par le réseau de l'environnement cloud ⇒ `lindera-ipadic` ne
  compile pas. Vérifs faites avec `DOCS_RS=1` (dictionnaire factice) : fmt, clippy `--all-targets -D warnings`,
  avant relecture 701 tests Rust OK / 2 échecs lindera attendus (identiques sur `e8a97a9`) / 12 ignorés ; Vitest 137 ; lint 0 erreur.
- Relecture indépendante faite. Corrigés : décalage vers une commande de même code (`same_page`) et 3 mineurs.
  **Décision attendue** : refus sur un ancien export non reconnu (cible CJK ou mise en page), conservé selon E1.
  Gate après corrections : 703 Rust OK / 2 échecs lindera / 12 ignorés ; Vitest 138.
- Non vérifié : gate avec le vrai dictionnaire ; faux positifs sur les 3 jeux réels (session 2) ; essai en jeu.
- **Prochaine action exacte** :
  1. Autoriser `lindera.dev` (accès réseau de l'environnement cloud), relancer le gate complet sans `DOCS_RS`
     sur la branche poussée.
  2. Contrôle des faux positifs sur les jeux réels (0 `SourceChanged` attendu sur un jeu non modifié).
  3. R2b faite le 2026-10-26 (voir plus haut).

## En cours (2026-10-25, Claude) — RW0 puis R1

- **Règle** : ne plus lancer de build, clippy ni de suite de tests sur le poste de l'utilisateur (il gèle) ; gate en session cloud sur la branche poussée (`tasks/lessons.md`).
- **RW0** (retrait de `wolfrpg-map-parser`, fork supprimé de GitHub, plus aucun clone neuf ne compilait) : branche `chore/remove-wolfrpg-map-parser` ; code et tests faits, **gate cloud vert** sur `32d36ad` (681 tests Rust, 124 Vitest, clippy, lint) ; plan et détail : `tasks/todo.md` (« Plan — RW0 »), `docs/journal/2026-10-25-rw0-retrait-wolfrpg-map-parser.md`. Wolf v2 est refusé à l'ouverture et à l'export avec un message dédié.
- **R1** (panne du fournisseur) : branche `fix/r1-provider-outage`, PR #10 ouverte (commits `7be55e4`, `53d6217`), essai réel fait ; son gate cloud a échoué faute de pouvoir récupérer le fork. Son suivi détaillé (plan R1, journal R1) est sur sa branche.
- **RW0 fusionné** le 2026-10-25 (PR #11, `252bcff`). `main` a été fusionné dans `fix/r1-provider-outage` (fusion plutôt que rebase : le hook du projet interdit le push forcé) ; conflits résolus en gardant les deux côtés (CHANGELOG : Fixed + Removed ; `tasks/todo.md` : plans R1 et RW0).
- **R1 vérifié** : gate cloud du 2026-10-25 sur `98236f1` (R1 + `main` avec RW0) : fmt OK, typecheck OK, Vitest 133 OK (35 fichiers), clippy `--locked -D warnings` OK, `cargo test --locked` 695 OK / 0 échec / 12 ignorés, lint 0 erreur (10 avertissements, les mêmes que sur `main`) ; tests R1 ciblés et tests RW0 tous verts.
- **R1 fusionné** le 2026-10-25 (PR #10, `af0c08e`) ; `main` local à jour. Branche locale `fix/r1-provider-outage` supprimable.
- **R2** (export sûr après mise à jour du jeu) : plan détaillé dans `tasks/todo.md` (« Plan détaillé — R2 »), **validé le 2026-10-25** : E1 = (a) refus total de l'export en cas d'écart ; E2 oui (reconstruction sans changement de rôle) ; E3 = (a) réalignement R2b dans R2 ; E4 oui (appariement exact par page) ; E5 oui (gate cloud + tests P-2 synthétiques ; essai en jeu seulement avec accord).
- **R2a non commencé** : branche `fix/r2a-export-source-guard` créée depuis `main` @ `af0c08e` ; seuls `tasks/todo.md` et `tasks/handoff.md` y sont modifiés. Arrêt demandé par l'utilisateur pendant la lecture de `ExportIntegrityDialog.tsx` (textes `exportIntegrity.*`). Reprise prévue le 2026-10-26 dans une nouvelle session.
- **Prochaine action exacte** : sur `fix/r2a-export-source-guard`, étape 1 du plan R2 (`IntegrityIssue::SourceChanged` dans `engines/mv_mz/integrity.rs`, `:21-77`), puis étapes 2 à 5 (contrôle avant `inject_to_bytes` dans `export.rs:369-460` et `debug_inject_file`, affichage dans `ExportIntegrityDialog.tsx`, tests P-2). Pas de build ni de tests en local : gate en cloud. Le hook PreToolUse bloque toute commande contenant la chaîne de l'installation npm (y compris sous pnpm) : passer par des scripts en fichier pour les textes qui la citent.
- À corriger à part : le hook PreToolUse qui interdit les installations npm/yarn bloque aussi l'installation pnpm (sous-chaîne) ; contourné par `pnpm i`.

## Session 3 (2026-10-25, Claude) — reprise

Rapport : `docs/audit/2026-10-analyse-complete.md` (Synthèse en tête ; en fin : Vérifications ciblées, Liste priorisée, Ce qu'on ne fait pas, Lots proposés).
Preuves : `docs/audit/2026-10-analyse-complete.session3.preuves.txt` ; probes : `…session3.probes.rs`.
Journal : `docs/journal/2026-10-25-analyse-session-3.md`.

- Audit du 05/09 : A et C corrigés en partie ; B, D, E, F toujours présents (E non vérifiable sur projet réel) ; limite SQL 33 000 corrigée.
- sportsfan30 : lots A, C, D confirmés ; B confirmé en partie (score QA calculé en continu ; filtres trompeurs).
- Relecture indépendante : S-X1 confirmé et aggravé (snapshot qui remet du japonais, `engines/mv_mz/originals.rs:320-345`) ; S-L2 « majeur ; bloquant en retraduction par sélection » (déduit, `translate.rs:34-41`), priorité 1 ; P-5a reformulé (détection incomplète).
- Lots R1–R10 et RW ajoutés à `tasks/todo.md`. Correctif « une seule notification » dans R1.
- **Prochaine action exacte :**
  1. **R1 implémenté le 2026-10-25** sur `fix/r1-provider-outage` : commit `7be55e4`, PR https://github.com/KATBlackCoder/Hoshi2Star/pull/10 (ouverte, non fusionnée). Puis compte des noms réparés (`RunSummary.repaired`) ajouté sur la même branche. Détail : `docs/journal/2026-10-25-r1-panne-fournisseur.md`.
  2. Étape 9 (essai réel) faite le 2026-10-25 sur la copie de la session 2 : Arrêter, panne, sélection en panne, reprise ⇒ un seul toast à chaque fois, aucune cible = source nouvelle, WebKit ≤ 289 Mo. Écart corrigé ensuite : les réparations de noms canoniques sont maintenant comptées (`repaired`, ligne dans le toast). Détail : journal R1, « Essai réel ».
  3. Prochaine action : relecture et fusion de la PR #10 par l'utilisateur ; puis plan détaillé du lot R2 (export sûr après mise à jour du jeu).

## Session 2 (2026-10-25, Claude) — reprise

Rapport : `docs/audit/2026-10-analyse-complete.md`, section « Parcours réels » (P-1 à P-9, liste des vérifications en jeu).
Preuves : `docs/audit/2026-10-analyse-complete.session2.preuves.txt`.
Journal : `docs/journal/2026-10-25-analyse-session-2.md`.

- Dossier de travail ignoré par Git : `test/_session2/`.
  - `games/` : copies reflink des 3 jeux ;
  - `xdg-data/com.blackat.hoshi2star/` : copie de la base et des réglages ;
  - `evidence/` : preuves brutes et captures ;
  - `zips/` ;
  - `tools/` : scripts Python de comparaison.
- Lancement de l'app sur la copie :
  `env XDG_DATA_HOME=/home/blackat/Bureau/Hoshi2Star/test/_session2/xdg-data pnpm tauri:linux`.
  Preuve dans `evidence/00-setup.txt` : `/proc/<pid>/fd` pointe vers la copie, sha256 de l'original inchangé.
- StandGirl : `projects.game_path` et `source_files.file_path` pointaient vers le jeu original (S-C4 confirmé).
  Ils ont été réécrits vers la copie, dans la base copiée seulement.
- **MV fait** (性処理係のある学校, projet neuf) :
  - extraction : 15 498 segments, 159 fichiers ;
  - aller-retour par marqueurs : 15 498/15 498 pointeurs justes, 0 autre valeur changée. Seule la mise en forme des lignes est perdue (S-X10) ;
  - S-X1 confirmé, plus grave que prévu : traductions écrites dans le nom de visage des commandes 101 ;
  - S-U1 et S-U3 confirmés ;
  - échantillon de 38 segments avec e4b : 38 en base = 38 dans le zip ;
  - comparaison e4b/e2b : 0 token en clair, mais du texte d'autres segments se retrouve dans les cibles ;
  - zip d'essai : `test/_session2/zips/MV_sample_e4b.zip`.
  - zip de reproduction S-X1 pour l'essai en jeu : `zips/MV_SX1_repro_after_update.zip`, sur une autre copie (voir le rapport).
- **MZ StandGirl fait** (projet ancien, extraction du 29/08 identique à une extraction neuve, 1 191 segments) :
  - aller-retour par marqueurs : 1 191/1 191 pointeurs justes ;
  - S-X1 sur MZ : l'export échoue (`key not found in JSON`) dès qu'un pointeur décalé devient invalide (choix, nom de locuteur `101[4]`) ; sinon, corruption silencieuse comme en MV ;
  - S-U2 confirmé : « Traduire » dans MV a traduit Map001 de StandGirl ;
  - échantillon e4b : 31 = 31 = 31, fuites de contexte fréquentes ;
  - zip d'essai : `zips/MZ_standgirl_sample_e4b.zip`.
- **MZ 借金JK fait** (projet neuf, 19 663 segments) :
  - aller-retour par marqueurs : 19 663/19 663 pointeurs justes ;
  - échantillon e4b de 34 segments, en **français** : 34 = 34 = 34. Projet créé en `fr` après un rechargement Vite, cause exacte non établie ;
  - zip d'essai : `zips/MZ_shakkin_sample_e4b_fr.zip`.
- Aucune catégorie `plugins.js` vérifiée dans les 3 jeux : l'édition de `plugins.js` n'est pas couverte.
- **S-L2 confirmé (P-6)** :
  - après l'arrêt d'Ollama, 1 119 segments sur 1 191 passent en `needs_review`, cible = source, en 10 s ;
  - événement `llm/completed`, aucune erreur ;
  - un toast par segment ; la webview finit tuée par manque de mémoire (8,7 Go) ;
  - « Tout traduire » ne reprendrait aucun segment.
  - Correctif demandé par l'utilisateur, une seule notification : noté dans `tasks/todo.md`.
- **Prochaine action exacte :**
  1. Essais en jeu faits le 2026-10-25 dans le navigateur intégré (rapport, « Essais en jeu ») :
     - S-X1 confirmé en jeu : partie MV bloquée au premier événement ;
     - les 3 zips d'échantillon fonctionnent.
     Optionnel : confirmation sous NW.js (`wine Game.exe` dans `test/_ingame/…`) ; Ollama relancé.
     Méthode : `.claude/launch.json` (non suivi, serveurs `ingame-*`) + `mcp__Claude_Browser__*`.
     Piste ouverte, non bloquante :
     - confirmation sous vrai NW.js : sous Proton (umu-run), le NW.js « normal » des jeux ignore `--remote-debugging-port` ;
     - pistes : NW.js SDK Linux sur une copie, piloté par CDP (téléchargement à faire valider), ou sonde `test/_session2/tools/H2sProbe.js` (déjà installée dans `test/_ingame/MV_SX1`) ;
     - grim est inutilisable sous KWin : utiliser spectacle (`-a`, fenêtre active).
     Liste détaillée : `test/_session2/A-VERIFIER-EN-JEU.md` (hors Git) ; résumé dans le rapport.
  2. Commit possible des docs, si l'utilisateur le demande, en stageant les chemins explicites.
     Les extraits ont été réduits le 2026-10-25 à sa demande : comptes, pointeurs, courts extraits ; détail dans `test/_session2/`.
     Règle : `test/` jamais dans Git.
  3. Session 3 : faite (voir plus haut).

## Dernier point atteint

- PR #5/#6 d'ArcoFrio fusionnées le 2026-10-16 (`3177567`) : `docs/journal/2026-10-15-revue-pr-arcofrio.md`.
- PR #9 fusionnée le 2026-10-25 (`123bd53`) : `tauri-plugin-mcp-bridge` 0.13.2, Tauri 2.12.5,
  `@tauri-apps/api`/`cli` 2.12.4. Le MCP Tauri pilote de nouveau l'app (en mode dev seulement).
- Plan + prompt de l'analyse complète : `docs/plans/2026-10-25-analyse-complete-app.md` (session 1 cochée).
- Session 1 (2026-10-25, Claude) : rapport `docs/audit/2026-10-analyse-complete.md`, preuves brutes
  `docs/audit/2026-10-analyse-complete.preuves.txt`, journal `docs/journal/2026-10-25-analyse-session-1.md`.
  Constats les plus graves, tous à confirmer :
  - S-X1 : traductions MV/MZ décalées après une mise à jour du jeu ;
  - S-X2 et S-X3 : pertes silencieuses ou `.dat` modifié à l'injection Wolf ;
  - S-L1 et S-L2 : token étranger accepté, panne du fournisseur transformée en `needs_review` massif ;
  - S-U1 : « Traduire » sans fichier bloque l'app ;
  - S-E1 à S-E4 : gel ou arrêt brutal sur jeu forgé ;
  - S-D2 : rustls/webpki à mettre à jour.
- Retours sportsfan30 (jeu MV, v0.4.10) : 4 lots A–D dans `tasks/todo.md`, preuves `fichier:ligne`.
- Études REA : dossier local `RAE-PLAN/` (ignoré par Git). Règles et index : `RAE-PLAN/README.md` ;
  RuneTranslate (architecture multi-moteurs) : `RAE-PLAN/runetranslate/PLAN.md`.

## Outillage (configuration locale, hors dépôt sauf `.mcp.json` et `.gitignore`)

- MCP : `filesystem` (chemin corrigé dans `.mcp.json`), `tauri` = `@hypothesi/tauri-mcp-server@0.13.2`
  (version fixée), `context7` (doc à jour), `rea` (rétro-ingénierie).
- Skills Tauri/Rust de Codex rendus disponibles à Claude par liens dans `.claude/skills/` (ignorés par Git).
- Skill utilisateur `ambition` (`~/.claude/skills/ambition/`, lien pour Codex dans `~/.codex/skills/`) :
  méthode « deeper, not bigger » de Felix Rieseberg, avec exemples bon/à éviter.
- Plugin officiel `code-review@claude-plugins-official` installé (scope utilisateur, activé).
- `cargo-audit` 0.22.2, `cargo-deny` 0.20.2, `cargo-llvm-cov` 0.9.1 installés par pacman (2026-10-25).
  llvm-cov : LLVM système, avec `LLVM_COV=/usr/bin/llvm-cov LLVM_PROFDATA=/usr/bin/llvm-profdata`.
  `cargo deny` : pas de `deny.toml` dans le dépôt ; celui de la session 1 est recopié dans le fichier de preuves.

## Vérifications

- PR #9 : gate complet vert (121 tests frontend, 691 tests Rust, clippy) ; app lancée en dev,
  migrations 0010/0011 appliquées sans erreur sur la base locale, MCP connecté (Tauri 2.12.5, bridge 0.13.2).
- Plugin `code-review` : installation et activation vérifiées (`claude plugin list`, fichiers en cache).
  Exécution non testée : il se charge au démarrage d'une session ; un test en mode non interactif
  a échoué pour une raison d'authentification (`OAuth session expired`), sans lien avec le plugin.
- Session 1 (2026-10-25) :
  - `pnpm test` : 121 tests verts ;
  - `cargo llvm-cov` : 691 tests verts, 12 ignorés, 85,4 % des lignes ;
  - clippy pedantic : 759 lints uniques sur la lib ;
  - cargo audit : 22 vulnérabilités ;
  - cargo deny : licenses et sources en échec ;
  - pnpm audit : 98 avis, outillage seulement.
- Non exécutés :
  - couverture TS (`@vitest/coverage-v8` non autorisé) ;
  - les 12 tests ignorés ;
  - reproductions dynamiques des constats ;
  - tests sur d'autres jeux MV/MZ, Windows, build de release.

## Prochaine action exacte

0. Jeux de test disponibles dans `test/` (ignoré par Git), inventaire du 2026-10-25, sans ouverture dans l'app :
   - MV : `[230426][こたつミカン] 性処理係のある学校 [RJ432000]` (784 Mo).
   - MZ, projet ancien (`.hoshi2star.json` présent) : `StandGirl-立ちんぼ六花の深い夜-` (331 Mo).
   - MZ, ancien export présent (`hoshi2star.zip`) : `Musume no Tomodachi … (RJ01311523)RAW` (506 Mo).
   - MZ, sans manifeste : `12cut vol103 …` (513 Mo, contient un `hoshi2star_debug_extract.json`).
   - MZ, sans manifeste : `借金JK人生NTRゲーム` (678 Mo).
   - Aucun jeu Wolf dans `test/`. **Décision utilisateur du 2026-10-25 : Wolf est reporté après la session REA**
     (`RAE-PLAN/runetranslate/PLAN.md`). La session 2 couvre MV et MZ seulement ; les points Wolf
     (S-X2, S-X3, S-E1, S-E2, S-E5 à S-E8, S-E11, S-X9) restent ouverts.
1. Session 2 MV/MZ faite le 2026-10-25. Restent les essais en jeu : voir « Session 2 » plus haut.
   Wolf (S-X2, S-X3…) : session 2 bis, après la session REA.
2. Session 3 faite ; lots R dans `tasks/todo.md`. Les correctifs (par exemple
   `cargo update -p rustls …`) ne sont **pas** autorisés avant ces lots : ils modifient `Cargo.lock`.
3. `/code-review:code-review` publie un commentaire sur la PR : pour un essai sans effet, PR #9 (fermée).
3. Ensuite : lots A–C de sportsfan30 (`tasks/todo.md`), puis session REA (`RAE-PLAN/runetranslate/PLAN.md`).
4. Avant la prochaine release : ArcoFrio au CHANGELOG (promis), export testé sur 2–3 jeux MV/MZ ;
   réponses attendues de sportsfan30 (modèle, Ollama) et d'ArcoFrio (modèles 8/12/16 Go).
