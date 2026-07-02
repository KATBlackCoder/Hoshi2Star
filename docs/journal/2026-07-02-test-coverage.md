# Journal — 2026-07-02 — Phase 7 remédiation : couverture de tests front + intégration

**Phase** : F4
**Durée estimée** : 2h
**Statut** : ✅ Complété

---

## Ce qui a été fait

Phase 7 du plan de remédiation (audit 2026-07-01) — mettre en place une suite de
tests automatisés (aucune n'existait ; CLAUDE.md documentait pourtant `pnpm test`)
et l'ajouter au gate de vérification. Trois volets front + un volet intégration Rust.

- **Volet A — Infra Vitest** : `vitest.config.ts` (jsdom, alias `@`, séparé de
  `vite.config.ts` pour ne pas polluer la conf Tauri) ; `src/test/setup.ts`
  (jest-dom, locale `fr` déterministe, stub `__TAURI_EVENT_PLUGIN_INTERNALS__`
  en assignation *writable*, `afterEach` async qui flush les microtâches
  d'`unlisten` **avant** `clearMocks()`). Scripts `pnpm test` / `pnpm test:watch`.
- **Volet B — Stores Zustand** : `editor`, `llm`, `settings`, `project` (14 tests) —
  le thunk `openProject` (mockIPC) vérifie l'ajout/activation, l'armement de
  `pendingGlossaryExtract` seulement si `!wasRestored`, la propagation d'erreur
  sans corruption du store, et `loadAllProjects` sans doublons.
- **Volet C — Composants/hooks (verrouillent Phases 3/5)** : `FileTree`
  (compteurs ✓/⚠, bouton inject caché si review en attente), `ProjectList`
  (`allSettled` — la barre de p2 survit à l'échec de p1), `SegmentGrid`
  (pagination successive, sélection **par id** via `getRowId`, toast d'erreur de
  save), `useAppHandlers` (gate d'export). La sélection `getRowId` a été prouvée
  rouge par revert avant de figer le test.
- **Volet D — Intégration Rust** : `src-tauri/tests/e2e_project_flow.rs`.
  `mock_builder().manage(AppState).build(mock_context(noop_assets()))` porte
  l'état ; les `pub async fn` (`open_project`/`update_segment`/`export_project`)
  sont **appelées directement** avec un `State` via `Manager::state()` — on teste
  la couche logique (extraction → DB → réinjection → zip), pas la sérialisation
  IPC. Deux scénarios bout-en-bout sur fixtures réelles (copiées en tempdir) :
  MV (`性処理係のある学校`, `www/data` seul) et **Wolf CommonEvent** (Honoka,
  `Data/BasicData`+`MapData` + stub `Game.ini`) — ce dernier **verrouille la
  réinjection des Common Events de la Phase 1** au niveau commande.
- **Gate étendu** : `pnpm test` ajouté à CLAUDE.md ; section « Tests » (pyramide)
  ajoutée à `docs/architecture.md`.

## Vérification

- **Rouge prouvé (Volet C)** : revert du `getRowId` de `SegmentGrid` → le test de
  sélection échoue (`["0","1"]` au lieu de `["s1","s2"]`) ; vert après restauration.
- **Marqueur ASCII** dans le scénario d'intégration : identique en UTF-8 (JSON
  MV) et Shift-JIS (`.dat` Wolf v2), donc localisable en octets bruts dans le ZIP
  exporté sans dépendre d'une fonction `pub(crate)` du parser.
- Gate complet : `pnpm typecheck` ✅ · `pnpm test` = **23 pass** ·
  `cargo clippy -- -D warnings` (gate mandaté, sans `--all-targets`) ✅ ·
  `cargo test` = **367 unitaires + 2 intégration** pass / 0 fail.

## Fichiers créés

- `vitest.config.ts`, `src/test/setup.ts`
- `src/stores/{editor,llm,settings,project}.test.ts`
- `src/components/editor/{FileTree,ProjectList,SegmentGrid}.test.tsx`
- `src/hooks/useAppHandlers.test.ts`
- `src-tauri/tests/e2e_project_flow.rs`
- `docs/journal/2026-07-02-test-coverage.md` — cette entrée

## Fichiers modifiés

- `package.json` — scripts `test` / `test:watch`
- `src-tauri/Cargo.toml` — dev-deps `tauri` (feature `test`) + `zip`
- `CLAUDE.md` — `pnpm test` dans le gate de vérification
- `docs/architecture.md` — section « Tests »
- `CHANGELOG.md`, `tasks/todo.md`

## Décisions prises

- **Appel direct des commandes** plutôt que `tauri::test::get_ipc_response` :
  `#[tauri::command]` laisse la `pub async fn` d'origine appelable ; on évite la
  construction fragile d'`InvokeRequest`/`INVOKE_KEY`/mapping camelCase et on teste
  la vraie logique métier plutôt que la plomberie IPC de Tauri.
- **`mock_context(noop_assets())`** au lieu de `generate_context!` : évite la
  dépendance à `dist/` (gitignoré → indisponible sur un checkout frais / CI).
- **Isolation par `tempfile::tempdir`** : `.hoshi2star.json` et `hoshi2star.zip`
  sont écrits *dans* la racine de jeu = le tempdir → cleanup gratuit au `Drop`,
  aucune pollution des fixtures.
- **`vitest.config.ts` séparé** de `vite.config.ts` — ne pas mêler la conf de
  test à la conf de build Tauri.

## Problèmes rencontrés

- Wolf non détecté au premier run : `detect_engine` exige un lanceur
  (`Game.exe`/`Game.ini`) à la racine → ajout d'un `Game.ini` stub dans la copie
  temporaire.
- Teardown Vitest : `unregisterListener` supprimé par `clearMocks()` avant la
  microtâche d'`unlisten` → `afterEach` rendu async avec un `setTimeout(0)`.

## Tâches ROADMAP cochées

- [ ] (aucune — remédiation audit)

## Backlog identifié (hors scope)

- 8 lints `cargo clippy --all-targets` préexistants dans le code de test
  `#[cfg(test)]` (`non_snake_case` sur `test_wolf_sysS_before_sys`,
  `if_same_then_else` dans `v3_format/map.rs`) — non couverts par le gate mandaté
  (sans `--all-targets`).

## Prochaine session

- Phase 6 — refactos DRY, puis Phase 8 — dispatch moteur (OCP), puis Phase 9 —
  perf. La suite de tests couvre désormais les seams touchés par ces refactos.

---
*Généré par Claude Code — Hoshi2Star*
