# Journal — 2026-07-02 — Dispatch moteur centralisé (remédiation audit Phase 8)

**Phase** : Remédiation audit — Phase 8 (dispatch moteur, OCP)
**Durée estimée** : 4h
**Statut** : ✅ Complété

---

## Ce qui a été fait

Centralisation du dispatch moteur, dispersé sur ≥4 sites (`match engine` +
`file_type.starts_with("wolf_"|"vx_")`). Refactor **majeur, iso-comportement** :
gate 100% vert sans modifier un test existant (ajout d'un test de caractérisation
autorisé). Découpé en 3 commits pour rester bisectable.

**Décision de design (ADR-006) : enum + méthodes, PAS de trait-objects.**
Ensemble de moteurs fermé et petit (3→4) → un `enum` fermé + `match` exhaustif
donne la vérification de complétude du compilateur pour zéro coût, là où un
trait-object serait un diff plus gros/risqué sans gain réel.

- **Étape 0 (commit `4c28f54`)** — test de caractérisation
  `mv_debug_dump_matches_open_project_extraction` : `debug_dump_segments` doit
  extraire exactement les mêmes `(json_key, source_text)` par fichier
  qu'`open_project` persiste. Verrouille le chemin debug (non couvert par les
  round-trips e2e) AVANT le refactor. `serde_json` ajouté aux dev-deps.
- **Commit A (`59c25a3`)** — décomposition de la god-fn `open_project` (~155 →
  ~25 lignes) : `extract_project(engine, game_dir, data_dir)` normalise les 3
  moteurs vers une forme commune (`ExtractedFile`/`ExtractedFileSeg`), partagée
  par `open_project` et `debug_dump_segments` ; helpers `insert_source_file` /
  `insert_segment` prenant `&mut Transaction` (transaction unique + atomicité
  préservées). Aucune abstraction nouvelle.
- **Commit B (`924ea84`)** — dispatch centralisé : `impl Engine { db_str,
  data_dir, game_title }` dans detector.rs (les 3 lecteurs de titre migrent de
  project.rs vers la couche moteur) ; `filter::classify_file_type → FileClass`
  route `export_project`, `collect_wolf_zip_entries`, `debug_inject_file`
  (4 `starts_with` supprimés). `extract_project` reçoit désormais le `data_dir`
  résolu une fois via `Engine::data_dir`.

## Fichiers créés

- docs/adr/ADR-006.md
- docs/journal/2026-07-02-engine-dispatch-phase-8.md

## Fichiers modifiés

- src-tauri/tests/e2e_project_flow.rs — test de caractérisation (étape 0)
- src-tauri/Cargo.toml — `serde_json` en dev-dep
- src-tauri/src/commands/project.rs — décomposition open_project + extract_project
  + helpers insert ; 3 matches métadonnées → méthodes Engine ; 3 lecteurs de titre
  retirés (déplacés vers detector.rs)
- src-tauri/src/engines/detector.rs — `impl Engine { db_str, data_dir, game_title }`
  + les 3 lecteurs de titre
- src-tauri/src/engines/filter.rs — `FileClass` + `classify_file_type`
- src-tauri/src/commands/export.rs — 4 sites `starts_with` → `classify_file_type`
- docs/architecture.md, CHANGELOG.md — dispatch moteur documenté

## Fichiers supprimés

- (aucun)

## Décisions prises

- **ADR-006** : dispatch moteur par méthodes sur enum ; trait-objects rejeté.
- **`Engine::from_db_str` retiré avant merge** : ajouté en commit B pour un
  consommateur export.rs qui ne s'est jamais matérialisé (font scopé hors
  périmètre) → code mort, que `pub` masquait au `clippy dead_code`. Retiré par
  discipline de portée (CLAUDE.md). Repéré par l'advisor.
- **Hors périmètre (laissé tel quel)** : `tokenizer.rs:95/135`, `qa.rs:225`,
  helpers font `export.rs` (`engine=="wolf"`) — config par moteur, pas dispatch
  de fichier.

## Problèmes rencontrés

- Import `engines::{...}` d'export.rs : match Edit initial échoué (rendu `::{`),
  résolu en ancrant sur la ligne `detector::` unique.
- `from_db_str` code mort non détecté par le gate (`pub` supprime `dead_code`) —
  relevé par l'advisor, retiré + gate re-vérifié.

## Couverture / vérification

- Gate complet vert : 367 tests unit Rust + 3 intégration + 23 front, clippy
  exit 0, typecheck clean, fmt clean. **Aucun test existant modifié.**
- Couvert e2e : `open_project`+`export_project` MV & Wolf (round-trip Wolf CE
  verrouille toujours Phase 1).
- Non couvert e2e → prouvé autrement : `debug_dump_segments` par le test de
  caractérisation (étape 0) ; `debug_inject_file` par relecture (3 voies
  inchangées) ; bras VX Ace d'`open_project` par relecture (moteur désactivé,
  sans fixture ; appel extracteur intact).

## Tâches ROADMAP cochées

- [ ] (Phase de remédiation audit — hors ROADMAP F0–F5)

## Prochaine session

- Phase 9 (perf) — dernière du plan de remédiation. À déclencher sur signal, pas
  prématurément. Plan + validation utilisateur avant tout code.

---
*Généré par Claude Code — Hoshi2Star*
