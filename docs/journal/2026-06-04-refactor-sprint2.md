# Journal — 2026-06-04 — Refactoring Sprint 2 — Split commands/project.rs

**Phase** : Maintenance / Refactoring
**Durée estimée** : 8h (estimé) / ~3h (réel — split mécanique)
**Statut** : ✅ Complété

---

## Ce qui a été fait

### Split de commands/project.rs (1 539 lignes → 5 fichiers)

| Fichier | Contenu | Lignes |
|---------|---------|--------|
| `commands/project.rs` | CRUD projet/fichiers/segments + helpers extraction | 727 |
| `commands/translate.rs` | translate_segments, translate_all_segments, get_ollama_models | 449 |
| `commands/export.rs` | export_project, export_qa_report, export_tm, export_debug_json | 195 |
| `commands/qa.rs` | qa_check_segment, get_qa_report, get_tm_suggestions | 93 |
| `domain/types.rs` | Project, SourceFile, Segment, ProviderConfig, QaReport, ProjectStats, OpenProjectResult, PaginatedSegments | 108 |

### Autres changements

- Créé `src-tauri/src/domain/mod.rs` + `src-tauri/src/domain/types.rs`
- Mis à jour `commands/mod.rs` — ajouté `export`, `qa`, `translate`
- Mis à jour `lib.rs` — imports répartis sur 4 modules au lieu de 1
- Corrigé `commands/glossary.rs` — `ProviderConfig` importé depuis `domain::types` au lieu de `commands::project`
- CHANGELOG.md mis à jour

## Fichiers créés

- `src-tauri/src/commands/export.rs`
- `src-tauri/src/commands/qa.rs`
- `src-tauri/src/commands/translate.rs`
- `src-tauri/src/domain/mod.rs`
- `src-tauri/src/domain/types.rs`
- `docs/journal/2026-06-04-refactor-sprint2.md` (ce fichier)

## Fichiers modifiés

- `src-tauri/src/commands/project.rs` — réduit de 1 539 à 727 lignes
- `src-tauri/src/commands/mod.rs` — 3 nouveaux modules
- `src-tauri/src/commands/glossary.rs` — import ProviderConfig corrigé
- `src-tauri/src/lib.rs` — imports répartis
- `CHANGELOG.md`

## Décisions prises

- **Helpers privés restent dans project.rs** — `read_game_title`, `classify_mv_mz_file`, `dispatch_extract` etc. sont exclusivement utilisés par `open_project`. Les déplacer dans un module dédié ne serait pas justifié (couplage trop fort avec le contexte projet).
- **727 lignes > 550 cible** — La cible était une estimation. `open_project` est intrinsèquement complexe (détection moteur + transaction multi-fichiers + manifest). Réduire davantage nécessiterait des changements de comportement.
- **Commit F3-12 préalable** — Les modifications non-commitées (translate_all_segments, ProjectStats, i18n) ont été commitées en premier pour avoir une base propre avant le refactoring.
- **Import circulaire évité** — `domain::types` importe uniquement depuis `llm::provider` (constantes DEFAULT_*), ce qui ne crée pas de cycle.

## Problèmes rencontrés

1. **glossary.rs importait `commands::project::ProviderConfig`** — lien de couplage non documenté découvert uniquement au premier `cargo build`. Corrigé en changeant l'import vers `domain::types::ProviderConfig`.
2. **Hook cargo fmt** a reformaté `project.rs` après une première tentative d'Edit partielle → nécessité de réécrire le fichier entier proprement.

## Résultats finaux

```
pnpm typecheck     → 0 erreur ✅
cargo clippy -D warnings → 0 warning ✅
cargo test         → 171/171 passed ✅
```

## Taille finale fichiers

```
195  commands/export.rs
116  commands/glossary.rs
  8  commands/mod.rs
727  commands/project.rs  (était 1539)
 93  commands/qa.rs
449  commands/translate.rs
108  domain/types.rs
```

## Tâches ROADMAP

Aucune tâche ROADMAP cochée (maintenance interne).

## Prochaine session

- **F4 Wolf RPG** — priorité absolue (ROADMAP.md)
- Sprint 3 refactoring (R-05 + R-08) : split llm/pipeline.rs → pipeline.rs + split.rs + progress.rs
- Beta privée : recrutement testeurs Discord/F95zone

---
*Généré par Claude Code — Hoshi2Star*
