# Journal — 2026-10-25 — Analyse complète, session 1 (carte et contrôles automatiques)

**Phase** : F5 (maintenance, avant release publique)
**Durée estimée** : 3h
**Statut** : ✅ Complété (session 1 seulement)
**Outil** : Claude
**Branche et commit de départ** : `main` @ `340e636`
**Objectif / plan / story** : session 1 de `docs/plans/2026-10-25-analyse-complete-app.md`. Lecture et contrôles seulement : aucun code applicatif modifié, aucun jeu réel ouvert.

---

## Ce qui a été fait

- Reprise selon `AGENTS.md` : `tasks/handoff.md`, `tasks/lessons.md`, `CONTEXT.md`, état Git.
- Cartographie par cinq sous-agents en lecture seule, lancés en parallèle : engines, export, llm, core/commandes/base, UI.
- Relecture manuelle des constats majeurs :
  - cycle de répertoires DXA v8 ;
  - débordement `u16` de `build_sfnt` ;
  - chemin de police ;
  - `Tokenizer::validate` ;
  - `translate_segments` sans fichier, avec son pendant UI ;
  - CSP.
- Modèle de menaces des entrées non fiables.
- Contrôles automatiques : clippy pedantic, cargo audit, cargo deny, couverture Rust (llvm-cov), pnpm audit, pnpm test.
- Skill `security-review` appliqué à tout `main`, faute de diff.
- Points à confirmer en session 2 sur jeux réels.

## Fichiers créés

- `docs/audit/2026-10-analyse-complete.md` : sections Architecture, Menaces, Constats statiques, À confirmer sur jeux réels.
- `docs/audit/2026-10-analyse-complete.preuves.txt` : sorties brutes résumées des outils, et `deny.toml` utilisé.
- `docs/journal/2026-10-25-analyse-session-1.md` (ce fichier).

## Fichiers modifiés

- `docs/plans/2026-10-25-analyse-complete-app.md` : session 1 cochée, statut mis à jour.
- `tasks/handoff.md` : point de reprise.

## Fichiers supprimés

- Aucun.
- La suppression locale de `docs/plans/2026-10-25-sessions-suivantes.md` existait avant la session. Elle a été laissée telle quelle : ni restaurée, ni commitée.

## Dépendances ajoutées

- Aucune dans le dépôt.
- Sur le poste, installés par l'utilisateur après accord : `cargo-audit` 0.22.2, `cargo-deny` 0.20.2, `cargo-llvm-cov` 0.9.1 (pacman).
- `@vitest/coverage-v8` non installé : non autorisé.

## Décisions prises

- `cargo deny` lancé avec une config temporaire hors dépôt (scratchpad), recopiée dans le fichier de preuves.
- `cargo llvm-cov` utilise la LLVM système (23.1.3, identique à celle de rustc), via `LLVM_COV` et `LLVM_PROFDATA`, sans rustup.
- Statuts du rapport : vérifié-exéc., vérifié-code (« relu » si relu par la session principale), déduit, inconnu.

## Problèmes rencontrés

- La session a redémarré pendant l'audit. Les sorties du scratchpad et un `cargo test` en arrière-plan ont été perdus. Les mesures ont été refaites (llvm-cov a relancé tous les tests).
- `cargo deny check --config` est refusé : l'option `--config` se place avant `check`.

## Corrections et erreurs à ne plus répéter

- Rien de nouveau pour `tasks/lessons.md`.
- Rappel appliqué : garder les preuves brutes dans le dépôt, puisque le scratchpad peut disparaître.

## Vérifications et limites

- `pnpm test` : 34 fichiers, 121 tests verts.
- `cargo llvm-cov --summary-only` : 691 tests verts, 12 ignorés (8 d'intégration, 4 de la lib), 85,4 % des lignes.
- `cargo clippy --all-targets -- -W clippy::pedantic` : sortie 0, 961 avertissements, dont 759 uniques sur la lib.
- `cargo audit` : 22 vulnérabilités et 10 avertissements. Celles qui comptent : rustls / webpki, 7 avis.
- `cargo deny` : bans ok (36 doublons) ; licenses en échec (pas de champ `license`, `marshal-rs` WTFPL) ; sources en échec (fork git `wolfrpg-map-parser`).
- `pnpm audit` : 98 avis, dont 77 en prod, tous via la CLI `shadcn` ou l'outillage de build.
- Non exécutés :
  - couverture TS ;
  - les 12 tests ignorés ;
  - typecheck et clippy `-D warnings` (aucun code modifié) ;
  - tout essai sur jeu réel ;
  - toute reproduction dynamique des constats. Ils sont prouvés par lecture, pas par un test qui échoue.

## Tâches ROADMAP cochées

- Aucune.

## Prochaine session

- Session 2 du plan.
- D'abord, demander à l'utilisateur :
  - les chemins des jeux MV, MZ, Wolf v2 et Wolf v3 ;
  - le modèle LLM d'essai ;
  - l'accord pour copier la base.
- Suivre la liste « À confirmer sur jeux réels » du rapport, en priorité S-X1, S-X2 et S-X3.

---
*Journal partagé Codex / Claude — Hoshi2Star*
