# Point de reprise — Hoshi2Star

**Mis à jour :** 2026-10-25 — Claude.
**Objectif utilisateur :** lancer une analyse complète de l'app dans une session dédiée, puis
traiter les retours de sportsfan30 (F95zone) et étudier RuneTranslate avec REA.
**Statut :** analyse complète, session 1 sur 3 faite (carte, menaces, contrôles automatiques).
Sessions 2 (jeux réels) et 3 (vérifications et rapport final) à faire. Aucun code applicatif modifié
depuis la PR #9. Rien n'est commité pour la session 1.

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
1. Session 2 du plan, avec son prompt prêt à coller :
   - demander d'abord les chemins des jeux MV, MZ, Wolf v2 et Wolf v3, le modèle d'essai, puis copier la base ;
   - suivre la liste « À confirmer sur jeux réels » du rapport, en priorité S-X1, S-X2 et S-X3.
2. Session 3, puis découpage en lots dans `tasks/todo.md`. Les correctifs (par exemple
   `cargo update -p rustls …`) ne sont **pas** autorisés avant ces lots : ils modifient `Cargo.lock`.
3. `/code-review:code-review` publie un commentaire sur la PR : pour un essai sans effet, PR #9 (fermée).
3. Ensuite : lots A–C de sportsfan30 (`tasks/todo.md`), puis session REA (`RAE-PLAN/runetranslate/PLAN.md`).
4. Avant la prochaine release : ArcoFrio au CHANGELOG (promis), export testé sur 2–3 jeux MV/MZ ;
   réponses attendues de sportsfan30 (modèle, Ollama) et d'ArcoFrio (modèles 8/12/16 Go).
