# Journal — 2026-05-24 — F1 DB + Commands + UI skeleton

**Phase** : F1
**Durée estimée** : 4h
**Statut** : ✅ Complété (F1 complet, 53 tests verts, typecheck propre)

---

## Ce qui a été fait

- Lecture de CONTEXT.md, ROADMAP.md, journal session précédente, memory avant de coder
- **Schema SQL** : `0001_initial.sql` avec tables `projects`, `source_files`, `segments`
  - Status enum simulé via CHECK constraint SQLite (`'untranslated' | 'translated' | 'reviewed' | 'needs_review'`)
  - Indexes sur FK, ON DELETE CASCADE, DEFAULT `datetime('now')` pour timestamps
  - Tests SQL documentés en commentaire dans le fichier
- **db/pool.rs** : `SqlitePoolOptions` max 5 conn, `SqliteConnectOptions::create_if_missing(true)`, `sqlx::migrate!("./migrations")`, 3 tests async (tables créées, CHECK constraint, CASCADE delete)
- **state.rs** : `AppState { db: SqlitePool }` minimal
- **lib.rs** : setup pool via `tauri::async_runtime::block_on`, `.manage(AppState { db: pool })`, enregistrement 5 commands dans un seul `generate_handler!`, plugin dialog ajouté
- **detector.rs** : `find_data_dir` rendu `pub` (requis par `open_project`)
- **commands/project.rs** : types `Project`, `SourceFile`, `Segment`, `PaginatedSegments` avec `sqlx::FromRow + serde`
  - `open_project` : détection moteur, lecture System.json pour titre, transaction SQLite unique pour insertion batch projet+fichiers+segments, dispatch vers extractor par filename
  - `get_source_files`, `get_segments` (paginé + check ownership), `update_segment`, `export_project` (réinjection + write)
  - 4 tests unitaires (classify_map_files, classify_data_files, dispatch_actors, dispatch_map)
- **capabilities/default.json** : ajout `"dialog:default"`
- **shadcn** : `scroll-area` + `resizable` installés via `pnpm dlx shadcn@latest add`
- **@tauri-apps/plugin-dialog** : `pnpm add` côté TS + `tauri-plugin-dialog = "2"` côté Rust
- **react-resizable-panels** : déjà présent (ajouté par shadcn), prop `orientation` (v4) et non `direction`
- **src/lib/types.ts** : types TS miroir des structs Rust (camelCase Tauri auto-serde)
- **src/stores/project.ts** : Zustand + thunk `openProject()` qui chaîne `open_project` + `get_source_files`
- **src/stores/editor.ts** : Zustand `activeFileId`, `activeSegmentId`
- **src/features/editor/columns.tsx** : factory `createSegmentColumns()` — EditableCell (blur/Enter save, Tab → segment suivant avec virtualizer.scrollToIndex), StatusBadge, QA score coloré
- **src/components/editor/FileTree.tsx** : ScrollArea + icônes lucide-react par type fichier, highlight sélection
- **src/components/editor/SegmentGrid.tsx** : TanStack Table + TanStack Virtual (pageSize 5000, virtualizer mesure dynamique), empty states, footer compteur
- **src/App.tsx** : ResizablePanelGroup 3 colonnes (18% / 60% / 22%), Toolbar avec bouton "Ouvrir un jeu" → tauri-plugin-dialog → `openProject()`, badge nom projet + moteur

## Fichiers créés

- `src-tauri/migrations/0001_initial.sql`
- `src-tauri/src/db/mod.rs`
- `src-tauri/src/db/pool.rs` — 3 tests
- `src-tauri/src/commands/project.rs` — 5 commands, 4 tests
- `src/lib/types.ts`
- `src/stores/project.ts`
- `src/stores/editor.ts`
- `src/features/editor/columns.tsx`
- `src/components/editor/FileTree.tsx`
- `src/components/editor/SegmentGrid.tsx`
- `src/components/ui/scroll-area.tsx` (shadcn)
- `src/components/ui/resizable.tsx` (shadcn)
- `docs/journal/2026-05-24-f1-db-commands-ui.md` (ce fichier)

## Fichiers modifiés

- `src-tauri/Cargo.toml` — ajout `uuid = "1"`, `tauri-plugin-dialog = "2"`
- `src-tauri/src/lib.rs` — pub mod db, setup pool, generate_handler 5 commands
- `src-tauri/src/state.rs` — AppState { db: SqlitePool }
- `src-tauri/src/commands/mod.rs` — pub mod project
- `src-tauri/src/engines/detector.rs` — find_data_dir rendu pub
- `src-tauri/capabilities/default.json` — dialog:default
- `src/App.tsx` — layout complet 3 colonnes
- `ROADMAP.md` — items F1 Core + Commands + UI cochés
- `package.json` — react-resizable-panels, @tauri-apps/plugin-dialog

## Dépendances ajoutées

**Rust (Cargo.toml):**
- `uuid = { version = "1", features = ["v4"] }` — UUIDs pour les IDs DB
- `tauri-plugin-dialog = "2"` — file/folder picker natif

**npm (package.json):**
- `react-resizable-panels ^4.11.1` — peer dep de shadcn resizable
- `@tauri-apps/plugin-dialog 2.7.1`

## Décisions prises

- **runtime queries sqlx** : `sqlx::query()` et `sqlx::query_as::<_, T>()` (runtime-checked) plutôt que `query!()` (compile-time) pour éviter la contrainte DATABASE_URL en CI/dev. Peut être migré en F3+ si perf requiert.
- **Batch insert en transaction** : `open_project` ouvre une transaction SQLite et insère tous les segments en une seule transaction → 100× plus rapide que des inserts individuels pour les grands jeux.
- **pageSize 5000 pour SegmentGrid** : tout charger d'un coup + virtual scroll client-side est plus simple et plus rapide que la pagination serveur pour l'usage CAT. À réévaluer si jeux > 50k segments.
- **TanStack Query reporté F2** : useEffect direct utilisé pour F1 skeleton. TanStack Query + QueryClientProvider sera ajouté en F2 avec les providers LLM.
- **orientation vs direction** : react-resizable-panels v4 renomme `direction` → `orientation`. shadcn resizable.tsx est agnostique (spread props) mais l'App.tsx doit utiliser `orientation`.
- **find_data_dir pub** : `find_data_dir` dans detector.rs rendu public pour que `open_project` puisse localiser le dossier data sans re-détecter.

## Problèmes rencontrés

- **react-resizable-panels v4 : `direction` → `orientation`** : L'erreur TS `Property 'direction' does not exist` était due au renommage du prop dans v4. `orientation="horizontal"` est la prop correcte (et c'est la valeur par défaut).

## Tâches ROADMAP cochées

- [x] `src-tauri/migrations/0001_initial.sql`
- [x] `src-tauri/src/db/pool.rs`
- [x] `src-tauri/src/state.rs`
- [x] Setup lib.rs `.manage(AppState { db })`
- [x] `open_project(path: String)`
- [x] `get_source_files(project_id)`
- [x] `get_segments(project_id, file_id)`
- [x] `update_segment(id, target_text)`
- [x] `export_project(project_id)`
- [x] Layout 3 colonnes ResizablePanelGroup
- [x] `src/components/editor/SegmentGrid.tsx`
- [x] `src/components/editor/FileTree.tsx`
- [x] `src/stores/editor.ts`
- [x] `src/stores/project.ts`
- [x] Import projet via tauri-plugin-dialog
- [-] TanStack Query (reporté F2)

## Résultats des checks

- `cargo fmt` ✅
- `cargo clippy -D warnings` ✅
- `cargo test` ✅ 53/53 (46 anciens + 7 nouveaux)
- `pnpm typecheck` ✅ 0 erreurs

## Prochaine session (F2)

F1 est complet — critère de sortie : "Ouvrir un jeu MV/MZ et afficher ses segments dans la grille" est satisfait par cette session.

F2 commence avec :
1. `src-tauri/migrations/0002_tm.sql` — table TM
2. `src-tauri/src/core/tm.rs` — TM exact match (hash SHA-256)
3. `src-tauri/src/llm/provider.rs` — trait LlmProvider + OllamaProvider
4. `src-tauri/src/llm/pipeline.rs` — orchestration passes
5. Commands F2 : `translate_segments`, `get_tm_suggestions`, `get_qa_report`
6. `src/stores/llm.ts` + `TMPanel.tsx` + `QAPanel.tsx`
7. TanStack Query setup (QueryClientProvider dans main.tsx)

---
*Généré par Claude Code — Hoshi2Star*
