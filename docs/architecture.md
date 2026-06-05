# Hoshi2Star — Architecture

> Dernière mise à jour : 2026-06-05 (version 0.3.2)
> Ce document décrit l'architecture réelle de l'application.
> À mettre à jour à chaque ajout de module majeur.

---

## Vue d'ensemble

```
┌───────────────────────────────────────────────────────────┐
│  CAT UI (React 19 + TypeScript)    src/                   │
│  Zustand stores · TanStack Table · shadcn/ui · i18next    │
│                                                           │
│  App.tsx → invoke() → Tauri IPC ← events h2s://…         │
├───────────────────────────────────────────────────────────┤
│  Commands Layer   src-tauri/src/commands/                 │
│  project · translate · export · qa · glossary             │
├───────────────────────────────────────────────────────────┤
│  LLM Layer        src-tauri/src/llm/                      │
│  tokenizer · provider · batch · pipeline · split · progress│
├───────────────────────────────────────────────────────────┤
│  Core Layer       src-tauri/src/core/                     │
│  tm · qa · glossary · manifest · report                   │
├───────────────────────────────────────────────────────────┤
│  Engine Layer     src-tauri/src/engines/                  │
│  detector · mv_mz/ · vx_ace/ (désactivé)                  │
├───────────────────────────────────────────────────────────┤
│  Shared utilities                                         │
│  domain/types.rs · utils/text · utils/time · db/pool     │
└───────────────────────────────────────────────────────────┘
```

**Règle absolue :** tout accès données depuis TypeScript passe par `invoke()`. Jamais de DB direct depuis le JS.

**Events Rust → TS :** préfixe `h2s://` obligatoire (ex: `h2s://llm/progress`).

---

## Stack technique

| Couche | Technologie | Version | Rôle |
|--------|------------|---------|------|
| Runtime desktop | Tauri | v2 | IPC, plugins système, fenêtre |
| Backend | Rust stable | 1.75+ | Logique métier, parsers, LLM |
| Async runtime | tokio | full features | Tâches LLM longues non-bloquantes |
| Base de données | SQLite via sqlx | 0.8 | Projets, segments, TM, glossaire |
| Sérialisation | serde + serde_json | — | Structs IPC Rust ↔ TypeScript |
| Erreurs Rust | thiserror | — | Enum `PipelineError`, `LlmError`, etc. |
| Frontend | React | 19 | UI composants |
| Langage frontend | TypeScript strict | — | Pas d'`any` implicite |
| Composants UI | shadcn/ui (owned) | — | `src/components/ui/` |
| Data grid | TanStack Table v8 + Virtual | — | Grille virtuelle 10k+ lignes |
| État global | Zustand | — | Slices typées, sélecteurs exportés |
| Styling | Tailwind CSS | v4 | Via shadcn |
| Build | Vite | — | Bundler par défaut Tauri |
| Package manager | pnpm | — | Uniquement pnpm — jamais npm/yarn |
| i18n | i18next | — | EN + FR, fichiers `src/locales/` |

---

## Architecture Rust (backend)

### `domain/types.rs`

Contient les 8 structs sérialisées via IPC entre le frontend TypeScript et les commandes Rust :
`Project`, `SourceFile`, `Segment`, `PaginatedSegments`, `ProviderConfig`, `OpenProjectResult`, `ProjectStats`, `QaReport`.

Séparé de `commands/` pour que les futurs modules (ex : `sync/git.rs` prévu en F5) puissent dépendre de ces types sans créer un couplage vers la couche commandes. `ProviderConfig` importe `DEFAULT_OLLAMA_URL` / `DEFAULT_OLLAMA_MODEL` depuis `llm::provider` pour son `impl Default` — unique source de vérité pour les valeurs Ollama.

### `commands/`

Tous les `#[tauri::command]` sont déclarés dans des sous-modules et enregistrés dans un unique `generate_handler![…]` dans `lib.rs`.

| Fichier | Commandes Tauri exposées | Raison du regroupement |
|---------|--------------------------|------------------------|
| `project.rs` (~727 lignes) | `open_project`, `get_source_files`, `get_segments`, `update_segment`, `list_projects`, `delete_project`, `get_project_stats` | CRUD projet + fichiers + segments. Contient aussi les helpers privés d'extraction (`dispatch_extract`, `classify_mv_mz_file`, etc.) étroitement couplés à `open_project`. |
| `translate.rs` (~449 lignes) | `translate_segments`, `translate_all_segments`, `get_ollama_models` | Toutes les commandes qui déclenchent une interaction LLM. `translate_all_segments` démarre un `tokio::spawn` et gère le cooldown automatique entre fichiers. |
| `export.rs` (~195 lignes) | `export_project`, `export_qa_report`, `export_tm`, `export_debug_json` | Toutes les commandes qui écrivent un fichier sur le disque via `tauri-plugin-dialog`. |
| `qa.rs` (~93 lignes) | `qa_check_segment`, `get_qa_report`, `get_tm_suggestions` | Commandes de lecture QA/TM — pas d'écriture DB. |
| `glossary.rs` (~116 lignes) | `get_glossary`, `add_glossary_term`, `update_glossary_term`, `delete_glossary_term`, `extract_glossary_terms` | CRUD glossaire + extraction LLM des termes depuis Actors/Skills/Items. |

### `core/`

Couche métier pure — pas de `tauri::State`, pas d'`AppHandle`, testable sans infra Tauri.

| Fichier | Rôle |
|---------|------|
| `tm.rs` | Translation Memory : insert, `lookup_exact` (hash SHA-256), `lookup_fuzzy` (Levenshtein normalisé, seuil 80 %, max 5 résultats). Export TMX via `generate_tmx()`. TM globale (ADR-003) — une table `tm_entries` par installation, partagée cross-projet. |
| `qa.rs` | Checks QA sur chaque segment : placeholders manquants (−25 pts), ligne trop longue en pixels (−10 pts), BOM UTF-8 (−15 pts), terme glossaire non respecté (−15 pts). Score plancher 0. Méthode `QaError::label(&self, lang)` pour les labels localisés. |
| `glossary.rs` | CRUD des termes glossaire. Deux niveaux : global (`project_id IS NULL`) et projet-local. Injection dans le prompt LLM (30 termes max, filtrés par contenu du batch). |
| `manifest.rs` | Écrit/lit `.hoshi2star.json` à la racine du dossier jeu. Permet la « smart restore » : si manifest + entrée DB correspondent à la réouverture, le projet est chargé sans ré-extraction. Mise à jour automatique des stats après `update_segment` et après chaque traduction de batch. |
| `report.rs` | Génère le rapport QA HTML autonome (CSS + JS inline, aucune dépendance externe). Recalcule les erreurs QA au moment de l'export (pas stockées en DB) pour avoir un rapport frais. Filtre interactif par fichier, score, type d'erreur. |

### `llm/`

| Fichier | Rôle |
|---------|------|
| `tokenizer.rs` | Remplace les codes d'échappement RPG Maker (`\V[n]`, `\C[n]`, `\N[n]`, `\n` littéral, etc.) par des tokens opaques `⟦ph_N⟧` avant envoi au LLM. Restaure après réponse. Deux modes : `MvMz` (groupes A+B+D+E) et `MzOnly` (C+A+B+D). |
| `provider.rs` | Trait `LlmProvider` (`translate`, `health_check`, `chat`). Implémentation `OllamaProvider` — REST `POST /api/chat`, parsing réponse numérotée `[1] text`, strip des blocs `<think>` (qwen3). Constantes `DEFAULT_OLLAMA_URL`, `DEFAULT_OLLAMA_MODEL`. |
| `batch.rs` | `group_segments` — découpe une liste d'IDs en lots de taille fixe. `dedup_by_hash` — déduplique les textes source identiques avant envoi LLM (économise des appels quand le jeu réutilise la même chaîne). |
| `pipeline.rs` | Orchestration haut niveau : `run_inner` (testable, prend une closure `on_progress`) et `run` (wrapper Tauri qui émet `h2s://llm/progress` + `h2s://llm/placeholder-warning`). Pour chaque batch : TM lookup → tokenize → `llm_translate_with_split` → DB update. |
| `split.rs` | `llm_translate_with_split` — fonction récursive `Box::pin` qui gère les échecs de traduction. Sur `ResponseFormat` ou échec de restauration des placeholders après `MAX_RETRIES` : si batch > 1 segment, split en deux et récursion ; si batch = 1 segment, marque `needs_review`. |
| `progress.rs` | Types des events `h2s://llm/*` : `ProgressPayload` (`done`, `total`) et `PlaceholderWarningPayload` (`segmentId`). |

### `engines/`

| Fichier / Dossier | Rôle |
|-------------------|------|
| `detector.rs` | Détection automatique du moteur à partir du dossier jeu. Ordre de test : MV/MZ (`data/*.json` présents) avant VX Ace (`data/*.rvdata2` ou `Data/*.rvdata2`). Retourne `Engine` enum + chemin du dossier `Data/`. |
| `mv_mz/extractor.rs` | Lit les fichiers JSON de `data/` (Actors, Armors, Weapons, Skills, Items, Enemies, Classes, CommonEvents, MapInfos, Maps, System). Décrypte `.rpgmvp`/`.rpgmvo` si nécessaire. Retourne des `Vec<(json_key, source_text)>`. |
| `mv_mz/injector.rs` | Réécrit les fichiers JSON avec les traductions. Conserve la structure JSON d'origine — only `value` fields modifiés. |
| `mv_mz/decryptor.rs` | Décryptage XOR des assets chiffrés RPG Maker MV/MZ. Clé lue depuis `System.json`. |
| `vx_ace/` | Extractor + injector RPG Maker VX Ace via marshal-rs (Ruby Marshal binary). **Code disponible mais désactivé** dans `engines/detector.rs` — réactivation prévue post-Wolf RPG stable. |

### `utils/`

| Fichier | Rôle |
|---------|------|
| `text.rs` | `escape_xml(s)` — échappe `&`, `<`, `>`, `"`. Partagé par `core/tm.rs` (export TMX) et `core/report.rs` (rapport HTML). |
| `time.rs` | `now_iso8601()` — horodatage ISO-8601 sans crate externe. Utilisé par `core/manifest.rs`. |

### `db/`

| Fichier | Rôle |
|---------|------|
| `pool.rs` | Initialise le `SqlitePool` avec `SqlitePoolOptions` (max 5 connexions, foreign keys ON). Lance `sqlx::migrate!("./migrations")` au démarrage — les migrations sont embarquées dans le binaire. |
| `migrations/` | 4 fichiers SQL : `0001_initial.sql` (projects, source_files, segments), `0002_tm.sql` (tm_entries), `0003_glossary.sql` (glossary_terms), `0004_source_files_translation_secs.sql` (colonne durée de traduction). |

---

## Architecture TypeScript (frontend)

### `stores/`

| Fichier | État géré | Thunks / actions |
|---------|-----------|-----------------|
| `editor.ts` | `activeFileId`, `activeSegmentId`, `activeSegmentSourceText/TargetText`, `glossaryTerms` | Sélecteurs exportés (`useActiveFileId`, `useGlossaryTerms`, etc.) |
| `project.ts` | `projects[]`, `activeProjectId`, `sourceFiles[]`, `pendingGlossaryExtract`, `isExtractingGlossary` | `addProject`, `setActiveProject`, `setSourceFiles`, `removeProject`. Thunks `openProject`, `loadAllProjects`, `deleteProject` (font des `invoke()` directement dans le store). |
| `llm.ts` | `isTranslating`, `translationProgress`, `providerConfig`, `isCooling`, `cooldownRemaining` | `startTranslation`, `startTranslateAll`, `setupTranslationListeners` (factorisation des 7 listeners en un seul helper) |
| `settings.ts` | Thème, langue, `providerConfig` persisté via `tauri-plugin-store` dans `settings.json` | `loadSettings`, `saveSettings` |

### `components/editor/`

| Composant | Rôle | Stores / commands clés |
|-----------|------|----------------------|
| `SegmentGrid.tsx` | Grille principale — TanStack Table + virtual scroll. Édition inline colonne Target. Filtres QA (All / Errors / Critical / Untranslated / Needs Review). Checkbox de sélection multiple + bouton "Traduire N lignes". | `useEditorStore`, `get_segments`, `update_segment`, `translate_segments` |
| `FileTree.tsx` | Liste les fichiers du projet actif. Clic → sélectionne le fichier actif. Badge durée de traduction (depuis `translation_secs` DB). | `useProjectStore`, `get_source_files` |
| `TMPanel.tsx` | Affiche suggestions TM (exact + fuzzy %). Clic applique la suggestion dans le segment actif. | `get_tm_suggestions`, `useEditorStore` |
| `QAPanel.tsx` | Affiche les erreurs QA live sur le segment actif (recalcul local). Bouton export rapport HTML. | `qa_check_segment`, `export_qa_report` |
| `GlossaryPanel.tsx` | CRUD inline des termes glossaire. Bouton auto-extraction LLM. | `get_glossary`, `add/update/delete_glossary_term`, `extract_glossary_terms` |
| `ProjectList.tsx` | Affiché si aucun projet actif. Liste tous les projets DB avec boutons Continuer / Supprimer. | `list_projects`, `delete_project` |

### `components/`

| Composant | Rôle |
|-----------|------|
| `AppToolbar.tsx` | Barre d'outils principale — boutons Open/Translate/TranslateAll/ExportAll, badge projet actif + moteur, `TranslationTimer`, `CooldownBadge`, barre de progression. Lit les stores directement. |
| `AppDialogs.tsx` | Toutes les modales conditionnelles de l'application — `SettingsModal`, `AboutModal`, `TranslateAllDialog`, `AlertDialog` export (confirm + blocked), `AlertDialog` glossaire. Reçoit un objet `handlers` depuis `App.tsx`. |
| `SettingsModal.tsx` | Ollama URL + modèle, thème clair/sombre, langue EN/FR. Persisté via `tauri-plugin-store`. |
| `AboutModal.tsx` | Tagline, auteur, licence MIT, adresses Bitcoin/Ethereum, lien GitHub. |
| `TranslateAllDialog.tsx` | Stats projet + inputs durée travail / pause avant de lancer `translate_all_segments`. |

### `hooks/`

| Fichier | Rôle |
|---------|------|
| `useAppHandlers.ts` | Hook appelé une seule fois dans `App.tsx`. Encapsule les 7 handlers async de l'application (`handleTranslate`, `handleTranslateAll`, `handleTranslateAllStart`, `handleExportAll`, `handleExportConfirm`, `handleGlossaryConfirm`, `handleGlossaryDecline`) et les états dialog locaux (`showSettings`, `showAbout`, `exportDialog`, `exportStats`, `showTranslateAll`, `translateAllStats`). Gère aussi le listener `h2s://glossary/extraction-done`. |

### `lib/`

| Fichier | Rôle |
|---------|------|
| `types.ts` | Interfaces TypeScript miroirs des structs Rust domain (`Project`, `Segment`, `TmSuggestion`, `QaResult`, `GlossaryTerm`, etc.). |
| `constants.ts` | `PH_RE` — regex des placeholders. `clonePH_RE()` retourne une instance fraîche avec `lastIndex` remis à zéro — utilisée par `AppToolbar.tsx`, `columns.tsx`, `highlight-utils.tsx`. |
| `format.ts` | `formatDuration(secs)`, `engineLabel(engine)`, `relativeDate(iso)` — helpers partagés par `FileTree.tsx` et `ProjectList.tsx`. |
| `highlight-utils.tsx` | `buildHighlightedNodes(text, glossaryTerms: string[], phRe: RegExp)` — surbrillance simultanée placeholders (bleu) et termes glossaire (vert). Testable sans shadcn ni stores. |
| `i18n.ts` | Configuration i18next. Ressources EN/FR dans `src/locales/`. |
| `utils.ts` | `cn()` — helper Tailwind merge (généré par shadcn). |

### `features/editor/`

| Fichier | Rôle |
|---------|------|
| `columns.tsx` | Définitions colonnes TanStack Table pour `SegmentGrid`. Contient `EditableCell` (colonne Target éditable inline) et `SourceCell` (qui appelle `buildHighlightedNodes` depuis `@/lib/highlight-utils`). |

---

## Flux de données — Cas d'usage principaux

### Ouvrir un projet

```
Clic "Open" → tauri-plugin-dialog → chemin absolu
  → invoke('open_project', { path })
  → commands/project.rs : open_project()
      → engines/detector.rs : detect_engine()   [quel moteur ?]
      → core/manifest.rs : read()               [manifest existe ?]
        → si manifest + DB match → wasRestored: true → retour immédiat
        → sinon → engines/mv_mz/extractor.rs    [extraction JSON]
          → INSERT INTO projects, source_files, segments
          → core/manifest.rs : write()          [crée .hoshi2star.json]
  → OpenProjectResult { project, wasRestored }
  → useProjectStore.addProject()
  → App.tsx : si !wasRestored → pendingGlossaryExtract = project.id
```

### Traduire un fichier

```
Clic Translate → invoke('translate_segments', { fileId, providerConfig })
  → commands/translate.rs : translate_segments()
      → tokio::spawn (non-bloquant)
          → core/glossary.rs : list_for_project() [termes filtrés]
          → llm/pipeline.rs : run()
              → llm/batch.rs : group_segments()   [lots de 20]
              → pour chaque lot :
                  → core/tm.rs : lookup_exact()   [TM hit ?]
                  → llm/tokenizer.rs : tokenize() [⟦ph_N⟧]
                  → llm/split.rs : llm_translate_with_split()
                      → llm/provider.rs : OllamaProvider::translate()
                      → Tokenizer::restore()
                      → si échec MAX_RETRIES → split récursif
                  → app.emit("h2s://llm/progress", { done, total })
          → UPDATE segments SET target_text, status
          → core/manifest.rs : update_stats()
          → app.emit("h2s://llm/completed", { count })
  ← useTranslationListeners() reçoit les events
  ← SegmentGrid se rafraîchit
```

### Réouvrir un projet existant

```
Clic sur un projet dans ProjectList
  → invoke('open_project', { path: project.gamePath })
  → core/manifest.rs : read() → ManifestData.projectId trouvé
  → SELECT * FROM projects WHERE id = manifest.projectId
  → si trouvé → retour immédiat sans extraction
  → OpenProjectResult { project, wasRestored: true }
  → App.tsx : toast "Project restored — continuing where you left off"
```

---

## Décisions d'architecture (ADRs)

| ADR | Décision | Lien |
|-----|----------|------|
| ADR-001 | SQLite via sqlx async (pas rusqlite sync, pas tauri-plugin-sql) — isolation DB côté Rust | [docs/adr/ADR-001.md](adr/ADR-001.md) |
| ADR-002 | Placeholder tokenisation Rust-side avant tout envoi LLM — UUID opaque `⟦ph_N⟧` | [docs/adr/ADR-002.md](adr/ADR-002.md) |
| ADR-003 | TM globale à l'installation (pas par projet) — fuzzy cross-projet = différenciateur clé | [docs/adr/ADR-003.md](adr/ADR-003.md) |
| ADR-004 | MVP limité à RPG Maker MV/MZ (JSON natif) — VX Ace ajouté via marshal-rs, en attente | [docs/adr/ADR-004.md](adr/ADR-004.md) |
| ADR-005 | `lib.rs` comme entrée app (pas `main.rs`) — requis pour builds mobiles Tauri futurs | [docs/adr/ADR-005.md](adr/ADR-005.md) |

---

## Ce qui n'est PAS dans cette version

- **RPG Maker VX Ace** — code complet dans `engines/vx_ace/` mais désactivé dans `detector.rs`. Réactivation prévue post-Wolf RPG stable.
- **Wolf RPG** — priorité absolue F4, pas encore commencé. Fichiers `.dat`/`.mps` + décryptage `.wolf` (DXA/WolfDec).
- **RPG Developer Bakin** — F5, dépend de l'adoption DLC.
- **Passe LLM review / tone** — pipeline multi-passes prévu mais passe 1 (translate) seulement implémentée.
- **OpenAI / DeepSeek providers** — `OllamaProvider` seulement. Trait `LlmProvider` prêt pour d'autres implémentations.
- **Système de licence** — F4, Polar.sh ou LemonSqueezy.
- **Sync Git collaborative** — F5, `sync/git.rs` via crate `git2`.
