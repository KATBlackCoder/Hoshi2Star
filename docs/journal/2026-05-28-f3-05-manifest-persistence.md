# Journal — 2026-05-28 — F3-05 : Manifest .hoshi2star.json

**Phase** : F3 — Persistence projet
**Durée estimée** : ~60–80 min
**Statut** : ✅ Complété

---

## Ce qui a été fait

Implémentation complète de F3-05 (plan `docs/plans/f3-05-manifest-persistence.md`),
5 steps exécutés dans l'ordre. Feature A (Steps 1–2) : écriture manifest.
Feature B (Step 3) : réouverture intelligente. Steps 4–5 : stats + toast.

### Step 1 — core/manifest.rs
Nouveau fichier `src-tauri/src/core/manifest.rs` contenant :
- `ManifestStats` struct avec `#[serde(rename_all = "camelCase")]`
- `ManifestData` struct avec factory `ManifestData::new(...)` — encapsule `now_iso8601()`
- `now_iso8601()` : format ISO 8601 `"YYYY-MM-DDTHH:MM:SSZ"` via calcul Grégorien,
  adapté de `unix_timestamp_to_date_str()` de `report.rs`, 0 dépendance externe
- `write_manifest()` : `std::fs::write` (sync, OK pour < 1 KB)
- `read_manifest()` : `NotFound → Ok(None)`, JSON corrompu → `Ok(None)` + `log::warn!`
- `update_stats()` : lit, modifie stats + `last_opened_at`, réécrit
- `pub const VERSION: &str = env!("CARGO_PKG_VERSION")`
- 4 tests unitaires avec `tempfile::tempdir()`
- `log = "0.4"` ajouté dans `[dependencies]` (absent du projet auparavant)

### Step 2 — Écriture manifest dans open_project
Dans `commands/project.rs` : compteurs `file_count`/`segment_count` incrémentés
dans les deux branches `Engine::MvMz` et `Engine::VxAce`. Après `tx.commit()`,
appel `manifest::write_manifest()` — erreur loguée et ignorée (`best-effort`).

### Step 3 — OpenProjectResult + réouverture intelligente
- Struct `OpenProjectResult { project: Project, was_restored: bool }` dans `project.rs`
- Signature `open_project` changée de `Result<Project, String>` en
  `Result<OpenProjectResult, String>`
- Logique ajoutée **avant** `detect_engine()` :
  1. `manifest::read_manifest()` → si `Some(mf)`
  2. `SELECT COUNT(*) FROM projects WHERE id = mf.project_id`
  3. Si > 0 → `update_stats(path, mf.stats.clone())` pour rafraîchir `last_opened_at`,
     puis retourner projet depuis DB avec `was_restored: true`
  4. Si = 0 → `preserved_project_id = Some(mf.project_id.clone())`
- `project_id` généré via `preserved_project_id.unwrap_or_else(|| uuid::Uuid::new_v4()...)`
- TypeScript : `OpenProjectResult` interface dans `src/lib/types.ts`,
  `openProject()` retourne `OpenProjectResult` dans `stores/project.ts`

### Step 4 — Mise à jour stats manifest
- `update_segment` : après UPDATE DB, requête SQL unique avec 4 sous-requêtes
  `COUNT(*)` pour récupérer `(game_path, files, segs, translated, glossary)`,
  appel `manifest::update_stats()` — erreur ignorée silencieusement
- `translate_segments` : `resolved_project_id` capturé au moment du `pid` déjà
  résolu pour le glossaire, requête stats en partant de `p.id = ?` (project direct),
  appel `manifest::update_stats()` une seule fois avant `h2s://llm/completed`

### Step 5 — Toast was_restored + i18n
- `stores/project.ts` : `import { t } from 'i18next'` + `import { toast } from 'sonner'`
- `if (wasRestored) { toast.success(t('project.restored')) }` après invoke
- `en.json` : `"project": { "restored": "Project restored — continuing where you left off" }`
- `fr.json` : `"project": { "restored": "Projet restauré — reprise de la traduction" }`

---

## Fichiers créés

- `src-tauri/src/core/manifest.rs` — module manifest complet (224 lignes)
- `docs/journal/2026-05-28-f3-05-manifest-persistence.md` (ce fichier)

## Fichiers modifiés

- `src-tauri/src/core/mod.rs` — `pub mod manifest;` ajouté
- `src-tauri/Cargo.toml` — `log = "0.4"` ajouté dans `[dependencies]`
- `src-tauri/src/commands/project.rs` — manifest import, compteurs, write_manifest,
  OpenProjectResult, logique réouverture, update_stats dans update_segment +
  translate_segments, resolved_project_id
- `src/lib/types.ts` — interface `OpenProjectResult`
- `src/stores/project.ts` — adapt invoke + toast + i18n imports
- `src/locales/en.json` — section `"project"` avec clé `"restored"`
- `src/locales/fr.json` — idem en français
- `ROADMAP.md` — section "Persistence projet" + item coché
- `CHANGELOG.md` — 6 entrées Added
- `docs/plans/f3-05-manifest-persistence.md` — statut Complété

## Fichiers supprimés

*(aucun)*

## Dépendances ajoutées

- `log = "0.4"` — crate Rust standard pour `log::warn!` (très léger, probablement
  déjà dépendance transitive de Tauri, mais explicitement déclaré pour clarté)

## Décisions prises

**`now_iso8601()` privée** : exposer via `ManifestData::new()` et `update_stats()`.
Quand `open_project` veut mettre à jour `last_opened_at` (cas restore), il appelle
`update_stats(&path, mf.stats.clone())` — stats inchangées, seul le timestamp
est rafraîchi. Évite d'exposer `now_iso8601` publiquement.

**`update_stats` pour rafraîchir `last_opened_at`** : passer `mf.stats.clone()` préserve
les compteurs existants tout en déclenchant la mise à jour du timestamp via la fonction
privée. Clean, sans ajouter de surface d'API.

**`resolved_project_id`** dans `translate_segments` : déclaré `let mut` avant le bloc
glossary, capturé dans `Some(project_id) => { resolved_project_id = Some(...) }`.
Permet de réutiliser le pid pour les stats sans re-requête, cohérent avec le plan.

**`log` non transitif utilisable** : bien que `log` soit probablement une dépendance
transitive, l'ajouter explicitement est la bonne pratique Rust — les dépendances
transitives ne sont pas garanties de rester disponibles.

**`std::fs::write` (sync) dans async context** : manifest < 1 KB, pas de loop,
acceptable. Même justification que dans `report.rs`. Alternative async non nécessaire.

## Problèmes rencontrés

- **`chrono_free_iso8601()` inexistante** : erreur lors du Step 3 — utilisé accidentellement
  un nom de fonction inventé au lieu du pattern `update_stats` qui appelle `now_iso8601()`
  en interne. Corrigé en remplaçant par `manifest::update_stats(&path, mf.stats.clone())`.

- **`log` crate absent** : `log::warn!` ne compilait pas — `log` non déclaré dans
  `Cargo.toml`. Ajout immédiat de `log = "0.4"` dans `[dependencies]`.

- **Formatter Prettier** : hook PostToolUse reformate `project.rs` après chaque Edit.
  Lecture systématique (Read) avant chaque Edit suivant pour éviter les conflits.

## Résultats tests

```
cargo test : 162 passed, 0 failed (dont 4 nouveaux : manifest.rs)
cargo clippy -- -D warnings : 0 warnings
cargo fmt --check : exit 0
pnpm typecheck : 0 erreurs
```

## Tâches ROADMAP cochées

- [x] Manifest `.hoshi2star.json` — écriture à l'ouverture, réouverture sans ré-extraction, stats auto-mises à jour

## Prochaine session

F4 (priorité absolue) :
- Wolf RPG v1/v2 — `engines/wolf/extractor.rs`
- Début recommandé : lire `docs/engines.md` et ADR existants pour contexte Wolf

---
*Généré par Claude Code — Hoshi2Star*
