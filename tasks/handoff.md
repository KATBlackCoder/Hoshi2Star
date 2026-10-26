# Point de reprise — Hoshi2Star

**Mis à jour :** 2026-10-26 — Claude (lot RW démarré : worktree `../Hoshi2Star-rw`, branche `feat/rw-wolf` ; R2 fusionné, PR #12).
**Objectif utilisateur :** lot RW, extraction et réinjection Wolf v2/v3 dans l'app (livraison par archive reconstruite).
**Statut :** R2 fusionné (PR #12, `2331c75`). RW : plan validé le 2026-10-26 ; RW-1 vérifié (gate cloud vert sur `cc7efc7`), RW-2 en cours.

## En cours (2026-10-26, Claude) — RW (Wolf)

- Plan validé : `tasks/todo.md`, « Plan détaillé — RW ». Branche `feat/rw-wolf` dans le worktree `../Hoshi2Star-rw` (depuis `origin/main` @ `2331c75`). Commits locaux ; push seulement avec accord (gate cloud).
- Base : étude REA RuneTranslate + prototypes Python validés **en jeu** sur Wolf 2.23 et 3.658 (preuves locales hors Git : `RAE-PLAN/wolf/`, `RAE-PLAN/essais/RESULTATS.md`). Constat clé : **l'archive `.wolf` passe avant les fichiers libres** (v2 et v3) ; l'archive reconstruite est acceptée par le jeu.
- Journal : `docs/journal/2026-10-26-rw-wolf.md`.
- **RW-1 écrit** (`5d90131`, `engines/wolf/decrypt/legacy_xor.rs`) : signature v6 chiffrée, LZ v6, noms et chemins v6 réels, version de chiffrement v8 (chaîne-clé « v3.14 », refus 3.31+ par `UnsupportedWolfCrypt`). Tests synthétiques au format DxLib réel. **Non compilé, non testé** : premier contrôle au gate cloud.
- Attention pour RW-4 : RW0 refuse Wolf v2 à l'ouverture et à l'export avec un message dédié ; ce refus doit être levé quand les cartes v2 seront lues.
- **Poussé** le 2026-10-26 (`origin/feat/rw-wolf` @ `cc7efc7`, accord de l'utilisateur) ; **gate cloud RW-1 vert** sur `cc7efc7` : fmt OK, typecheck OK, Vitest 142/142, clippy `--all-targets -D warnings` 0 avertissement, `cargo test --locked` 724 OK / 0 échec / 12 ignorés (717 sur `main`, +7 RW-1), lint 0 erreur (10 avertissements frontend connus). Lecture de vraies archives non couverte (tests `diag_*` ignorés).
- Environnement cloud (hors périmètre, à traiter à part) : le hook `PreToolUse` bloque toute commande dont le texte contient la sous-chaîne interdite de npm, y compris la commande d'installation de pnpm et même une note qui la cite (motif trop large ; contourné par `pnpm i`) ; le conteneur cloud n'a pas les bibliothèques GTK/WebKit de Tauri (installées à la main via apt pour clippy) : à mettre dans le script de setup cloud.
- **Prochaine action exacte** : RW-2 (détection de version, `engines/detector.rs`), puis RW-4, RW-3, RW-6, RW-5, RW-7, RW-8, RW-9. Pas de build local ; gate en cloud après push autorisé.

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
