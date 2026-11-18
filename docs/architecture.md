# Hoshi2Star — carte du code

> Mise à jour : 2026-11-18, après la fusion du plan DD (`main` @ `e7fe775`) ; plan UWF (déchiffrement Wolf dans
> l'app, `engines/wolf/unpack.rs`, commandes `wolf_unpack.rs`, `WolfUnpackDialog`).
> Ce document est une **carte** : où vit chaque responsabilité, comment les parties s'enchaînent et où intervenir.
> Il ne liste pas les fonctions : le code et ses commentaires font foi. Les règles et conventions détaillées sont
> dans `CONTEXT.md`, les décisions dans `docs/adr/`.
>
> **Règle de mise à jour** (`AGENTS.md`) : tout lot qui crée, déplace ou supprime une frontière de module met
> cette carte à jour dans la même PR.

Documents liés : [terminologie](architecture/terminology.md) (architecture détaillée), [guide terminologie](terminology.md),
[Wolf RPG](engines/wolf-uberwolf.md) (ouvrir et déchiffrer un jeu).

---

## Vue d'ensemble

```
┌──────────────────────────────────────────────────────────────────┐
│  Interface   src/            React 19 · Zustand · TanStack Query │
│              lib/api/* ──invoke()──►        ◄── events h2s://…    │
├──────────────────────────────────────────────────────────────────┤
│  IPC         src-tauri/src/commands/   56 commandes, un seul     │
│                                        generate_handler! (lib.rs)│
├───────────────────────┬──────────────────────────────────────────┤
│  LLM   llm/           │  Métier   core/                          │
│  pipeline, provider,  │  TM, QA, rapport, terminologie,          │
│  tokenizer, contexte  │  mise en page (text_fit), originaux, .h2s │
├───────────────────────┴──────────────────────────────────────────┤
│  Moteurs   engines/   detector · mv_mz/ · wolf/ · vx_ace/ (inactif)│
├──────────────────────────────────────────────────────────────────┤
│  Données   db/ (SQLite sqlx, migrations)  ·  domain/types.rs (IPC)│
└──────────────────────────────────────────────────────────────────┘
```

L'export n'est pas une couche à part : il est orchestré par `commands/export.rs`, qui appelle les injecteurs et les
gardes d'intégrité de chaque moteur et la mise en page de `core/text_fit/`.

---

## Back : `src-tauri/src/`

### Entrée et état

| Chemin | Rôle |
|---|---|
| `lib.rs` | Entrée de l'app (ADR-005) : plugins, pool SQLite, `AppState`, l'unique `generate_handler!`. Pont MCP en debug seulement. |
| `main.rs` | Garde console Windows, appelle `run()`. |
| `state.rs` | `AppState` : base, drapeau d'annulation de traduction, service terminologique, `data_dir`, profils, cache de police, drapeaux du déchiffrement Wolf (annulation, en cours). |
| `domain/types.rs` | Types échangés avec le front (miroir TS : `src/lib/types.ts`). |
| `utils/` | `text.rs` (échappement XML/HTML), `time.rs` (dates sans dépendance). |

### `commands/` — couche IPC

Une commande valide ses arguments, charge le contexte et délègue ; elle rend `Result<T, String>`.

| Fichier | Domaine |
|---|---|
| `project.rs` | Ouvrir un jeu, fichiers, segments, édition, stats, suppression, dump de debug. |
| `translate.rs` | Traduire une sélection ou tout le projet, annuler, lister les modèles du fournisseur. |
| `qa.rs` | QA d'un segment, rapport QA du projet, suggestions TM. |
| `export.rs` | Export du jeu (zip), pré-contrôle, export de debug, rapport QA, TM/TMX, JSON de debug, préfixes de police. |
| `text_fit.rs` | Réglages, scan, aperçu et exceptions de mise en page ; aiguillage `FitEngine`. |
| `wolf_text_fit.rs` | Fenêtre de message Wolf et ajustement Wolf à l'export (`WolfExportFit`). |
| `wolf_unpack.rs` | Déchiffrer un jeu Wolf empaqueté (`unpack_wolf_game`, `cancel_wolf_unpack`) : un seul à la fois, travail dans un `spawn_blocking`, progression `h2s://wolf/unpack-progress` (au plus toutes les 100 ms), clés Pro dans `<data_dir>/unwolf/UberWolfConfig.json` (ADR-010). |
| `coverage.rs` | Couverture de texte MV/MZ (scan, statut, catégories). |
| `resync.rs` | Réaligner un projet MV/MZ après une mise à jour du jeu. |
| `search.rs` | Recherche dans tout le projet. |
| `terminology.rs` | Bibliothèque terminologique : CRUD, scan, traduction, interlocuteurs Wolf. |
| `pack.rs`, `pack_terminology.rs` | Pack d'échange `.h2s` (export, aperçu, import) et sa partie terminologie. |
| `pilot.rs` | Pilote MV/MZ sur une base jetable. |
| `app.rs` | Capacité de mise à jour automatique (ADR-007). |

### `core/` — métier, sans Tauri

| Chemin | Rôle |
|---|---|
| `tm.rs` | Mémoire de traduction globale à l'installation (ADR-003) : exact + approché, export TMX. |
| `qa.rs` | Règles QA d'un segment (enum `QaError`) et score. |
| `report.rs` | Aperçu QA, audit avant export (bloque sur erreur critique), rapport HTML. |
| `text_fit/` | **Mise en page commune** (ADR-009) : trait `TextBox` et échelle unique `fit()` (`fit.rs`), réglages et choix de l'adaptateur `engine_fit()` (`settings.rs`), polices (`font.rs`), coupure des lignes (`wrap.rs`), jetons (`token.rs`), fenêtre mesurée (`measured.rs`), scan (`scan.rs`). Seul `engine_fit()` lit le moteur. |
| `terminology/` | Analyse japonaise, scan, dépôt SQL, résolveur commun au LLM et à la QA ([détail](architecture/terminology.md)). |
| `originals.rs` | Instantané d'origine des fichiers MV/MZ (le format est dans `engines/mv_mz/originals.rs`). |
| `manifest.rs` | `.hoshi2star.json` à la racine du jeu : réouverture sans réextraction. |
| `h2s_pack.rs` | Format du pack `.h2s` (données seulement). |

### `llm/` — pipeline de traduction

| Chemin | Rôle |
|---|---|
| `pipeline.rs` | Orchestration par lots : dédoublonnage, TM, jetons, contexte, appel, restauration, QA, écriture, événements, pauses. |
| `provider.rs` | Trait `LlmProvider` et `OpenAiCompatibleProvider` (Ollama et API compatibles). |
| `tokenizer.rs` | Codes d'échappement → `⟦ph_N⟧` avant envoi, restauration après (ADR-002). |
| `split.rs`, `semantic_retry.rs` | Découpe récursive d'un lot en échec ; nouvel essai segment par segment si la QA sémantique refuse. |
| `batch.rs`, `progress.rs`, `prompts.rs` | Lots et dédoublonnage ; charges des événements `h2s://llm/*` ; gabarits (`src-tauri/prompts/`). |
| `context/` | Contexte envoyé au LLM, par moteur (`mv_mz.rs`, `wolf.rs`). |

### `engines/` — lecture et écriture par moteur

| Chemin | Rôle |
|---|---|
| `detector.rs` | Enum `Engine` (`db_str` / `from_db_str`), détection : MV/MZ puis Wolf ; VX Ace désactivé. |
| `filter.rs`, `terminology.rs` | Filtre commun « à traduire » ; adaptateurs terminologiques par moteur (MV/MZ seulement). |
| `mv_mz/` | Extracteur, injecteur, garde d'intégrité, instantanés, déchiffrement des assets, `plugins.js`, locale, resync, `coverage/`, `layout/` (adaptateur `MzTextBox` et simulation de la fenêtre). |
| `wolf/` | Fichiers libres de `Data/` seulement (ADR-008) : `sources.rs` (`WolfSources`, refuse les archives de texte ; `packed_state` repère un jeu encore empaqueté, marqueur `wolf_packed`), `unpack.rs` (déchiffrement d'un jeu empaqueté par la crate `unwolf`, **seul module qui importe unwolf**, ADR-010), `sites.rs` (sites de texte des commandes 101/102 communs à l'extracteur et à l'injecteur), extracteur, injecteur, garde d'intégrité, filtre, `dat_parser.rs`, `encoding.rs`, `v3_format/`, `layout/` (adaptateur `WolfTextBox`). `decrypt/` est du code mort (retrait prévu, plan UWF, UWF-7). |
| `vx_ace/` | Extracteur et injecteur conservés, inatteignables tant que la détection est désactivée. |

### `db/`

| Chemin | Rôle |
|---|---|
| `pool.rs` | Pool SQLite, migrations embarquées (`src-tauri/migrations/`, dernière : `0013`), réparation `translation_secs`. |
| `project_context.rs` | `ProjectContext::load` : une lecture du projet, moteur typé, fichiers sources. |

---

## Front : `src/`

| Chemin | Rôle |
|---|---|
| `main.tsx`, `App.tsx` | Entrée (i18n, `QueryClientProvider`) ; coquille et modes (`library`, `terminology`, `patch`). |
| `lib/api/` | **Seul accès à Rust** avec `features/terminology/api.ts` : un objet typé par domaine (`project`, `export`, `textFit`, `coverage`, `resync`, `qa`, `tm`, `llm`, `pack`, `app`, `wolf`). |
| `lib/` | `types.ts` (miroir des types Rust), `engine.ts` (capacités par moteur), `events.ts` (noms `h2s://`), `queryClient.ts` (clés et invalidations), `projectError.ts` (erreurs d'ouverture → clés i18n), `utils.ts` (`errorMessage`), `segmentUpdates.ts`, `constants.ts`. |
| `stores/` | Zustand : `project`, `llm`, `editor`, `search`, `settings`, `ui`, `updater`, `pilot`, et l'état des dialogues `textFit`, `coverage`, `resync`, `wolfUnpack` (ce dernier porte aussi `openGameFolder`, l'ouverture d'un dossier de jeu qui propose le déchiffrement Wolf). |
| `hooks/` | `useAppHandlers` (traduction, export), `useProjectDataSync` (réaction unique aux écritures du back), `useCoalescedSegmentUpdates`, `useTauriEvent`, `useExportToFile`. |
| `components/` | `shell/` (coquille, panneaux), `editor/` (liste des projets, grille, panneaux TM/QA/terminologie/aperçu), `settings/`, `pilot/`, `patch/`, dialogues d'export, de mise en page et de déchiffrement Wolf (`WolfUnpackDialog`), `ui/` (shadcn, ne pas éditer à la main). |
| `features/` | `terminology/` (espace terminologique autonome, TanStack Query) ; `editor/columns.tsx` (colonnes de la grille). |
| `locales/` | `fr.json` (langue par défaut) et `en.json`, mêmes clés. |

---

## Parcours

**Ouvrir un jeu.** `AppToolbar` ou `ProjectList` → `stores/wolfUnpack.openGameFolder` →
`stores/project.openProject` → `projectApi.open` → `commands/project.rs::open_project` : manifeste (réouverture
directe si connu) → sinon `detect_engine` (sur « moteur non détecté », `wolf::sources::packed_state` donne
`wolf_packed` pour un jeu Wolf empaqueté) → extraction (`engines/<moteur>/extractor`) → insertion des fichiers et
segments en une transaction → instantané MV/MZ (`core/originals.rs`) → manifeste écrit. Erreurs Wolf traduites par
`src/lib/projectError.ts`.

**Déchiffrer un jeu Wolf** (ADR-010). `wolf_packed` ou `wolf_archive_present` à l'ouverture → `WolfUnpackDialog`
(confirmation) → `wolfApi.unpackGame` → `commands/wolf_unpack.rs` → `engines/wolf/unpack.rs::unpack_game` (crate
`unwolf`) : archives de texte extraites, protection Pro 3.5 levée, jeu contrôlé lisible, archives déplacées dans
`<jeu>/Hoshi2Star-archives/` → rapport → nouvelle ouverture du même dossier.

**Traduire.** `stores/llm` → `llmApi.run('translate_*')` → `commands/translate.rs` (contexte projet, fournisseur) →
`llm/pipeline.rs` : TM → jetons → contexte et règles terminologiques → `split` → fournisseur → restauration et QA →
écriture en base → événements `h2s://llm/*` → `useProjectDataSync` et la grille se rafraîchissent.

**Contrôler (QA).** `QAPanel` / `TMPanel` (panneau de droite) → `commands/qa.rs` → `core/qa.rs` (+ résolveur
terminologique) ; rapport par `core/report.rs`.

**Exporter.** `useAppHandlers.handleExportAll` → dialogue de police → `commands/export.rs::export_project` :
audit QA (bloquant) → MV/MZ : instantané à jour → mise en page (`commands/text_fit.rs` → `core/text_fit` →
`MzTextBox`) → `mv_mz/injector` → garde d'intégrité ; Wolf : `WolfExportFit` → `wolf/injector` en mémoire
(`WolfSources`, garde d'intégrité) → zip. Une anomalie d'intégrité bloque l'export
(`ExportIntegrityDialog`). `preflight_export` vérifie avant (version et encodage Wolf).

**Autres.** Resync MV/MZ (`commands/resync.rs` → `engines/mv_mz/resync`, événement `h2s://project/resynced`) ;
couverture MV/MZ (`commands/coverage.rs` → `engines/mv_mz/coverage/`) ; packs `.h2s` (`commands/pack.rs`,
événement `h2s://project/import-done`) ; terminologie (`features/terminology/` → `commands/terminology.rs` →
`core/terminology/`).

---

## Où changer quoi

| Besoin | Où |
|---|---|
| Nouvelle commande IPC | Fonction dans `commands/<domaine>.rs` → ajout à l'unique `generate_handler!` de `lib.rs` → types dans `domain/types.rs` et `src/lib/types.ts` → fonction dans `src/lib/api/<domaine>.ts`. Jamais d'`invoke` ailleurs. |
| Nouvel événement back → front | `app.emit("h2s://<domaine>/<action>")` → nom dans `src/lib/events.ts` → `useTauriEvent`. |
| Nouvelle règle QA | Variante de `QaError` et score dans `core/qa.rs` ; vérifier `core/report.rs` et `QAPanel`. |
| Mise en page d'un seul moteur | Son adaptateur seulement : `engines/<moteur>/layout/fit.rs` et `text.rs`. `core/text_fit/` est commun (ADR-009). |
| Nouveau moteur | `engines/detector.rs` (variante, `db_str`, `from_db_str`, détection) et `engines/<moteur>/` ; puis les `match` exhaustifs sur `Engine` (`commands/project.rs`, `commands/text_fit.rs` `FitEngine`, `core/text_fit/settings.rs` `engine_fit`, `engines/terminology.rs`, `llm/context/mod.rs`, export) ; mise en page : `TokenRules` → `XxxTextBox` (`TextBox` + `FitAdapter`) ; capacités front dans `src/lib/engine.ts`. Le compilateur signale chaque `match` oublié (ADR-006). |
| Capacité d'un moteur dans l'interface | Un prédicat dans `src/lib/engine.ts` (et son test), pas de `engine === "…"` dans les composants. |
| Nouveau dialogue global | Composant dans `src/components/`, état dans un petit store (`stores/textFit` comme modèle), monté dans `AppDialogs.tsx`. |
| Texte d'interface | Même clé dans `src/locales/fr.json` et `en.json`. |
| Schéma de base | Nouvelle migration dans `src-tauri/migrations/` ; jamais modifier une migration publiée. |

---

## Règles à ne jamais casser

| Règle | Référence |
|---|---|
| SQLite côté Rust seulement (sqlx async), jamais depuis le JS | ADR-001 |
| Codes d'échappement tokenisés avant le LLM, restaurés après | ADR-002 |
| TM globale à l'installation | ADR-003 |
| `lib.rs` est l'entrée de l'app | ADR-005 |
| Moteur aiguillé par `match` sur l'enum `Engine`, pas d'objets-traits | ADR-006 |
| Mise à jour automatique signée, limitée aux installations qui la supportent | ADR-007 |
| Wolf lu et exporté depuis les fichiers libres de `Data/` ; archive de texte présente ⇒ refus | ADR-008 |
| Mise en page : une échelle commune, un adaptateur par moteur, dispatch statique | ADR-009 |
| La crate `unwolf` n'est importée que par `engines/wolf/unpack.rs` ; archives déplacées, jamais supprimées | ADR-010 |
| Un export ne sort jamais sans passer la garde d'intégrité du moteur | `engines/*/integrity.rs` |

---

## Tests

| Niveau | Où |
|---|---|
| Unitaires Rust | `#[cfg(test)] mod tests` dans chaque module ; tests sur vrais jeux en `#[ignore]` (variables `H2S_*`). |
| Intégration Rust | `src-tauri/tests/` : parcours complet (`e2e_project_flow.rs`), export, mise en page (dont le corpus figé `text_fit_corpus.rs`), couverture, terminologie, migrations. |
| Front | `src/**/*.test.{ts,tsx}` (Vitest, jsdom) ; IPC simulé par `mockIPC` ; `src/test/setup.ts`, `src/test/renderWithQuery.tsx`. |

Gate : `pnpm typecheck && pnpm test && cargo clippy -- -D warnings && cargo test` (en session cloud : la compilation
gèle le poste de développement).

---

## Pas dans cette version

- **VX Ace** : code conservé, détection désactivée.
- **Polices WolfX** : non déchiffrées par l'app (choix de la police d'ajustement Wolf : `BACKLOG.md`). Pro V1/V2 :
  UberWolf en recours.
- **RPG Developer Bakin** : non pris en charge.
- **Passes LLM de relecture et de ton** : seule la passe de traduction existe.
- **Synchronisation Git collaborative** : non commencée.
