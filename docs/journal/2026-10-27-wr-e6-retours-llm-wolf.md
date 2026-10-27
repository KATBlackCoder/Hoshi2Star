# Journal — 2026-10-27 — WR-E6 : retours à la ligne Wolf côté LLM

**Phase** : F5
**Durée estimée** : 3h (plan + code)
**Statut** : 🔄 En cours (code et tests écrits ; gate cloud vert ; essai réel E6-7 concluant ; PR #18 fusionnée après #16)
**Outil** : Claude
**Branche et commit de départ** : `feat/wr-e6-soft-breaks` depuis `origin/feat/wr-wolf-wrap` @ `090e445` (PR #16 non fusionnée), worktree `../Hoshi2Star-e6`
**Objectif / plan / story** : décision WR-E6 du plan WR (constat K6) ; plan `tasks/todo.md`, « Plan — WR-E6 »

---

## Ce qui a été fait

- Reprise : PR #16 (`feat/wr-wolf-wrap`) encore ouverte ; le dossier principal porte des modifications non commitées d'une autre session (non touchées).
- Lecture du tokenizer (`llm/tokenizer.rs`, `engines/wolf/placeholders.rs`), du protocole de réponse (`llm/provider.rs`), de la restauration (`llm/split.rs`), de la QA (`core/qa.rs`, `core/report.rs`), des contextes (`llm/context/`), du module WR `engines/wolf/layout/` et de l'éditeur (`src/lib/constants.ts`, `features/editor/columns.tsx`).
- Mesure hors ligne des règles « structurelles » sur le projet DX (jeu 2.23) : copie en lecture seule de la base, script Python dans le dossier temporaire de la session (hors dépôt).
- Plan écrit (constats E6-K1 à K8, décisions E6-E1 à E6, étapes E6-0 à E6-7).
- Plan **validé** par l'utilisateur (recommandations), base : empiler sur `feat/wr-wolf-wrap` sans attendre la fusion de #16.
- E6-1 à E6-5 écrits (non compilés) :
  - `engines/wolf/line_breaks.rs` : `soft_breaks` (retours souples et `LinePlan` décidés sur la source), `rebreak` (coupe équilibrée en autant de lignes que la source, via `wrap(…, Some(n))` et `Metrics::Estimated`) ;
  - `llm/tokenizer.rs` : `Engine::WolfMessage`, `Engine::for_segment(…, json_key)`, `Tokenized.line_plan`, `Tokenized::restore` ;
  - pipeline : `llm/context/wolf.rs` lit `json_key` ; `split.rs` restaure par `Tokenized::restore` ; la relance sémantique reçoit la clé ;
  - QA : `QaSemanticContext.json_key`, paramètre `json_key` de `check_segment` ; appels mis à jour (pipeline, relance, `commands/qa.rs`, `commands/project.rs::update_segment`, `core/report.rs`, pilote, tests) ;
  - tests : `line_breaks` (11), tokenizer (4), QA (1), pipeline (1, fournisseur simulé, cas réel K6).
- `CHANGELOG.md` : entrée « Fixed ».
- Relecture indépendante (agent, lecture seule) : rien de bloquant. Corrigé : nom, note et jonction lus sur le texte visible (codes de couleur ignorés) ; `\f[n]` neutralisé pour la mesure de la coupe. 2 tests ajoutés (`line_breaks` : 11 au total).

## Fichiers modifiés

- `tasks/todo.md` — section « Plan — WR-E6 ».
- `tasks/handoff.md` — point de reprise WR-E6.
- `src-tauri/src/llm/tokenizer.rs`, `llm/split.rs`, `llm/pipeline.rs`, `llm/semantic_retry.rs`, `llm/context/mod.rs`, `llm/context/mv_mz.rs`.
- `src-tauri/src/core/qa.rs`, `core/report.rs`, `commands/qa.rs`, `commands/project.rs`, `commands/pilot.rs`, `engines/mv_mz/terminology.rs`.
- `src-tauri/src/engines/wolf/mod.rs` (module `line_breaks`), `engines/wolf/layout/fit.rs` (`is_label` en `pub(crate)`).
- `src-tauri/tests/e2e_project_flow.rs`, `tests/real_game_coverage.rs`, `tests/qa_v2_contract.rs`, `tests/terminology_real_pilot.rs` (nouveaux arguments `None`).
- `CHANGELOG.md`.

## Fichiers créés

- `docs/journal/2026-10-27-wr-e6-retours-llm-wolf.md` (ce fichier).
- `src-tauri/src/engines/wolf/line_breaks.rs`, `src-tauri/src/llm/context/wolf.rs`.

## Décisions prises

- E6-E1 à E6-E6 = recommandations du plan (validées le 2026-10-27).
- Sélection d'un message par la clé (`is_fitted_key`) : pas de migration ; `Engine::Wolf` reste le jeu de la base de données et de `filter::needs_translation`.

## Vérifications et limites

- Aucun build, aucun test, aucun clippy en local (règle du poste local). Seul `rustfmt --check` a été lancé sur les fichiers touchés : conforme.
- **Gate cloud vert** le 2026-10-27 sur `38033e9` (diffère de `f414e62` par `tasks/handoff.md` seulement) : fmt OK, typecheck OK, Vitest 153/153, clippy `--all-targets -D warnings` 0 avertissement, `cargo test --locked --no-fail-fast` **803 OK / 0 échec / 15 ignorés** (+17 par rapport aux 786 de `feat/wr-wolf-wrap`), lint 0 erreur (10 avertissements connus). Les 11 tests de `line_breaks`, les 4 du tokenizer, celui du pipeline et celui de la QA passent. Les 15 ignorés lisent de vrais jeux, absents en cloud.
- **Essai réel E6-7** (projet `APP`, jeu 2.23, app lancée depuis le worktree) : 27/27 messages retraduits par le LLM (26 entrées de TM fautives supprimées d'abord, sauvegarde `backups/hoshi2star-avant-essai-e6-20261027-215036.db`). Avant : 9 lignes de ponctuation seule, 5 lignes finales vides, mots perdus (#94, #95, #96). Après : **0 ligne de ponctuation seule, 0 ligne vide, 27/27 au nombre de lignes de la source**, lignes équilibrées (écart ≤ 4 caractères en général), `※` gardé, aucun mot perdu à la lecture, 27/27 `translated` (aucun `needs_review`). Scores QA 70 à 100 : −10 par ligne au-delà de 40 colonnes (`LineTooLong`, avertissement) ; la largeur reste à la mise en page de l'export (WR). Hors E6 : coupe après un trait d'union (« H- ⏎ scenes », #98) ; noms instables d'un message à l'autre (Gyoin/Uo-in, Inmame/Andō : glossaire).
- Mesure E6-K7 faite sur un seul jeu (2.23) ; règles structurelles non mesurées sur 3.658 ni sur d'autres jeux.

## Prochaine session

- PR #16 fusionnée (`e47a1e5`), puis PR #18 repointée vers `main` et fusionnée, le 2026-10-27, à la demande de l'utilisateur. Essai en jeu de l'export des messages retraduits facultatif. Limites gardées : décoration de bord contenant un `\n` (cible laissée telle quelle) ; espaces de tête retirées d'un bloc recoupé.

---
*Journal partagé Codex / Claude — Hoshi2Star*
