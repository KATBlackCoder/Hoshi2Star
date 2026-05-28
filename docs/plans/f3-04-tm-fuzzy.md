# Plan F3-04 — TM Fuzzy Matching + Export TMX

## Objectif

Ajouter le fuzzy matching à la Translation Memory existante : algorithme Levenshtein
normalisé (seuil 80 % configurable), suggestions avec score de similarité dans le TMPanel,
et export au format TMX standard compatible OmegaT/Trados/memoQ.

## Statut : [x] Complété — 2026-05-28

## Prérequis

- F3-03 complet (Glossaire) — `[x]` Fait
- Hotfix placeholder-validation-robustness — `[x]` Fait
- `tm.rs` : `lookup_exact()` + `TmEntry` struct existants ✅
- `get_tm_suggestions` command : retourne `Vec<TmEntry>` exact match uniquement ✅
- `TMPanel.tsx` : affiche `TmEntry` avec `ConfidenceBadge` ✅
- `tauri-plugin-dialog` déjà installé (utilisé par import projet F1) ✅

## Estimation

7 steps · ~45–65 min total

## Items ROADMAP concernés

```
F3 — Core Layer — TM v2 (fuzzy matching) :
  [x] Levenshtein distance normalisée sur les segments (seuil 80 % configurable)
  [x] src-tauri/src/core/tm.rs — lookup_fuzzy(source_text, threshold) → Vec<TmSuggestion>
  [x] Export TM au format TMX standard (compatibilité OmegaT/Trados)

F3 — CAT UI — F3 :
  [x] TM sidebar avec fuzzy suggestions (% match affiché)
```

---

## Steps

---

### Step 1 — Implémenter Levenshtein manuellement dans tm.rs

**Objectif :** Ajouter la fonction `levenshtein(a: &str, b: &str) -> usize` et
`similarity_score(a: &str, b: &str) -> f32` dans `tm.rs`. Zéro dépendance externe —
l'algorithme tient en ~25 lignes.

**Fichiers touchés :**
- `src-tauri/src/core/tm.rs` ← ajouter les deux fonctions

**Dépend de :** *(aucun — code autonome)*

**Pourquoi pas `strsim`** : crate légère (0 dep), mais l'implémentation manuelle est
triviale (~25 lignes), évite toute dépendance externe, et donne un contrôle total sur
la normalisation Unicode (trim + to_lowercase, cohérent avec `hash_source`).

Tâches :
- [x] Implémenter `fn levenshtein(a: &str, b: &str) -> usize` :
  - Algorithme Wagner-Fischer (2 rangées, O(n×m) temps, O(min(n,m)) espace)
  - Opérer sur `char`s (pas bytes) pour corriger le comptage Unicode japonais
  - Cas limites : si `a` ou `b` est vide → retourner `len` de l'autre
- [x] Implémenter `pub fn similarity_score(a: &str, b: &str) -> f32` :
  ```rust
  let dist = levenshtein(a, b);
  let max_len = a.chars().count().max(b.chars().count());
  if max_len == 0 { return 1.0; }
  1.0 - (dist as f32 / max_len as f32)
  ```
  - Normalise entre 0.0 (totalement différent) et 1.0 (identique)
  - `pub` : sera réutilisée dans `lookup_fuzzy`
- [x] Tests unitaires dans `#[cfg(test)] mod tests` :
  - `test_levenshtein_known_values` : `levenshtein("kitten", "sitting") == 3`, `levenshtein("", "abc") == 3`, `levenshtein("abc", "abc") == 0`
  - `test_similarity_identical` : `similarity_score("こんにちは", "こんにちは") == 1.0`
  - `test_similarity_close` : `similarity_score("こんにちは", "こんにちわ") >= 0.80`
  - `test_similarity_empty` : `similarity_score("", "") == 1.0`

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml core::tm::tests::test_levenshtein
cargo test --manifest-path src-tauri/Cargo.toml core::tm::tests::test_similarity
```
Résultat attendu : 5 tests passent (green), `cargo clippy -D warnings` vert

Commit message : `feat(core): add Levenshtein + similarity_score to tm.rs — no external dep`

---

### Step 2 — Struct TmSuggestion + type TS

**Objectif :** Ajouter la struct `TmSuggestion` côté Rust et son miroir `TmSuggestion` dans
`types.ts`.

**Fichiers touchés :**
- `src-tauri/src/core/tm.rs` ← ajouter `TmSuggestion` struct
- `src/lib/types.ts` ← ajouter interface `TmSuggestion`

**Dépend de :** *(aucun — struct indépendante)*

Tâches :
- [x] Dans `tm.rs`, après la définition de `TmEntry`, ajouter :
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct TmSuggestion {
      pub entry: TmEntry,
      pub score: f32,
      pub match_type: String, // "exact" | "fuzzy"
  }
  ```
- [x] Dans `src/lib/types.ts`, ajouter l'interface TS :
  ```ts
  export interface TmSuggestion {
    entry: TmEntry
    score: number
    matchType: 'exact' | 'fuzzy'
  }
  ```
  — Placer juste après l'interface `TmEntry` existante (ligne ~61)

Test de validation :
```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
pnpm typecheck
```
Résultat attendu : compile clean, 0 erreur TS

Commit message : `feat(core): add TmSuggestion struct (Rust + TS)`

---

### Step 3 — ⚠️ Implémenter lookup_fuzzy()

**Objectif :** Fonction qui charge toutes les entrées TM pour un `lang_pair`, calcule le
score Levenshtein pour chacune, et retourne les meilleures suggestions triées par score
décroissant.

**Fichiers touchés :**
- `src-tauri/src/core/tm.rs` ← ajouter `lookup_fuzzy`

**Dépend de :** Step 1, Step 2

**Pourquoi ⚠️** : scan complet en mémoire — acceptable jusqu'à ~5k entrées TM.
Au-delà, le temps de réponse peut dépasser 50 ms. Documenter cette limite dans le code,
prévoir un index trigram en F5 si nécessaire.

Tâches :
- [x] Signature :
  ```rust
  pub async fn lookup_fuzzy(
      source_text: &str,
      lang_pair: &str,
      threshold: f32,  // 0.80 par défaut
      limit: usize,    // 10 par défaut
      db: &SqlitePool,
  ) -> Result<Vec<TmSuggestion>, sqlx::Error>
  ```
- [x] Logique :
  1. Charger toutes les entrées TM pour `lang_pair` :
     ```sql
     SELECT id, source_hash, source_text, target_text, engine,
            lang_pair, confidence, created_at
     FROM tm_entries WHERE lang_pair = ?
     ```
  2. Pour chaque entrée, calculer `score = similarity_score(source_text_normalised, entry.source_text_normalised)`
     — normaliser avec `text.trim().to_lowercase()` avant comparaison (cohérence avec `hash_source`)
  3. Filtre `score >= threshold` → `match_type` = si `score == 1.0` alors `"exact"` sinon `"fuzzy"`
  4. Trier par score décroissant
  5. Prendre les `limit` premiers
- [x] Commentaire de performance au-dessus de la fonction :
  ```rust
  // Scans all TM entries in memory. Acceptable for ~5k entries.
  // For larger TMs, consider a trigram index (backlog F5).
  ```
- [x] Tests unitaires (avec une DB in-memory via `test_db()` existante) :
  - `test_fuzzy_exact_match_returns_score_1` : insérer "こんにちは"/"Hello" → query "こんにちは" → score 1.0, match_type "exact"
  - `test_fuzzy_similar_returns_high_score` : insérer "こんにちは"/"Hello" → query "こんにちわ" → score >= 0.80
  - `test_fuzzy_dissimilar_filtered_out` : insérer "ABC"/"Hello" → query "XYZ" → résultat vide (score < 0.80)
  - `test_fuzzy_threshold_respected` : insérer 3 entrées avec scores variés → seules celles >= threshold retournées
  - `test_fuzzy_sorted_by_score_descending` : vérifier l'ordre de la slice retournée
  - `test_fuzzy_empty_tm_returns_empty` : DB vide → `Ok(vec![])`

> **Note déduplication** : `lookup_fuzzy` peut retourner l'exact match en tête (score 1.0)
> en plus des fuzzy matches. Le frontend gère la distinction via `match_type`.
> Pas de déduplication nécessaire — comportement souhaité.

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml core::tm::tests::test_fuzzy
```
Résultat attendu : 6 tests passent, `cargo clippy -D warnings` vert

Commit message : `feat(core): implement lookup_fuzzy — Levenshtein scan, threshold 0.80`

---

### Step 4 — Mettre à jour get_tm_suggestions command

**Objectif :** Remplacer l'appel à `lookup_exact` par `lookup_fuzzy` dans
`commands/project.rs` et mettre à jour le type de retour.

**Fichiers touchés :**
- `src-tauri/src/commands/project.rs` ← modifier `get_tm_suggestions`

**Dépend de :** Step 2, Step 3

Tâches :
- [x] Modifier `get_tm_suggestions` (actuellement lignes ~582–596) :
  ```rust
  #[tauri::command]
  pub async fn get_tm_suggestions(
      source_text: String,
      lang_pair: String,
      state: tauri::State<'_, AppState>,
  ) -> Result<Vec<tm::TmSuggestion>, String> {
      tm::lookup_fuzzy(&source_text, &lang_pair, 0.80, 5, &state.db)
          .await
          .map_err(|e| e.to_string())
  }
  ```
  - `threshold = 0.80` (80 %) — valeur par défaut
  - `limit = 5` — max 5 suggestions dans le panel
  - Les exact matches (score 1.0) apparaissent naturellement en tête après le tri
- [x] Supprimer la construction du hash SHA-256 (plus besoin dans cette command)
- [x] Vérifier que la signature est cohérente avec `lib.rs` (generate_handler — pas de
  changement d'enregistrement nécessaire, même nom de command)

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 0 erreur compilation, clippy vert

Commit message : `feat(commands): get_tm_suggestions — switch to lookup_fuzzy (threshold 0.80, limit 5)`

---

### Step 5 — Mettre à jour TMPanel.tsx

**Objectif :** Adapter l'affichage pour `TmSuggestion` avec badge score coloré et label
"Exact" / "Fuzzy ~XX%".

**Fichiers touchés :**
- `src/components/editor/TMPanel.tsx` ← adapter pour `TmSuggestion`
- `src/locales/en.json` ← ajouter clés `exact`, `fuzzy`
- `src/locales/fr.json` ← idem en français

**Dépend de :** Step 2 (type TS `TmSuggestion`)

Tâches :
- [x] Changer le type de `useQuery` : `TmEntry[]` → `TmSuggestion[]`
  - L'`invoke` retourne maintenant `TmSuggestion[]`, les données sont dans `suggestion.entry`
- [x] Remplacer `ConfidenceBadge` par un nouveau `MatchBadge` :
  ```
  score 1.0                → vert    "Exact"
  score >= 0.9 && < 1.0   → vert clair "~90%"
  score >= 0.8 && < 0.9   → jaune   "~80%"
  ```
  - Label : utiliser `t("tmPanel.exact")` si `matchType === 'exact'`,
    sinon `t("tmPanel.fuzzy", { percent: Math.round(score * 100) })`
  - Les couleurs restent cohérentes avec l'existant (`ConfidenceBadge` actuel)
- [x] Mettre à jour le rendu des cards :
  - `entry.id` → `suggestion.entry.id` (key)
  - `entry.targetText` → `suggestion.entry.targetText`
  - `entry.sourceText` → `suggestion.entry.sourceText`
  - `entry.confidence` remplacé par `suggestion.score`
- [x] Ajouter les clés i18n dans `tmPanel` section :
  ```json
  "exact": "Exact",
  "fuzzy": "~{{percent}}%"
  ```
  Versions en et fr.
- [x] Supprimer `ConfidenceBadge` (n'est plus utilisé)

Test de validation :
```bash
pnpm typecheck
pnpm lint
```
Résultat attendu : 0 erreur TS, 0 erreur lint

Commit message : `feat(ui): TMPanel — display TmSuggestion with Exact/Fuzzy badge`

---

### Step 6 — Command export_tm + Cargo.toml rien à ajouter

**Objectif :** Nouvelle command Rust `export_tm` qui génère un fichier TMX depuis la DB
et l'écrit sur disque.

**Fichiers touchés :**
- `src-tauri/src/core/tm.rs` ← ajouter `generate_tmx(entries, lang_pair)`
- `src-tauri/src/commands/project.rs` ← ajouter `export_tm`
- `src-tauri/src/lib.rs` ← enregistrer `export_tm` dans `generate_handler!`

**Dépend de :** Step 1 (TmEntry déjà défini)

**Pourquoi pas de crate XML** : le format TMX 1.4 est suffisamment simple pour être
généré avec `std::fmt::Write` en ~30 lignes. 0 dépendance supplémentaire.

Tâches :
- [x] Dans `tm.rs`, ajouter `pub fn generate_tmx(entries: &[TmEntry], src_lang: &str) -> String` :
  - Construire le XML TMX 1.4 manuellement via `use std::fmt::Write` :
    ```xml
    <?xml version="1.0" encoding="UTF-8"?>
    <tmx version="1.4">
      <header creationtool="Hoshi2Star" creationtoolversion="0.3.0"
              datatype="plaintext" segtype="sentence"
              adminlang="en-US" srclang="{src_lang}" o-tmf="Hoshi2Star"/>
      <body>
        {tu elements}
      </body>
    </tmx>
    ```
  - Pour chaque `TmEntry` : générer un `<tu>` avec 2 `<tuv>` (src_lang + target)
  - Extraire `target_lang` du champ `lang_pair` (ex: `"ja-en"` → `"en"`)
  - Échapper les caractères XML : `&` → `&amp;`, `<` → `&lt;`, `>` → `&gt;`
    (pas de crate, fonction helper privée `xml_escape(s: &str) -> String`)
  - Test unitaire `test_generate_tmx_structure` : vérifier que la sortie contient
    `<tmx`, `<header`, `<body`, `<tu`, les valeurs source et target

- [x] Dans `commands/project.rs`, ajouter :
  ```rust
  #[tauri::command]
  pub async fn export_tm(
      lang_pair: String,
      output_path: String,
      state: tauri::State<'_, AppState>,
  ) -> Result<(), String>
  ```
  - Requête SQL : toutes les entrées pour `lang_pair` (pas de filtre projet — TM globale)
  - Appeler `tm::generate_tmx(&entries, &src_lang_from_lang_pair)`
  - Écrire dans `output_path` avec `tokio::fs::write`
  - `project_id: Option<String>` reporté en backlog (export filtré par projet — cas d'usage futur)
- [x] Enregistrer dans `generate_handler![..., export_tm]` dans `lib.rs`

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml core::tm::tests::test_generate_tmx
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : test TMX vert, clippy vert

Commit message : `feat(core): add generate_tmx + export_tm command — TMX 1.4, no XML dep`

---

### Step 7 — Bouton Export TMX dans TMPanel.tsx

**Objectif :** Ajouter un bouton "Exporter TM" dans le header du TMPanel qui déclenche
un dialog de sauvegarde et appelle `export_tm`.

**Fichiers touchés :**
- `src/components/editor/TMPanel.tsx` ← ajouter le bouton export
- `src/locales/en.json` ← clé `export`, `exporting`, `exportSuccess`, `exportError`
- `src/locales/fr.json` ← idem

**Dépend de :** Step 5, Step 6

Tâches :
- [x] Dans le header du `TMPanel`, ajouter un `Button` shadcn (variant `ghost`, size `xs`)
  avec icône `Download` (lucide-react) et tooltip `t("tmPanel.export")` :
  ```tsx
  <Button variant="ghost" size="icon" className="h-5 w-5 ml-auto"
    onClick={handleExport} disabled={isExporting} title={t("tmPanel.export")}>
    <Download className="h-3 w-3" />
  </Button>
  ```
- [x] `handleExport` :
  ```ts
  const path = await save({ filters: [{ name: 'TMX', extensions: ['tmx'] }] })
  if (!path) return
  setIsExporting(true)
  await invoke('export_tm', { langPair: 'ja-en', outputPath: path })
    .then(() => toast.success(t("tmPanel.exportSuccess")))
    .catch((e) => toast.error(t("tmPanel.exportError", { error: String(e) })))
    .finally(() => setIsExporting(false))
  ```
  - `save` importé de `@tauri-apps/plugin-dialog` (déjà installé)
  - `toast` de `sonner` (déjà utilisé dans le projet)
  - `isExporting: boolean` dans un `useState` local
- [x] Clés i18n à ajouter dans `tmPanel` :
  ```json
  "export": "Export TM (.tmx)",
  "exporting": "Exporting...",
  "exportSuccess": "TM exported successfully",
  "exportError": "Export failed: {{error}}"
  ```
  Versions en et fr.
- [x] Vérifier que `capabilities/default.json` inclut la permission `dialog:default`
  (déjà nécessaire pour l'import projet F1 — ne devrait pas manquer)

Test de validation :
```bash
pnpm typecheck
pnpm lint
```
Résultat attendu : 0 erreur TS, 0 erreur lint

Commit message : `feat(ui): TMPanel — add Export TM (.tmx) button with file dialog`

---

## Tests obligatoires avant push GitHub

```bash
# Rust — unitaires + intégration
cargo test --manifest-path src-tauri/Cargo.toml
# Résultat attendu : tous les tests passent (inclut les 11 nouveaux tm.rs)

# Rust — linting qualité
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
# Résultat attendu : 0 warning, 0 erreur

# Rust — formatage
cargo fmt --manifest-path src-tauri/Cargo.toml
# Résultat attendu : aucun fichier modifié

# TypeScript — vérification types
pnpm typecheck
# Résultat attendu : 0 erreur

# TypeScript — lint
pnpm lint
# Résultat attendu : 0 erreur ESLint
```

## Commandes git de push final

```bash
# Depuis la branche feat/f3-04-tm-fuzzy (pas plan/)
git checkout main
git merge --no-ff feat/f3-04-tm-fuzzy -m "feat(f3-04): TM fuzzy matching (Levenshtein 80%) + export TMX"
git push origin main
git branch -d feat/f3-04-tm-fuzzy
git branch -d plan/f3-04-tm-fuzzy
```

## Mise à jour après complétion

Fichiers à mettre à jour une fois tous les steps complétés :

- `ROADMAP.md` : cocher les 3 items F3 Core Layer TM v2 + l'item CAT UI "TM sidebar fuzzy"
- `CHANGELOG.md` : section `[Unreleased]` → entrée `Added` TM fuzzy + export TMX
- `docs/journal/` : nouvelle entrée `YYYY-MM-DD-f3-04-tm-fuzzy.md`
