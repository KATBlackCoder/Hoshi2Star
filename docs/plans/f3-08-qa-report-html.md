# Plan F3-08 — Rapport QA HTML exportable + Filtre QA dans SegmentGrid

## Objectif

**Feature A** : Exporter un rapport QA autonome en HTML (inline CSS + JS, 0 dépendance)
listant tous les segments traduits avec erreurs, statistiques globales et filtres interactifs.

**Feature B** : Ajouter un filtre QA dans la toolbar de SegmentGrid pour afficher
uniquement les segments avec erreurs, erreurs critiques, non traduits, ou à réviser.

## Statut : [x] Complété — 2026-05-28

## Prérequis

- F3-04 complet (TM fuzzy + export TMX) — `[x]` Fait
- `qa::check()` disponible dans `src-tauri/src/core/qa.rs` ✅
- `get_qa_report` command existante dans `commands/project.rs` ✅
- `QAPanel.tsx` avec header et bouton export pattern établi (via TMPanel) ✅
- `tauri-plugin-dialog` installé (save() pour export) ✅
- `xml_escape()` privée dans `tm.rs` → dupliquer comme `fn html_escape()` dans `report.rs` ✅

## Estimation

5 steps · ~70–90 min total
- Feature A : Steps 1–4 (~55–70 min)
- Feature B : Step 5 (~15–20 min, indépendant)

## Items ROADMAP concernés

```
F3 — CAT UI — F3 :
  [ ] Rapport QA exportable HTML
```

---

## Architecture — décisions prises avant de coder

### xml_escape

`fn xml_escape()` dans `tm.rs` est **privée**. Pour `report.rs`, dupliquer comme
`fn html_escape(s: &str) -> String` — même 4 remplacements (`&`, `<`, `>`, `"`).
Couplage évité, `report.rs` reste auto-suffisant. Pas de refactor de `tm.rs`.

### Placement de report.rs

Nouveau fichier `src-tauri/src/core/report.rs` + `pub mod report;` dans `core/mod.rs`.
Cohérent avec la couche Core (tm.rs, glossary.rs, qa.rs).

### Recalcul des erreurs à l'export

Les erreurs détaillées ne sont pas stockées en DB (seulement `qa_score`).
→ Recalculer `qa::check()` au moment de l'export avec `glossary_terms: &[]`.
→ Rapport indicatif (sans glossaire), toujours frais, 0 migration.

### Filtre Feature B

Filtrage côté client sur les segments déjà en mémoire. Introduire
`filteredSegments = useMemo(filter, [segments, qaFilter])` et passer
`filteredSegments` à `useReactTable({ data: filteredSegments })`.
Le virtualizer utilise déjà `segments.length` — à corriger en `rows.length` (bug latent).

---

## Steps

---

### Step 1 — Struct QaSegmentDetail + collect_qa_details()

**Objectif :** Créer `src-tauri/src/core/report.rs` avec la struct de données
et la requête DB qui collecte les segments traduits avec leurs erreurs QA recalculées.

**Fichiers touchés :**
- `src-tauri/src/core/report.rs` ← nouveau fichier
- `src-tauri/src/core/mod.rs` ← ajouter `pub mod report;`

**Dépend de :** *(aucun — nouveau module)*

Tâches :
- [ ] Créer `report.rs` avec :
  ```rust
  use crate::core::qa;
  use serde::{Deserialize, Serialize};
  use sqlx::SqlitePool;

  #[derive(Debug, Clone, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct QaSegmentDetail {
      pub segment_id: String,
      pub file_name: String,
      pub segment_number: u32,
      pub source_text: String,
      pub target_text: String,
      pub qa_score: u8,
      pub errors: Vec<qa::QaError>,
  }
  ```
- [ ] Implémenter `pub async fn collect_qa_details(pool: &SqlitePool, project_id: &str) -> Result<Vec<QaSegmentDetail>, sqlx::Error>` :
  - Requête SQL : JOIN segments + source_files pour récupérer `file_name` et `segment_number` (via `json_key` comme numéro de référence ou index de séquence)
    ```sql
    SELECT s.id, sf.file_name,
           ROW_NUMBER() OVER (PARTITION BY s.source_file_id ORDER BY s.rowid) AS seg_num,
           s.source_text, s.target_text
    FROM segments s
    JOIN source_files sf ON s.source_file_id = sf.id
    WHERE sf.project_id = ?
      AND s.status IN ('translated', 'reviewed', 'needs_review')
      AND s.target_text != ''
    ORDER BY sf.file_name, s.rowid
    ```
  - Pour chaque ligne : appeler `qa::check(&row.source_text, &row.target_text, &[])`
  - Garder uniquement les segments où `result.score < 100`
  - Construire et retourner `Vec<QaSegmentDetail>`
- [ ] Ajouter `pub mod report;` dans `core/mod.rs`

**Note :** `segment_number` est le numéro 1-based dans le fichier (calculé via `ROW_NUMBER()` SQLite).
Si la version SQLite ne supporte pas `ROW_NUMBER()`, utiliser le `rowid` de SQLite
comme proxy de tri, et calculer le numéro côté Rust avec `enumerate()`.

Test de validation :
```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml core::report
```
Résultat attendu : compile clean, module accessible

Commit message : `feat(core): add report.rs — QaSegmentDetail + collect_qa_details`

---

### Step 2 — ⚠️ generate_qa_html() — générateur HTML autonome

**Objectif :** Générer un fichier HTML complet, autonome (0 CDN, 0 JS externe),
avec tableau filtrable des erreurs QA, statistiques globales et couleurs par seuil.

**Fichiers touchés :**
- `src-tauri/src/core/report.rs` ← ajouter `generate_qa_html` + `html_escape` + tests

**Dépend de :** Step 1

**Pourquoi ⚠️** : HTML complexe à générer manuellement, CSS inline, JS inline.
Risque de fuite d'apostrophes, guillemets non échappés. Tester dans Firefox sans serveur.

Tâches :
- [ ] Ajouter `fn html_escape(s: &str) -> String` (privée) :
  ```rust
  fn html_escape(s: &str) -> String {
      s.replace('&', "&amp;")
       .replace('<', "&lt;")
       .replace('>', "&gt;")
       .replace('"', "&quot;")
  }
  ```

- [ ] Implémenter `pub fn generate_qa_html(project_title: &str, details: &[QaSegmentDetail], lang: &str) -> String` :

  Structure du HTML (tout inline, pas de fichiers externes) :
  ```
  <!DOCTYPE html>
  <html lang="{lang}">
  <head>
    <meta charset="UTF-8">
    <title>QA Report — {project_title}</title>
    <style>
      /* ... CSS inline ... */
    </style>
  </head>
  <body>
    <h1>QA Report — {project_title}</h1>
    <p>Generated: {date} · {total_with_errors} segments with errors / {total_checked} checked</p>

    <!-- Stats par type d'erreur -->
    <div class="stats">
      <span>Missing placeholder: {count}</span>
      <span>Line too long: {count}</span>
      <span>BOM detected: {count}</span>
      <span>Glossary mismatch: {count}</span>
    </div>

    <!-- Filtres JS inline -->
    <div class="filters">
      <select id="fileFilter">...</select>  <!-- par fichier source -->
      <div class="checkboxes">
        <label><input type="checkbox" class="err-filter" value="missing_placeholder" checked>
          Missing placeholder</label>
        <label><input type="checkbox" class="err-filter" value="line_too_long" checked>
          Line too long</label>
        <label><input type="checkbox" class="err-filter" value="bom_detected" checked>
          BOM detected</label>
        <label><input type="checkbox" class="err-filter" value="glossary_mismatch" checked>
          Glossary mismatch</label>
      </div>
      <select id="scoreFilter">
        <option value="all">All errors</option>
        <option value="lt90">Score &lt; 90</option>
        <option value="lt70">Score &lt; 70 (critical)</option>
      </select>
    </div>

    <!-- Tableau -->
    <table id="qaTable">
      <thead>
        <tr>
          <th>File</th><th>#</th><th>Source</th><th>Target</th>
          <th>Score</th><th>Errors</th>
        </tr>
      </thead>
      <tbody>
        {rows}
      </tbody>
    </table>

    <!-- Message si 0 erreurs -->
    <!-- ou si filtres excluent tout -->

    <script>/* filtrage JS inline */</script>
  </body>
  </html>
  ```

  Couleurs des lignes (via class CSS) :
  - `score == 100` → vert (ne devrait pas apparaître — filtré en Step 1)
  - `score >= 70` → orange
  - `score < 70` → rouge

  CSS inline (palette sombre compatible dark mode navigateur) :
  - Background `#1a1a1a`, texte `#e0e0e0`, table alternée `#222`/`#1e1e1e`
  - Bordures `#333`, header table `#2a2a2a`
  - Vert : `#22c55e`, orange : `#f59e0b`, rouge : `#ef4444`

  JS inline (filtrage sans framework) :
  ```javascript
  function applyFilters() {
    const file = document.getElementById('fileFilter').value;
    const score = document.getElementById('scoreFilter').value;
    const checked = [...document.querySelectorAll('.err-filter:checked')]
      .map(cb => cb.value);
    document.querySelectorAll('#qaTable tbody tr').forEach(row => {
      const rowFile = row.dataset.file;
      const rowScore = parseInt(row.dataset.score);
      const rowErrors = row.dataset.errors.split(',');
      const fileOk = file === 'all' || rowFile === file;
      const scoreOk = score === 'all'
        || (score === 'lt90' && rowScore < 90)
        || (score === 'lt70' && rowScore < 70);
      const errOk = rowErrors.some(e => checked.includes(e));
      row.style.display = (fileOk && scoreOk && errOk) ? '' : 'none';
    });
  }
  // Attacher sur tous les controls
  ['fileFilter','scoreFilter'].forEach(id =>
    document.getElementById(id).addEventListener('change', applyFilters));
  document.querySelectorAll('.err-filter').forEach(cb =>
    cb.addEventListener('change', applyFilters));
  ```

  Chaque `<tr>` a des attributs `data-file`, `data-score`, `data-errors` (liste CSV des types d'erreur présents).

  Labels bilingues (`lang` param) :
  - "en" : titres et labels en anglais
  - "fr" : titres et labels en français
  - Implémentation simple : `if lang == "fr" { ... } else { ... }` sur les chaînes statiques

  Si `details` est vide : générer un HTML avec message "All segments pass QA ✓"
  (pas d'erreur retournée — rapport valide avec 0 ligne dans le tableau).

- [ ] Tests unitaires :
  - `test_generate_qa_html_structure` : créer 2 `QaSegmentDetail` avec erreurs variées,
    appeler `generate_qa_html("MyGame", &details, "en")`,
    vérifier que la sortie contient `<html`, `<table`, `MyGame`, la source et la target
    d'un segment, le score sous forme de texte
  - `test_generate_qa_html_empty_details` : `details = &[]` → contient "All segments pass QA"
  - `test_html_escape_special_chars` : `html_escape("a & <b>") == "a &amp; &lt;b&gt;"`

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml core::report::tests
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : 3 tests verts, clippy vert.
Test manuel : ouvrir le fichier généré dans Firefox sans serveur → tableau visible, filtres fonctionnels.

Commit message : `feat(core): add generate_qa_html — inline HTML report, 0 external dep`

---

### Step 3 — Command export_qa_report

**Objectif :** Exposer l'export HTML via une command Tauri, brancher `collect_qa_details`
et `generate_qa_html`, écrire le fichier sur disque.

**Fichiers touchés :**
- `src-tauri/src/commands/project.rs` ← ajouter `export_qa_report`
- `src-tauri/src/lib.rs` ← importer + enregistrer dans `generate_handler!`

**Dépend de :** Step 1, Step 2

Tâches :
- [ ] Dans `project.rs`, ajouter l'import `use crate::core::report;` en haut
- [ ] Ajouter la command :
  ```rust
  #[tauri::command]
  pub async fn export_qa_report(
      project_id: String,
      output_path: String,
      lang: String,   // "en" | "fr"
      state: tauri::State<'_, AppState>,
  ) -> Result<(), String> {
      // 1. Titre du projet
      let project_title: String = sqlx::query_scalar(
          "SELECT name FROM projects WHERE id = ?",
      )
      .bind(&project_id)
      .fetch_one(&state.db)
      .await
      .map_err(|e| e.to_string())?;

      // 2. Collecter les détails QA (erreurs recalculées)
      let details = report::collect_qa_details(&state.db, &project_id)
          .await
          .map_err(|e| e.to_string())?;

      // 3. Générer le HTML
      let html = report::generate_qa_html(&project_title, &details, &lang);

      // 4. Écrire sur disque
      tokio::fs::write(&output_path, html.as_bytes())
          .await
          .map_err(|e| e.to_string())
  }
  ```
- [ ] Dans `lib.rs` :
  - Ajouter `export_qa_report` dans l'import `use commands::project::{...}`
  - Ajouter `export_qa_report` dans `generate_handler![...]`

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : compile clean, 0 warning

Commit message : `feat(commands): add export_qa_report — collects QA details, writes HTML`

---

### Step 4 — Bouton Export HTML dans QAPanel

**Objectif :** Ajouter un bouton "Export QA Report" dans le header du QAPanel,
même pattern que le bouton Export TM dans TMPanel.

**Fichiers touchés :**
- `src/components/editor/QAPanel.tsx` ← bouton + handleExport
- `src/locales/en.json` ← clés `export`, `exportSuccess`, `exportError`
- `src/locales/fr.json` ← idem en français

**Dépend de :** Step 3

Tâches :
- [ ] Dans `QAPanel.tsx` :
  - Ajouter `import { FileDown } from "lucide-react"` (pas Download — différencier de TMX)
  - Ajouter `import { save } from "@tauri-apps/plugin-dialog"`
  - Ajouter `import { toast } from "sonner"`
  - Ajouter `import { Button } from "@/components/ui/button"`
  - Ajouter `const { i18n } = useTranslation()` (déjà `const { t } = useTranslation()` — compléter)
  - Ajouter `const [isExporting, setIsExporting] = useState(false)`
  - Ajouter `handleExport` :
    ```typescript
    const handleExport = async () => {
      if (!activeProjectId) return;
      const path = await save({
        filters: [{ name: "HTML", extensions: ["html"] }],
        defaultPath: "qa-report.html",
      });
      if (!path) return;
      setIsExporting(true);
      invoke("export_qa_report", {
        projectId: activeProjectId,
        outputPath: path,
        lang: i18n.language,   // "en" ou "fr"
      })
        .then(() => toast.success(t("qaPanel.exportSuccess")))
        .catch((e) => toast.error(t("qaPanel.exportError", { error: String(e) })))
        .finally(() => setIsExporting(false));
    };
    ```
  - Dans le header `<div>`, ajouter le bouton après le badge ok/total :
    ```tsx
    {activeProjectId && (
      <Button
        variant="ghost"
        size="icon"
        className="h-5 w-5 ml-auto"
        onClick={handleExport}
        disabled={isExporting || !activeProjectId}
        title={t("qaPanel.export")}
      >
        <FileDown className="h-3 w-3" />
      </Button>
    )}
    ```

- [ ] Ajouter dans `en.json` section `qaPanel` :
  ```json
  "export": "Export QA Report",
  "exportSuccess": "QA report exported",
  "exportError": "Export failed: {{error}}"
  ```

- [ ] Ajouter dans `fr.json` section `qaPanel` :
  ```json
  "export": "Exporter le rapport QA",
  "exportSuccess": "Rapport QA exporté",
  "exportError": "Échec de l'export : {{error}}"
  ```

Test de validation :
```bash
pnpm typecheck
pnpm lint
```
Résultat attendu : 0 erreur TS, 0 erreur lint

Commit message : `feat(ui): QAPanel — add Export QA Report button with file dialog`

---

### Step 5 — Filtre QA dans la toolbar SegmentGrid

**Objectif :** Ajouter un Select shadcn dans la toolbar de SegmentGrid permettant
de filtrer les segments par score QA, statut. Filtrage côté client sur les données
déjà en mémoire, virtualizer corrigé sur `rows.length`.

**Fichiers touchés :**
- `src/components/editor/SegmentGrid.tsx` ← filtre + filteredSegments + fix virtualizer
- `src/locales/en.json` ← 5 clés dans `segmentGrid`
- `src/locales/fr.json` ← idem en français

**Dépend de :** *(aucun — indépendant de Feature A)*

Tâches :
- [ ] Vérifier que le composant `Select` shadcn est disponible dans `src/components/ui/select.tsx`.
  Si absent : `pnpm dlx shadcn@latest add select` avant de modifier SegmentGrid.

- [ ] Dans `SegmentGrid.tsx` :
  - Ajouter `import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select"`
  - Ajouter `const [qaFilter, setQaFilter] = useState<'all' | 'errors' | 'critical' | 'untranslated' | 'needs_review'>('all')`
  - Ajouter `const filteredSegments = useMemo(() => { ... }, [segments, qaFilter])` :
    ```typescript
    const filteredSegments = useMemo(() => {
      switch (qaFilter) {
        case 'errors':
          return segments.filter(s => s.qaScore !== null && s.qaScore < 100);
        case 'critical':
          return segments.filter(s => s.qaScore !== null && s.qaScore < 70);
        case 'untranslated':
          return segments.filter(s => s.status === 'untranslated');
        case 'needs_review':
          return segments.filter(s => s.status === 'needs_review');
        default:
          return segments;
      }
    }, [segments, qaFilter]);
    ```
  - Modifier `useReactTable` : `data: filteredSegments` (au lieu de `data: segments`)
  - Corriger le virtualizer (bug latent) : `count: rows.length` au lieu de `count: segments.length`
    **Attention** : `rows` doit être déclaré avant `virtualizer`. Réorganiser si nécessaire :
    ```typescript
    const table = useReactTable({ data: filteredSegments, columns, getCoreRowModel: getCoreRowModel() });
    const rows = table.getRowModel().rows;
    const virtualizer = useVirtualizer({ count: rows.length, ... });
    ```
  - Ajouter la toolbar entre le header de colonnes et le body virtuel :
    ```tsx
    {/* Toolbar filtre */}
    <div className="shrink-0 border-b px-3 py-1.5 flex items-center gap-2">
      <Select value={qaFilter} onValueChange={(v) => setQaFilter(v as typeof qaFilter)}>
        <SelectTrigger className="h-7 w-48 text-xs">
          <SelectValue />
        </SelectTrigger>
        <SelectContent>
          <SelectItem value="all">{t("segmentGrid.filterAll")}</SelectItem>
          <SelectItem value="errors">{t("segmentGrid.filterQaErrors")}</SelectItem>
          <SelectItem value="critical">{t("segmentGrid.filterQaCritical")}</SelectItem>
          <SelectItem value="untranslated">{t("segmentGrid.filterUntranslated")}</SelectItem>
          <SelectItem value="needs_review">{t("segmentGrid.filterNeedsReview")}</SelectItem>
        </SelectContent>
      </Select>
    </div>
    ```
  - Mettre à jour le footer pour afficher "X / Y segments" quand un filtre est actif :
    ```tsx
    {/* Footer */}
    <div className="shrink-0 border-t px-3 py-1.5 text-xs text-muted-foreground">
      {qaFilter === 'all'
        ? t("segmentGrid.footer", { count: segments.length.toLocaleString() })
        : t("segmentGrid.footerFiltered", {
            shown: filteredSegments.length.toLocaleString(),
            total: segments.length.toLocaleString(),
          })
      }
    </div>
    ```

- [ ] Réinitialiser `qaFilter` à `'all'` quand `activeFileId` change :
  ```typescript
  useEffect(() => {
    setQaFilter('all');
  }, [activeFileId]);
  ```

- [ ] Ajouter dans `en.json` section `segmentGrid` :
  ```json
  "filterAll": "All segments",
  "filterQaErrors": "QA: Errors only",
  "filterQaCritical": "QA: Score < 70",
  "filterUntranslated": "Untranslated",
  "filterNeedsReview": "Needs review",
  "footerFiltered": "{{shown}} / {{total}} segments"
  ```

- [ ] Ajouter dans `fr.json` section `segmentGrid` :
  ```json
  "filterAll": "Tous les segments",
  "filterQaErrors": "QA : Erreurs uniquement",
  "filterQaCritical": "QA : Score < 70",
  "filterUntranslated": "Non traduits",
  "filterNeedsReview": "À réviser",
  "footerFiltered": "{{shown}} / {{total}} segments"
  ```

Test de validation :
```bash
pnpm typecheck
pnpm lint
```
Résultat attendu : 0 erreur TS, 0 erreur lint.
Test visuel : changer le filtre → grille se met à jour, footer affiche "X / Y segments".

Commit message : `feat(ui): SegmentGrid — QA filter toolbar + fix virtualizer count`

---

## Tests obligatoires avant merge

```bash
# Rust — unitaires
cargo test --manifest-path src-tauri/Cargo.toml
# Résultat attendu : tous les tests passent (inclut les 3 nouveaux report.rs)

# Rust — linting
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings

# Rust — formatage
cargo fmt --manifest-path src-tauri/Cargo.toml

# TypeScript
pnpm typecheck
pnpm lint
```

## Commandes git de push final

```bash
git checkout main
git merge --no-ff feat/f3-08-qa-report-html -m "feat(f3-08): QA HTML report export + SegmentGrid QA filter"
git push origin main
git branch -d feat/f3-08-qa-report-html
git branch -d plan/f3-08-qa-report-html
```

## Mise à jour après complétion

- `ROADMAP.md` : cocher `[ ] Rapport QA exportable HTML` dans F3 — CAT UI
- `CHANGELOG.md` : entrées `Added` rapport QA HTML + filtre SegmentGrid
- `docs/journal/` : nouvelle entrée `YYYY-MM-DD-f3-08-qa-report-html.md`
