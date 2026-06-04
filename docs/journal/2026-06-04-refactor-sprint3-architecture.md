# Journal — 2026-06-04 — Refactoring Sprint 3 + docs/architecture.md

**Phase** : Maintenance / Refactoring + Documentation
**Durée estimée** : 5h (estimé) / ~1.5h (réel)
**Statut** : ✅ Complété

---

## Ce qui a été fait

### R-05 — Split llm/pipeline.rs (718 lignes → 3 fichiers)

| Fichier | Contenu | Lignes |
|---------|---------|--------|
| `llm/pipeline.rs` | run / run_inner / translate_batch + tous les tests | ~240 |
| `llm/split.rs` | llm_translate_with_split + MAX_RETRIES | 127 |
| `llm/progress.rs` | ProgressPayload, PlaceholderWarningPayload | 21 |

Rationale du split :
- `split.rs` isole la logique récursive la plus complexe du pipeline — facilite le débogage en production quand le LLM retourne des réponses partielles
- `progress.rs` rassemble les types des events `h2s://llm/*` — un seul endroit à lire pour comprendre le protocole Rust → TypeScript

### R-08 — Labels QA vers QaError::label()

- Ajout de `impl QaError { pub fn label(&self, lang: &str) -> String }` dans `core/qa.rs`
- Suppression de `error_label_en` et `error_label_fr` dans `core/report.rs` (fonctions privées)
- Call site mis à jour : `escape_xml(&err.label(if use_fr { "fr" } else { "en" }))`
- Avantage : si un nouveau `QaError` variant est ajouté, un seul endroit à mettre à jour (pas 3)

Décision de design : les labels dans `qa.rs` ne font PAS d'`escape_xml` — le texte brut appartient au domaine QA. C'est `report.rs` (couche HTML) qui applique l'échappement. Cette séparation respecte les responsabilités de chaque couche.

### Tâche 2 — docs/architecture.md

Créé `docs/architecture.md` (~220 lignes) documentant :
- Vue d'ensemble 5 couches avec flux Rust ↔ TypeScript
- Stack technique (table)
- Rust backend : domain/types, commands/ (5 modules), core/ (5 modules), llm/ (6 modules), engines/, utils/, db/
- TypeScript frontend : stores/ (4 stores), components/editor/ (6 composants), components/ (3 modales), lib/ (5 fichiers), features/
- 3 flux de données détaillés (open project, translate, smart restore)
- Table des 5 ADRs avec liens
- Section "Ce qui n'est PAS dans cette version"

## Fichiers créés

- `src-tauri/src/llm/progress.rs`
- `src-tauri/src/llm/split.rs`
- `docs/architecture.md`
- `docs/journal/2026-06-04-refactor-sprint3-architecture.md` (ce fichier)

## Fichiers modifiés

- `src-tauri/src/llm/pipeline.rs` — réduit de 718 à ~240 lignes
- `src-tauri/src/llm/mod.rs` — ajout `progress` + `split`
- `src-tauri/src/core/qa.rs` — ajout `impl QaError::label()`
- `src-tauri/src/core/report.rs` — suppression `error_label_en/fr`, call site simplifié
- `CHANGELOG.md`

## Décisions prises

- **Retour `String` et non `&str`** pour `QaError::label()` — les labels contiennent des données dynamiques (numéro de ligne, unités, nom du placeholder) ; `&str` est impossible sans heap allocation.
- **Pas d'`escape_xml` dans qa.rs** — `core/qa.rs` est une couche domaine pure, pas HTML. L'échappement reste dans `core/report.rs`.
- **`MAX_RETRIES` migré dans split.rs** — la constante n'est utilisée que dans `llm_translate_with_split` ; la placer avec la fonction qui l'utilise.

## Problèmes rencontrés

Aucun. Les deux refactorings étaient mécaniques. Hook cargo fmt a reformaté qa.rs après l'Edit, mais la lecture préalable avant tout edit ultérieur a évité les conflits.

## Résultats finaux

```
pnpm typecheck           → 0 erreur ✅
cargo clippy -D warnings → 0 warning ✅
cargo test               → 171/171 passed ✅
```

## Taille finale des fichiers modifiés

```
 21  llm/progress.rs  (nouveau)
127  llm/split.rs     (nouveau)
240  llm/pipeline.rs  (était 718)
  6  llm/mod.rs
500  core/qa.rs       (était ~430)
640  core/report.rs   (était 705)
```

## Tâches ROADMAP

Aucune tâche ROADMAP cochée (maintenance interne + documentation).

## Prochaine session

- **F4 Wolf RPG** — priorité absolue (ROADMAP.md) : `engines/wolf/extractor.rs`, `decryptor.rs`, `injector.rs`
- Sprint 4 refactoring (R-04) : extraire `AppToolbar.tsx`, `AppDialogs.tsx`, `useAppHandlers.ts` depuis `App.tsx`
- Beta privée : recrutement testeurs Discord/F95zone

---
*Généré par Claude Code — Hoshi2Star*
