# Plan F3-05 — Manifest .hoshi2star.json (Persistence + Réouverture)

## Objectif

**Feature A :** Écriture d'un fichier `.hoshi2star.json` à la racine du dossier jeu
lors de chaque `open_project` réussi. Stocke les métadonnées du projet (ID, titre,
moteur, stats de traduction) pour permettre une réouverture sans ré-extraction.

**Feature B :** Logique de réouverture intelligente dans `open_project` — si le manifest
existe et que le projet est en DB, retourner directement le projet existant avec
`was_restored: true` sans ré-extraire les fichiers.

## Statut : [x] Complété

## Prérequis

- F3-08 complet (QA HTML + filtre SegmentGrid) — `[x]` Fait
- `open_project` fonctionnel : transaction SQLite, extraction, retourne `Project` ✅
- `update_segment` command fonctionnelle (traduction manuelle segment par segment) ✅
- `translate_segments` command + background `tokio::spawn` fonctionnels ✅
- `uuid` crate déjà dans `Cargo.toml` ✅
- `serde_json` crate déjà dans `Cargo.toml` ✅
- `chrono` **absent** des deps — datetime générés manuellement (pattern `report.rs`) ✅

## Estimation

5 steps · ~60–80 min total

## Items ROADMAP concernés

Aucun item existant dans `ROADMAP.md` ne couvre cette feature.
Ajouter sous F3 après complétion :
```
### Persistence projet
- [x] Manifest .hoshi2star.json — écriture à l'ouverture, réouverture sans ré-extraction
```

---

## Steps

---

### Step 1 — Module core/manifest.rs

**Objectif :** Créer `src-tauri/src/core/manifest.rs` avec les structs `ManifestData` +
`ManifestStats`, les fonctions `write_manifest`, `read_manifest`, `update_stats`, et
l'implémentation `ManifestData::new(...)` (factory — encapsule `now_iso8601` + VERSION).

**Fichiers touchés :**
- `src-tauri/src/core/manifest.rs` ← nouveau fichier
- `src-tauri/src/core/mod.rs` ← ajouter `pub mod manifest;`

**Dépend de :** *(aucun — module autonome)*

**Note chemin manifest :** `.hoshi2star.json` commence par un point → caché sur Linux/macOS.
Sur Windows, les fichiers avec point ne sont pas automatiquement cachés — comportement
différent mais acceptable pour le MVP. Construire le chemin avec
`Path::new(game_path).join(".hoshi2star.json")`.

**Note permissions Tauri :** `std::fs::write` dans le backend Rust n'est PAS soumis
aux capabilities Tauri — celles-ci ne s'appliquent qu'au frontend TypeScript
(`tauri-plugin-fs`). Le backend Rust peut écrire partout où l'OS le permet.
Aucune modification de `capabilities/default.json` requise.

Tâches :
- [ ] Constante version et structs publiques :
  ```rust
  pub const VERSION: &str = env!("CARGO_PKG_VERSION");

  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct ManifestStats {
      pub file_count: u32,
      pub segment_count: u32,
      pub translated_count: u32,
      pub glossary_term_count: u32,
  }

  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct ManifestData {
      pub project_id: String,
      pub game_title: String,
      pub engine: String,
      pub game_path: String,
      pub hoshi2star_version: String,
      pub created_at: String,
      pub last_opened_at: String,
      pub stats: ManifestStats,
  }
  ```

- [ ] Factory `ManifestData::new(...)` :
  ```rust
  impl ManifestData {
      pub fn new(
          project_id: String,
          game_title: String,
          engine: String,
          game_path: String,
          stats: ManifestStats,
      ) -> Self {
          let now = now_iso8601();
          Self {
              project_id, game_title, engine, game_path,
              hoshi2star_version: VERSION.to_string(),
              created_at: now.clone(),
              last_opened_at: now,
              stats,
          }
      }
  }
  ```
  `now_iso8601()` reste **privée** — seul `ManifestData::new` et `update_stats` l'utilisent.

- [ ] Helper privé `fn now_iso8601() -> String` :
  - Utiliser `std::time::SystemTime::now().duration_since(UNIX_EPOCH)` pour les secondes
  - Convertir manuellement en `"YYYY-MM-DDTHH:MM:SSZ"` via calcul Grégorien
  - Même pattern que `unix_timestamp_to_date_str()` dans `report.rs` — étendre pour inclure l'heure
  - 0 dépendance externe (`chrono` absent, cohérent avec le reste du projet)

- [ ] `pub fn write_manifest(game_path: &str, data: &ManifestData) -> Result<(), std::io::Error>` :
  - Chemin : `Path::new(game_path).join(".hoshi2star.json")`
  - Sérialiser avec `serde_json::to_string_pretty(data)`
    → convertir erreur serde en `std::io::Error` via `std::io::Error::new(InvalidData, e)`
  - Écrire avec `std::fs::write(path, json)` (sync — fichier < 1 KB, opération légère,
    justifié même dans un contexte async)

- [ ] `pub fn read_manifest(game_path: &str) -> Result<Option<ManifestData>, std::io::Error>` :
  - Construire le chemin manifest
  - Si fichier absent (`kind() == ErrorKind::NotFound`) → `Ok(None)` (pas d'erreur)
  - Toute autre erreur I/O → propager
  - Lire avec `std::fs::read_to_string`
  - `serde_json::from_str::<ManifestData>(&content)` :
    - Succès → `Ok(Some(data))`
    - Erreur désérialisation → `log::warn!("manifest corrupt at {game_path}: {e}"); Ok(None)`
      ← manifest corrompu = cas d'utilisation normale, ne jamais bloquer l'ouverture

- [ ] `pub fn update_stats(game_path: &str, stats: ManifestStats) -> Result<(), std::io::Error>` :
  - Appeler `read_manifest(game_path)` :
    - `None` → `Ok(())` — pas de manifest à mettre à jour, ignorer silencieusement
    - `Some(mut data)` → `data.stats = stats; data.last_opened_at = now_iso8601();`
  - Appeler `write_manifest(game_path, &data)`

- [ ] Tests unitaires `#[cfg(test)] mod tests` (utiliser `tempfile::tempdir()`) :
  - `test_write_read_round_trip` : écrire `ManifestData::new(...)` dans tempdir → lire →
    vérifier `project_id` et `stats.segment_count` corrects
  - `test_read_absent_returns_none` : lire depuis un dossier sans manifest → `Ok(None)`, pas de panic
  - `test_read_corrupt_returns_none` : écrire `"invalid json"` dans `.hoshi2star.json` →
    lire → `Ok(None)`, pas de panic
  - `test_update_stats_updates_counts` : write → update_stats avec nouvelles valeurs →
    read → vérifier que `stats.translated_count` est mis à jour, `project_id` préservé

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml core::manifest::tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 4 tests passent, clippy vert.

Commit message : `feat(core): add manifest.rs — ManifestData, write/read/update_stats + 4 tests`

---

### Step 2 — Écriture manifest dans open_project (Feature A)

**Objectif :** Appeler `manifest::write_manifest` à la fin d'un `open_project` réussi,
après `tx.commit()`. Comptabiliser files + segments pendant l'extraction. Un échec
d'écriture est logué mais N'ÉCHOUE PAS la command — le manifest est toujours optionnel.

**Fichiers touchés :**
- `src-tauri/src/commands/project.rs` ← modifier `open_project`

**Dépend de :** Step 1

Tâches :
- [ ] Ajouter `use crate::core::manifest;` dans les imports

- [ ] Ajouter deux compteurs avant la boucle d'extraction (avant le `match engine`) :
  ```rust
  let mut file_count: u32 = 0;
  let mut segment_count: u32 = 0;
  ```
  Incrémenter `file_count += 1` à chaque `INSERT source_files`,
  `segment_count += 1` à chaque `INSERT segments` — dans les deux branches
  `Engine::MvMz` et `Engine::VxAce`.

- [ ] Après `tx.commit().await.map_err(...)` (ligne ~231), AVANT le SELECT final :
  ```rust
  let manifest_data = manifest::ManifestData::new(
      project_id.clone(),
      game_title.clone(),
      engine_str.to_string(),
      path.clone(),
      manifest::ManifestStats {
          file_count,
          segment_count,
          translated_count: 0,
          glossary_term_count: 0,
      },
  );
  if let Err(e) = manifest::write_manifest(&path, &manifest_data) {
      log::warn!("manifest write failed for {path}: {e}");
  }
  ```

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : tous les tests existants passent, clippy vert.
Vérification manuelle : après `open_project`, un fichier `.hoshi2star.json` doit
apparaître dans le dossier jeu avec `projectId`, `engine`, `stats` corrects.

Commit message : `feat(commands): open_project — write .hoshi2star.json after successful insert`

---

### Step 3 — ⚠️ Logique réouverture + OpenProjectResult (Feature B)

**Objectif :** Ajouter `OpenProjectResult { project: Project, was_restored: bool }` et
modifier `open_project` pour retourner ce wrapper. Insérer la logique de réouverture
AVANT la détection moteur (ligne ~120 de `project.rs`) : si manifest présent + projet
en DB → retourner le projet existant sans ré-extraction.

**Pourquoi ⚠️ :** Modification de la signature de retour de `open_project` — impact
direct sur le frontend (`types.ts`, `stores/project.ts`). Nécessite de vérifier TOUS
les call-sites de `invoke('open_project', ...)`.

**Fichiers touchés :**
- `src-tauri/src/commands/project.rs` ← struct `OpenProjectResult`, refactoring `open_project`
- `src/lib/types.ts` ← ajouter `interface OpenProjectResult`
- `src/stores/project.ts` ← adapter `invoke<OpenProjectResult>('open_project', ...)`

**Dépend de :** Step 1, Step 2

Tâches Rust :
- [ ] Ajouter dans `commands/project.rs` (après `Project` struct) :
  ```rust
  #[derive(Debug, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct OpenProjectResult {
      pub project: Project,
      pub was_restored: bool,
  }
  ```

- [ ] Changer la signature de `open_project` :
  ```rust
  pub async fn open_project(
      path: String,
      state: tauri::State<'_, AppState>,
  ) -> Result<OpenProjectResult, String>
  ```
  (`lib.rs` n'a pas besoin d'être modifié — même nom de command enregistré)

- [ ] Déclarer `preserved_project_id: Option<String> = None;` avant la détection moteur

- [ ] Insérer au début de `open_project` (AVANT `detect_engine`, ligne ~120) :
  ```
  1. manifest::read_manifest(&path)
  2. Si Ok(Some(manifest)) :
     a. SELECT COUNT(*) FROM projects WHERE id = manifest.project_id
     b. Si count > 0 (projet en DB) :
        - Mettre à jour last_opened_at : lire manifest, modifier, réécrire
          (via write_manifest — update_stats ne suffit pas pour ce champ seul)
        - Fetcher Project depuis DB par manifest.project_id
        - Retourner Ok(OpenProjectResult { project, was_restored: true })
     c. Si count = 0 (DB effacée ou nouvelle installation) :
        - preserved_project_id = Some(manifest.project_id.clone())
        - Continuer extraction normale ci-dessous
  3. Si Ok(None) ou Err → preserved_project_id reste None, continuer normalement
  ```

- [ ] Modifier la génération du `project_id` (ligne ~150) :
  ```rust
  let project_id = preserved_project_id
      .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
  ```

- [ ] Adapter le retour final (cas extraction fraîche) :
  ```rust
  Ok(OpenProjectResult { project, was_restored: false })
  ```

Tâches TypeScript :
- [ ] Dans `src/lib/types.ts`, après l'interface `Project` :
  ```ts
  export interface OpenProjectResult {
    project: Project
    wasRestored: boolean
  }
  ```

- [ ] Dans `src/stores/project.ts`, adapter l'invoke :
  ```ts
  // Avant :
  const project = await invoke<Project>('open_project', { path })
  
  // Après :
  const result = await invoke<OpenProjectResult>('open_project', { path })
  const { project, wasRestored } = result
  // stocker project dans le store ; wasRestored géré au Step 5
  ```
  Vérifier qu'il n'existe pas d'autres call-sites `invoke('open_project', ...)`.

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
pnpm typecheck
```
Résultat attendu : 0 erreur Rust, 0 erreur TS.
Vérification manuelle :
- 1er `open_project` d'un jeu → `.hoshi2star.json` créé, `wasRestored: false`
- 2e `open_project` du même jeu → aucune nouvelle row DB, `wasRestored: true`

Commit message : `feat(commands): open_project returns OpenProjectResult — smart restore from manifest`

---

### Step 4 — Mise à jour stats manifest

**Objectif :** Mettre à jour les stats du manifest à deux points clés, avec des
stratégies différentes selon la fréquence des opérations.

**Stratégie update_stats — décision :**

`update_segment` est une command manuelle (un segment à la fois, déclenchée par
l'utilisateur) → la latence d'un `std::fs::write` < 1 KB est négligeable. Appel
direct à `update_stats` après chaque sauvegarde.

`translate_segments` opère en `tokio::spawn` et met à jour 20–100+ segments via
des `sqlx::query` directes (pas via `update_segment`). Appeler `update_stats` par
segment ajouterait 20–100+ écritures fichier inutiles. Solution retenue : **un seul
appel `update_stats` à la fin du spawn**, juste avant l'émission de
`h2s://llm/completed`. Jamais de debounce (complexité non justifiée ici).

Dans les deux cas : si `update_stats` échoue → `let _ = ...;` ignorer silencieusement.

**Fichiers touchés :**
- `src-tauri/src/commands/project.rs` ← modifier `update_segment` + `translate_segments`

**Dépend de :** Step 1, Step 3

Tâches — 4A : `update_segment` :
- [ ] Après le `sqlx::query("UPDATE segments SET target_text = ?...")`, effectuer
  une requête SQL unique pour récupérer `game_path` + tous les compteurs de stats
  pour le projet auquel appartient le segment :
  ```sql
  SELECT p.game_path,
    (SELECT COUNT(*) FROM source_files sf2 WHERE sf2.project_id = p.id),
    (SELECT COUNT(*) FROM segments s2
       JOIN source_files sf2 ON s2.source_file_id = sf2.id
       WHERE sf2.project_id = p.id),
    (SELECT COUNT(*) FROM segments s2
       JOIN source_files sf2 ON s2.source_file_id = sf2.id
       WHERE sf2.project_id = p.id AND s2.status = 'translated'),
    (SELECT COUNT(*) FROM glossary_terms g
       WHERE g.project_id = p.id OR g.project_id IS NULL)
  FROM segments s
    JOIN source_files sf ON s.source_file_id = sf.id
    JOIN projects p ON sf.project_id = p.id
  WHERE s.id = ?
  ```
  Type attendu : `(String, i64, i64, i64, i64)` via `sqlx::query_as`.

- [ ] Si la requête réussit → appeler `manifest::update_stats(&game_path, stats)` :
  ```rust
  if let Ok(Some((game_path, files, segs, translated, glossary))) = stats_row {
      let _ = manifest::update_stats(&game_path, manifest::ManifestStats {
          file_count: files as u32,
          segment_count: segs as u32,
          translated_count: translated as u32,
          glossary_term_count: glossary as u32,
      });
  }
  ```

Tâches — 4B : `translate_segments` (inside `tokio::spawn`) :
- [ ] Dans le bloc `Ok(results) => { ... }`, après la boucle `for r in results`,
  AVANT l'émission de `h2s://llm/completed` :
  - Réutiliser `pid` (déjà résolu plus haut dans le spawn pour les `glossary_terms`)
  - Requête similaire à 4A mais en partant de `p.id = ?` (project_id direct)
  - Appeler `manifest::update_stats(...)` — erreur ignorée silencieusement
  - `let _ = handle.emit("h2s://llm/completed", ...)` reste le dernier appel du bloc

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : tests passent, clippy vert.
Vérification manuelle :
- Sauvegarder un segment manuellement → relire `.hoshi2star.json` →
  `translatedCount` est incrémenté
- Lancer `translate_segments` sur un fichier → après `h2s://llm/completed` →
  relire `.hoshi2star.json` → `translatedCount` reflète le total du batch

Commit message : `feat(commands): update manifest stats on update_segment + translate_segments batch`

---

### Step 5 — Frontend was_restored toast + i18n

**Objectif :** Afficher un toast sonner "Project restored" quand `wasRestored === true`,
ajouter les clés i18n dans `en.json` et `fr.json`.

**Fichiers touchés :**
- `src/stores/project.ts` ← gérer `wasRestored` après `invoke('open_project')`
- `src/locales/en.json` ← ajouter section `"project"` avec clé `"restored"`
- `src/locales/fr.json` ← idem en français

**Dépend de :** Step 3

Tâches :
- [ ] Dans `src/stores/project.ts`, après invoke et destructuring de Step 3 :
  ```ts
  if (wasRestored) {
    toast.success(t('project.restored'))
  }
  ```
  — `toast` de `sonner` (même import que dans `QAPanel.tsx` et `TMPanel.tsx`)
  — `t()` : vérifier le pattern i18n utilisé dans le store (import `i18next` ou hook)
  — Si `wasRestored === false` : comportement actuel inchangé (pas de toast spécial)

- [ ] Dans `src/locales/en.json`, ajouter la section `"project"` (créer si absente) :
  ```json
  "project": {
    "restored": "Project restored — continuing where you left off"
  }
  ```

- [ ] Dans `src/locales/fr.json` :
  ```json
  "project": {
    "restored": "Projet restauré — reprise de la traduction"
  }
  ```

Test de validation :
```bash
pnpm typecheck
```
Résultat attendu : 0 erreur TS.
Vérification manuelle : ré-ouvrir un projet déjà ouvert → toast
"Project restored — continuing where you left off" s'affiche en haut de l'écran.

Commit message : `feat(ui): open_project — toast "Project restored" when was_restored=true (i18n en+fr)`

---

## Tests obligatoires avant push GitHub

```bash
# Rust — unitaires + intégration
cargo test --manifest-path src-tauri/Cargo.toml
# Résultat attendu : tous les tests passent (inclut les 4 nouveaux manifest.rs)

# Rust — linting qualité
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
# Résultat attendu : 0 warning, 0 erreur

# Rust — formatage
cargo fmt --manifest-path src-tauri/Cargo.toml
# Résultat attendu : aucun fichier modifié

# TypeScript — vérification types
pnpm typecheck
# Résultat attendu : 0 erreur

# Note : pnpm lint échoue actuellement (manque eslint.config.js — bug pré-existant
# non lié à cette feature). Seul pnpm typecheck est obligatoire.
```

## Commandes git de push final

```bash
# Depuis la branche feat/f3-05-manifest-persistence (pas plan/)
git checkout main
git merge --no-ff feat/f3-05-manifest-persistence \
  -m "feat(f3-05): manifest .hoshi2star.json — write on open + smart restore"
git push origin main
git branch -d feat/f3-05-manifest-persistence
git branch -d plan/f3-05-manifest-persistence
```

## Mise à jour après complétion

Fichiers à mettre à jour une fois tous les steps complétés :

- `ROADMAP.md` : ajouter + cocher l'item "Manifest .hoshi2star.json" sous F3 (section nouvelle "Persistence projet")
- `CHANGELOG.md` : section `[Unreleased]` → entrée `Added` manifest + restore
- `docs/journal/` : nouvelle entrée `YYYY-MM-DD-f3-05-manifest-persistence.md`
