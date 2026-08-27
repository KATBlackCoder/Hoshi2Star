# Hoshi2Star — Architecture

> Dernière mise à jour : 2026-08-27
> Ce document décrit l'architecture réelle de l'application.
> À mettre à jour à chaque ajout de module majeur.

La bibliothèque terminologique est détaillée dans [son document d'architecture](architecture/terminology.md) et dans [le guide utilisateur](terminology.md).

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
│  project · translate · export · qa · terminology · pack   │
├───────────────────────────────────────────────────────────┤
│  LLM Layer        src-tauri/src/llm/                      │
│  tokenizer · provider · batch · pipeline · split · progress│
├───────────────────────────────────────────────────────────┤
│  Core Layer       src-tauri/src/core/                     │
│  tm · qa · terminology/ · manifest · report · h2s_pack    │
├───────────────────────────────────────────────────────────┤
│  Engine Layer     src-tauri/src/engines/                  │
│  detector · mv_mz/ · wolf/ · vx_ace/ (désactivé)          │
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
| Base de données | SQLite via sqlx | 0.8 | Projets, segments, TM, terminologie |
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

Contient les structs sérialisées via IPC entre le frontend TypeScript et les commandes Rust :
`Project`, `SourceFile`, `Segment`, `PaginatedSegments`, `ProviderConfig`, `OpenProjectResult`, `ProjectStats`, `QaReport`, `FontScanResult`.

`SourceFile` porte `translated_count`, `needs_review_count` et `total_count` (`i64`, `#[sqlx(default)]`) — calculés dans `get_source_files` (helper testable `fetch_source_files(pool, project_id)`) par `LEFT JOIN segments` + agrégation. **Sémantique stricte (Option 2, audit 2026-07-01)** : `translated_count` compte `status = 'translated'` uniquement (plus `target_text != ''`) ; `needs_review_count` compte `status = 'needs_review'`. Les requêtes qui ne font pas ce JOIN (ex: `debug_inject_file`) utilisent `0 as total_count, 0 as translated_count` comme littéraux (le champ manquant retombe sur `#[sqlx(default)]`).

`ProjectStats` porte les 4 compteurs de statut (`untranslated_count`, `translated_count`, `needs_review_count`, `reviewed_count`) + `file_count`/`total_segments` — invariant : la somme des 4 == `total_segments` (helper testable `fetch_project_stats(pool, project_id)`, figé par `test_counter_semantics_one_segment_per_status`). Côté UI : le FileTree affiche `✓ N · ⚠ M` par fichier et ne propose l'injection debug que si tout est `translated` ; `SegmentStatsBar` (ProjectList) rend translated (vert) / reviewed (bleu) / needs_review (ambre) et somme à 100 %.

`FontScanResult { existing_font_count, total_translated }` est renvoyé par `scan_font_status` pour alimenter `FontSizeDialog` avant toute injection ou export.

Séparé de `commands/` pour que les futurs modules (ex : `sync/git.rs` prévu en F5) puissent dépendre de ces types sans créer un couplage vers la couche commandes. `ProviderConfig` importe `DEFAULT_OLLAMA_URL` / `DEFAULT_OLLAMA_MODEL` / `DEFAULT_BATCH_SIZE` depuis `llm::provider` pour son `impl Default` — source de vérité Rust pour les valeurs Ollama et la taille de lot, **miroir TS dans `src/lib/constants.ts`** (mêmes noms, mêmes valeurs — modèle par défaut unifié `gemma4:e4b`, consommé par `stores/settings.ts` et `stores/llm.ts`). `ProviderConfig.batch_size` (`#[serde(default = "default_batch_size")]` pour la rétrocompat) est copié vers `TranslationContext.batch_size` dans `commands/translate.rs`, puis clampé `[1, 100]` par `pipeline::run_inner` avant `batch::group_segments`.

### `commands/`

Tous les `#[tauri::command]` sont déclarés dans des sous-modules et enregistrés dans un unique `generate_handler![…]` dans `lib.rs`.

| Fichier | Commandes Tauri exposées | Raison du regroupement |
|---------|--------------------------|------------------------|
| `project.rs` | `open_project`, `get_source_files`, `get_segments`, `update_segment`, `list_projects`, `delete_project`, `get_project_stats`, `debug_dump_segments` | CRUD projet + fichiers + segments. `extract_project(engine, game_dir, data_dir)` normalise l'extraction des 3 moteurs vers une forme commune, partagée par `open_project` (persistance, via les helpers `insert_source_file`/`insert_segment` dans la transaction unique) et `debug_dump_segments` (dump JSON). Le dispatch moteur lui-même vit dans la couche moteur (cf. `detector.rs`, `filter.rs`), pas ici (ADR-006). |
| `translate.rs` (~400 lignes) | `translate_segments`, `translate_all_segments`, `get_ollama_models` | Toutes les commandes qui déclenchent une interaction LLM. `translate_all_segments` démarre un `tokio::spawn`, crée un `pipeline::CooldownState` (une fois pour tout le projet) et le passe à `pipeline::run` — le cooldown est désormais vérifié par batch (pas par fichier). |
| `export.rs` | `export_project`, `export_qa_report`, `export_tm`, `export_debug_json`, `debug_inject_file`, `scan_font_status` | Toutes les commandes qui écrivent un fichier sur le disque ou injectent les données du jeu. `debug_inject_file` enforce la complétude avant injection. `scan_font_status` compte les segments déjà préfixés `\f[N]`. `apply_font_prefix` / `persist_font_size` (helpers privés) gèrent le préfixe Wolf RPG taille police. |
| `qa.rs` (~93 lignes) | `qa_check_segment`, `get_qa_report`, `get_tm_suggestions` | Commandes de lecture QA/TM — pas d'écriture DB. |
| `terminology.rs` | liste/stats, CRUD, scan/cancel, traduction sélectionnée, inspecteur segment, compatibilité `extract_wolf_speakers` | API étroite : validation IPC puis délégation au repository/service/resolver. Aucun SQL terminologique dans React. |
| `pack_terminology.rs` | mapping import/export terminologique `.h2s` v1 | Couche de compatibilité isolée vers le champ historique `glossary.json`; protège les traductions verrouillées. |
| `app.rs` | `updater_supported` | Commandes au niveau app, hors couche métier. `updater_supported() -> bool` (`cfg!(windows) || env APPIMAGE`) gate l'UI d'auto-update aux installs réellement updatables (Windows NSIS + Linux AppImage) ; deb/rpm et dev renvoient `false` (ADR-007). |

### `core/`

Couche métier pure — pas de `tauri::State`, pas d'`AppHandle`, testable sans infra Tauri.

| Fichier | Rôle |
|---------|------|
| `tm.rs` | Translation Memory : insert, `lookup_exact` (hash SHA-256), `lookup_fuzzy` (Levenshtein normalisé, seuil 80 %, max 5 résultats). Export TMX via `generate_tmx()`. TM globale (ADR-003) — une table `tm_entries` par installation, partagée cross-projet. |
| `qa.rs` | Checks QA sur chaque segment : placeholders, largeur, BOM et cohérence terminologique selon révision/enforcement. La preview reste pure. |
| `terminology/` | Analyse japonaise Lindera, scan incrémental, repository paginé, traduction de candidats et resolver commun au LLM et au QA. Les règles moteur restent dans `engines/*/terminology.rs`. Voir le [document dédié](architecture/terminology.md). |
| `manifest.rs` | Écrit/lit `.hoshi2star.json` à la racine du dossier jeu. Permet la « smart restore » : si manifest + entrée DB correspondent à la réouverture, le projet est chargé sans ré-extraction. `refresh_stats(pool, project_id)` est le **point unique** de recalcul des stats projet (best-effort), appelé après `update_segment` et après chaque traduction de batch. |
| `report.rs` | Génère le rapport QA HTML autonome (CSS + JS inline, aucune dépendance externe). Recalcule les erreurs QA au moment de l'export (pas stockées en DB) pour avoir un rapport frais. Filtre interactif par fichier, score, type d'erreur. |

### `llm/`

| Fichier | Rôle |
|---------|------|
| `tokenizer.rs` | Remplace les codes d'échappement RPG Maker (`\V[n]`, `\C[n]`, `\N[n]`, `\n` littéral, etc.) par des tokens opaques `⟦ph_N⟧` avant envoi au LLM. Restaure après réponse. Deux modes : `MvMz` (groupes A+B+D+E) et `MzOnly` (C+A+B+D). |
| `provider.rs` | Trait `LlmProvider` (`translate`, `health_check`, `chat`). Implémentation `OllamaProvider` — REST `POST /api/chat`, parsing réponse numérotée `[1] text`, strip des blocs `<think>` (qwen3). Constantes `DEFAULT_OLLAMA_URL`, `DEFAULT_OLLAMA_MODEL`. |
| `batch.rs` | `group_segments` — découpe une liste d'IDs en lots de taille fixe. `dedup_by_hash` — déduplique les textes source identiques avant envoi LLM (économise des appels quand le jeu réutilise la même chaîne). |
| `pipeline.rs` | Orchestration haut niveau : `run_inner` (testable, prend `app_handle: Option<&AppHandle>`, `cooldown: Option<&mut CooldownState>` et une closure `on_progress`) et `run` (wrapper Tauri fin). Pour chaque batch, **séquentiellement** : TM lookup → tokenize → `llm_translate_with_split` → `persist_batch_results` (UPDATE `segments` immédiat — persistance incrémentale) → emit `h2s://llm/progress`/`placeholder-warning` → `CooldownState::maybe_rest` (si fourni, `.await` une pause `h2s://llm/cooling` avant le batch suivant). |
| `split.rs` | `llm_translate_with_split` — fonction récursive `Box::pin` qui gère les échecs de traduction. Sur `ResponseFormat` ou échec de restauration des placeholders après `MAX_RETRIES` : si batch > 1 segment, split en deux et récursion ; si batch = 1 segment, marque `needs_review`. |
| `progress.rs` | Types des events `h2s://llm/*` : `ProgressPayload` (`done`, `total`), `PlaceholderWarningPayload` (`segmentId`) et `CoolingPayload` (`remainingSecs`). |

### `engines/`

| Fichier / Dossier | Rôle |
|-------------------|------|
| `detector.rs` | Détection automatique du moteur à partir du dossier jeu. Ordre de test : MV/MZ (`data/*.json`) → VX Ace (`data/*.rvdata2`) → Wolf RPG (`Game.exe` + `Data/`). Retourne `Engine` enum + chemin du dossier `Data/`. **Point unique de dispatch métadonnées** (ADR-006) : `impl Engine { db_str, data_dir, game_title }` — les lecteurs de titre par moteur (System.json / System.rvdata2 / Game.ini) vivent ici, pas dans la couche commande. |
| `mv_mz/extractor.rs` | Lit les fichiers JSON de `data/` (Actors, Armors, Weapons, Skills, Items, Enemies, Classes, CommonEvents, MapInfos, Maps, System). Décrypte `.rpgmvp`/`.rpgmvo` si nécessaire. Retourne des `Vec<(json_key, source_text)>`. |
| `mv_mz/injector.rs` | Réécrit les fichiers JSON avec les traductions. Conserve la structure JSON d'origine — only `value` fields modifiés. |
| `mv_mz/decryptor.rs` | Décryptage XOR des assets chiffrés RPG Maker MV/MZ. Clé lue depuis `System.json`. |
| `vx_ace/` | Extractor + injector RPG Maker VX Ace via marshal-rs (Ruby Marshal binary). **Code disponible mais désactivé** dans `engines/detector.rs` — réactivation prévue post-Wolf RPG stable. |
| `wolf/extractor.rs` | Lit les `.mps` (cartes) et `.dat`/`.project` (base de données) depuis `Data/MapData/` et `Data/BasicData/`. Fallback transparent vers les archives `.wolf` (DXA chiffrées) via `wolf/decrypt/legacy_xor.rs`. Exporte `extract_all_wolf()` → `Vec<(file_name, file_type, Vec<WolfSegment>)>`. |
| `wolf/injector.rs` | Réinjecte les traductions dans `.mps`, `.dat` **et `CommonEvent.dat`** via `wolfrpg-map-parser` (v2.x) / `v3_format` (v3.5 Inko, LZ4). Écrit dans `Data/MapData/` et `Data/BasicData/` (Option A — priorité sur les archives). `inject_map` / `inject_dat` / `inject_common_events` sont routés par `inject_all` (disque) et `inject_all_to_memory` (ZIP) selon le préfixe de clé. `injection_bucket(segment_key)` mappe une clé de segment vers son fichier physique — tous les `CommonEvents/*` **collapsent dans un seul bucket** (`CommonEvent.dat` est unique, contrairement aux `.mps`/`.dat` un-par-stem). Charge les bytes sources via `load_mps_for_stem`/`load_dat_for_stem`/`load_common_event_bytes` (archive-aware). Splice binaire v2.x partagé par `.mps` et `CommonEvent.dat` (`splice_wolf_strings`). |
| `wolf/decrypt/legacy_xor.rs` | Décryptage XOR des archives `.wolf` (DXA v2/v3 jusqu'à v3.31). Détecte la clé par heuristique. Émet `PossibleWolfX` si toutes les clés échouent (WolfX v3.5+). |
| `wolf/decrypt/wolfx.rs` | Couture WolfX (Wolf v3.5+ Pro, ChaCha20) — **non implémenté par décision** : pré-étape manuelle UberWolf, puis ouvrir le dossier `Data/` déchiffré. Renvoie une erreur de guidage. |
| `wolf/encoding.rs` | Conversion Shift-JIS ↔ UTF-8 pour les fichiers Wolf v2/v3. |
| `wolf/dat_parser.rs` | Parseur binaire des fichiers `.dat` + `.project` Wolf RPG (format WolfTL). |

### `utils/`

| Fichier | Rôle |
|---------|------|
| `text.rs` | `escape_xml(s)` — échappe `&`, `<`, `>`, `"`. Partagé par `core/tm.rs` (export TMX) et `core/report.rs` (rapport HTML). |
| `time.rs` | `now_iso8601()` — horodatage ISO-8601 sans crate externe. Utilisé par `core/manifest.rs`. |

### `db/`

| Fichier | Rôle |
|---------|------|
| `pool.rs` | Initialise le `SqlitePool` avec `SqlitePoolOptions` (max 5 connexions, foreign keys ON), puis applique les migrations SQL embarquées. Une réparation transactionnelle et idempotente vérifie ensuite `pragma_table_info('source_files')` et ajoute la colonne nullable `translation_secs` uniquement si elle manque. Sont supportées les bases neuves, les snapshots antérieurs à `0004`, les bases dont le ledger SQLx indique `0004` ou plus mais dont la colonne manque, et les bases courantes déjà conformes. La réparation ne réécrit aucune ligne existante ; en cas d'échec elle effectue un rollback et l'erreur demande de sauvegarder le fichier avant de réessayer. Les réglages Tauri étant stockés hors de cette base, ils ne sont pas touchés. |
| `migrations/` | Migrations SQL immuables jusqu'à `0008_terminology_library.sql`. `0008` crée entrées, traductions, occurrences, scans et état incrémental, puis migre les données historiques. `glossary_terms` reste une donnée de rollback, sans lecteur applicatif actif. Les migrations publiées ne sont jamais réécrites afin de préserver leurs checksums SQLx. |

---

## Architecture TypeScript (frontend)

### `stores/`

| Fichier | État géré | Thunks / actions |
|---------|-----------|-----------------|
| `editor.ts` | `activeFileId`, `activeSegmentId`, textes source/cible actifs | État minimal de l'éditeur et sélecteurs ciblés. Les données terminologiques serveur n'y sont pas dupliquées. |
| `project.ts` | `projects[]`, `activeProjectId`, `sourceFiles[]` | `addProject`, `setActiveProject`, `setSourceFiles`, `removeProject`. Thunks de cycle de vie projet. |
| `llm.ts` | `isTranslating`, `translationProgress`, `providerConfig`, `isCooling`, `cooldownRemaining` | `startTranslation`, `startTranslateAll`, `setupTranslationListeners` (factorisation des 7 listeners en un seul helper) |
| `settings.ts` | Thème, langue, `providerConfig` persisté via `tauri-plugin-store` dans `settings.json` | `loadSettings`, `saveSettings` |
| `updater.ts` | Machine à états auto-update : `status: idle\|checking\|available\|downloading\|ready\|dismissed\|error`, `version`/`notes`, `downloaded`/`contentLength` (→ %). `dismissed_version` persisté via `tauri-plugin-store` (`updater.json`) | `checkForUpdate` (guard `updater_supported`, silencieux offline/dev, respecte le rejet sauf version plus récente), `startDownload` (callbacks `Started`/`Progress`/`Finished` → %), `dismiss`/`reopen`, `postpone`, `applyAndRestart` (`relaunch()`). Rendu par `UpdateDialog.tsx` (AlertDialog Oui/Non/redémarrer) + `UpdateBadge.tsx` (icône toolbar visible si `dismissed`). Check déclenché une fois au mount dans `App.tsx`. Voir ADR-007. |

### `components/editor/`

| Composant | Rôle | Stores / commands clés |
|-----------|------|----------------------|
| `SegmentGrid.tsx` | Grille principale — TanStack Table + virtual scroll. Édition inline colonne Target. Filtres QA (All / Errors / Critical / Untranslated / Needs Review). Checkbox de sélection multiple + bouton "Traduire N lignes". | `useEditorStore`, `get_segments`, `update_segment`, `translate_segments` |
| `FileTree.tsx` | Liste les fichiers du projet actif. Clic → sélectionne le fichier actif. Badge durée de traduction (depuis `translation_secs` DB). Bouton "Debug Inject" (`FlaskConical`, hover-only) visible uniquement quand `translatedCount === totalCount > 0`; déclenche `scan_font_status` → `FontSizeDialog` → `debug_inject_file`. Rows : `<div role="button">` (pas `<button>`) pour éviter l'imbrication HTML invalide avec le bouton inject. | `useProjectStore`, `get_source_files`, `scan_font_status`, `debug_inject_file` |
| `TMPanel.tsx` | Affiche suggestions TM (exact + fuzzy %). Clic applique la suggestion dans le segment actif. | `get_tm_suggestions`, `useEditorStore` |
| `QAPanel.tsx` | Affiche les erreurs QA live sur le segment actif (recalcul local). Bouton export rapport HTML. | `qa_check_segment`, `export_qa_report` |
| `TerminologyInspector.tsx` | Affiche en lecture les termes résolus pour le segment actif et ouvre l'espace de travail pour les modifier. | `get_segment_terminology` |
| `ProjectList.tsx` | Affiché si aucun projet actif. Liste tous les projets DB avec boutons Continuer / Supprimer. | `list_projects`, `delete_project` |

### `components/`

| Composant | Rôle |
|-----------|------|
| `AppToolbar.tsx` | Barre d'outils principale — boutons Open/Translate/TranslateAll/ExportAll, badge projet actif + moteur, `TranslationTimer`, `CooldownBadge`, barre de progression. Lit les stores directement. |
| `AppDialogs.tsx` | Modales globales de réglages, traduction complète et export. Les dialogues terminologiques restent dans `features/terminology/`. |
| `SettingsModal.tsx` | Ollama URL + modèle, thème clair/sombre, langue EN/FR. Persisté via `tauri-plugin-store`. |
| `AboutModal.tsx` | Tagline, auteur, licence MIT, adresses Bitcoin/Ethereum, lien GitHub. |
| `TranslateAllDialog.tsx` | Stats projet + inputs durée travail / pause avant de lancer `translate_all_segments`. |
| `FontSizeDialog.tsx` | `AlertDialog` proposant d'appliquer un préfixe `\f[N]` (taille police Wolf RPG) avant toute injection ou export. Input numérique (défaut 18, range 8–64). Checkbox "remplacer existants" si `existingFontCount > 0`. Utilisé par `FileTree.tsx` (debug inject) et `AppDialogs.tsx` (export projet). |

### `hooks/`

| Fichier | Rôle |
|---------|------|
| `useAppHandlers.ts` | Actions globales de traduction/export et états de leurs dialogues. La terminologie possède ses mutations et listeners bornés dans `features/terminology/` afin de ne pas regonfler ce hook. |

### `lib/`

| Fichier | Rôle |
|---------|------|
| `types.ts` | Interfaces TypeScript miroirs des contrats Rust (`Project`, `Segment`, `TmSuggestion`, `QaResult`, `TerminologyEntry`, etc.). |
| `constants.ts` | `PH_RE_SOURCE` (MV/MZ) et `PH_RE_WOLF` (Wolf RPG, miroir de `RE_WOLF` Rust). `getPlaceholderRegex(engine)` retourne la regex adaptée au moteur — utilisée par `SourceCell` dans `columns.tsx` pour le highlight des placeholders. `clonePH_RE()` retourne une instance `PH_RE_SOURCE` fraîche avec `lastIndex` remis à zéro pour les usages non-engine-aware. |
| `format.ts` | `formatDuration(secs)`, `engineLabel(engine)`, `relativeDate(iso)` — helpers partagés par `FileTree.tsx` et `ProjectList.tsx`. |
| `highlight-utils.tsx` | Surbrillance des codes moteur et autres marqueurs éditoriaux, testable sans shadcn ni stores. |
| `i18n.ts` | Configuration i18next. Ressources EN/FR dans `src/locales/`. |
| `utils.ts` | `cn()` — helper Tailwind merge (généré par shadcn). |

### `features/terminology/`

Fonctionnalité autonome chargée à la demande. `TerminologyWorkspace` orchestre de petits composants spécialisés (`Toolbar`, `Table`, `ScanProgress`, dialogues d'édition/traduction). TanStack Query possède les listes/stats serveur et les invalide après mutation; `terminologyUiStore` ne conserve que recherche, filtres, portée et page. La table demande 100 lignes par page et ne charge jamais toute la bibliothèque.

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
  → App.tsx : affiche le projet; le scan terminologique reste une action explicite
```

### Traduire un fichier

```
Clic Translate → invoke('translate_segments', { fileId, providerConfig })
  → commands/translate.rs : translate_segments()
      → tokio::spawn (non-bloquant)
          → core/terminology/resolver.rs [termes présents dans le sous-lot]
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

## Tests

Trois niveaux, exécutés par le gate de vérification
(`pnpm typecheck && pnpm test && cargo clippy -- -D warnings && cargo test`) :

| Niveau | Emplacement | Portée |
|--------|-------------|--------|
| **Unitaires Rust** | `#[cfg(test)] mod tests` inline dans chaque module (`src-tauri/src/**`) | Parsers binaires Wolf (round-trip byte-exact), QA, TM, encodage, détecteur. Les tests contre fixtures réelles font `if !path.exists() { return; }` (fixtures `test/` gitignorées). |
| **Intégration Rust** | `src-tauri/tests/e2e_project_flow.rs` | Flux bout-en-bout via la vraie couche commande : `open_project → update_segment → export_project`. `mock_builder().manage(AppState).build(mock_context(noop_assets()))` porte l'état ; les `pub async fn` sont appelées directement avec un `State` via `Manager::state()` (pas de sérialisation IPC). Scénarios MV et Wolf CommonEvent (ce dernier verrouille la réinjection des Common Events, cf. Phase 1 remédiation). Isolation par `tempfile::tempdir` — aucune pollution des fixtures. |
| **Front (Vitest)** | `src/**/*.test.{ts,tsx}` | Stores Zustand, composants/hooks. `@tauri-apps/api/mocks` (`mockIPC`) simule `invoke()` ; jsdom + `@testing-library/react`. Config : `vitest.config.ts` (séparée de `vite.config.ts`), setup global `src/test/setup.ts`. |

## Décisions d'architecture (ADRs)

| ADR | Décision | Lien |
|-----|----------|------|
| ADR-001 | SQLite via sqlx async (pas rusqlite sync, pas tauri-plugin-sql) — isolation DB côté Rust | [docs/adr/ADR-001.md](adr/ADR-001.md) |
| ADR-002 | Placeholder tokenisation Rust-side avant tout envoi LLM — UUID opaque `⟦ph_N⟧` | [docs/adr/ADR-002.md](adr/ADR-002.md) |
| ADR-003 | TM globale à l'installation (pas par projet) — fuzzy cross-projet = différenciateur clé | [docs/adr/ADR-003.md](adr/ADR-003.md) |
| ADR-004 | MVP limité à RPG Maker MV/MZ (JSON natif) — VX Ace ajouté via marshal-rs, en attente | [docs/adr/ADR-004.md](adr/ADR-004.md) |
| ADR-005 | `lib.rs` comme entrée app (pas `main.rs`) — requis pour builds mobiles Tauri futurs | [docs/adr/ADR-005.md](adr/ADR-005.md) |
| ADR-006 | Dispatch moteur par méthodes sur enum `Engine` (pas de trait-objects) — ensemble fermé + complétude compilateur | [docs/adr/ADR-006.md](adr/ADR-006.md) |
| ADR-007 | Auto-update in-app via `tauri-plugin-updater` (GitHub Releases `latest.json` + signature minisign obligatoire, deb/rpm non couverts) | [docs/adr/ADR-007.md](adr/ADR-007.md) |

---

## Ce qui n'est PAS dans cette version

- **RPG Maker VX Ace** — code complet dans `engines/vx_ace/` mais désactivé dans `detector.rs`. Réactivation prévue post-Wolf RPG stable.
- **Wolf RPG WolfX** — chiffrement WolfX (v3.5+, `WOLF_RPG_Editor_EX`) non supporté. Décrypter manuellement avec UberWolf, puis ouvrir le dossier `Data/` directement. Support planifié v0.5.0.
- **RPG Developer Bakin** — F5, dépend de l'adoption DLC.
- **Passe LLM review / tone** — pipeline multi-passes prévu mais passe 1 (translate) seulement implémentée.
- **OpenAI / DeepSeek providers** — `OllamaProvider` seulement. Trait `LlmProvider` prêt pour d'autres implémentations.
- **Système de licence** — F4, Polar.sh ou LemonSqueezy.
- **Sync Git collaborative** — F5, `sync/git.rs` via crate `git2`.
