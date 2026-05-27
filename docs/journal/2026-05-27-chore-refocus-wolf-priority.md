# Journal — 2026-05-27 — Chore : Refocus Wolf RPG Priority

**Phase** : F3 / Chore
**Durée estimée** : 30min
**Statut** : ✅ Complété

---

## Ce qui a été fait

- Désactivation de la détection VX Ace dans `detector.rs` (bloc commenté avec note explicite)
- Marquage `#[ignore]` des deux tests VX Ace dans `detector.rs` avec raison documentée
- Renommage F3 dans ROADMAP.md : "Polissage + Glossaire + TM fuzzy + beta privée"
- Renommage F4 dans ROADMAP.md : "Wolf RPG (priorité absolue)" avec justification marché (~40% DLsite)
- Section "Moteurs — ordre de priorité" ajoutée dans ROADMAP.md
- Engine Layer VX Ace F3 remplacé par entrée `[-]` (reporté, code disponible mais désactivé)
- CONTEXT.md architecture : ligne engines/ mise à jour (wolf priorité F4, vx_ace désactivé)
- CONTEXT.md contexte produit : note moteurs prioritaires + VX Ace désactivé volontairement
- CONTEXT.md : section "Workflow Git — branches" ajoutée avec types, critères merge, commandes
- CHANGELOG.md mis à jour (Changed + Added)
- Branche `chore/refocus-wolf-rpg-priority` → merge --no-ff sur main → push

## Fichiers créés

- `docs/journal/2026-05-27-chore-refocus-wolf-priority.md` (ce fichier)

## Fichiers modifiés

- `src-tauri/src/engines/detector.rs` — bloc VX Ace commenté, 2 tests marqués `#[ignore]`
- `ROADMAP.md` — renommage F3/F4, table priorités moteurs, section VX Ace reporté
- `CONTEXT.md` — architecture engines/, contexte produit, workflow Git branches
- `CHANGELOG.md` — entrées Changed/Added

## Fichiers supprimés

- *(aucun)*

## Décisions prises

- **VX Ace désactivé** : le code `engines/vx_ace/` est complet et fonctionnel (130 tests),
  mais la détection est commentée pour éviter que les utilisateurs ouvrent des projets VX Ace
  sur lesquels on ne peut pas encore s'engager (`.rgss3a` non supporté, pas de QA workflow VX Ace).
  Réactivation triviale : décommenter un bloc dans `detector.rs`.
- **Wolf RPG priorité absolue** : ~40% des jeux JP non traduits sur DLsite sont Wolf RPG.
  RuneTranslate (concurrent) le supporte déjà. F4 = point de différentiation critique.
- **Workflow Git documenté dans CONTEXT.md** : convention branches pour les prochaines sessions.

## Problèmes rencontrés

- *(aucun)*

## Tâches ROADMAP cochées

- *(aucune nouvelle tâche F3 — session chore/refactor uniquement)*

## Résultats tests

- `cargo test` : 128 passed, 2 ignored, 0 failed
- `cargo clippy -D warnings` : clean
- `pnpm typecheck` : clean

## Prochaine session

- F3 : démarrer TM v2 (fuzzy matching Levenshtein) — `src-tauri/src/core/tm.rs`
- F3 : démarrer Glossaire v1 — migration SQL + `core/glossary.rs`
- Wolf RPG F4 : recherche format `.dat/.mps`, évaluer approche (bindings Rust natifs vs sidecar)

---
*Généré par Claude Code — Hoshi2Star*
