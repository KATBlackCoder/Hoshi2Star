# Hoshi2Star — ROADMAP

> Référence de progression pour Claude Code.
> Mettre à jour le statut des tâches au fil du développement.
> Format : `[ ]` à faire · `[x]` fait · `[~]` en cours · `[-]` abandonné/reporté

---

## Légende des phases

| Phase | Horizon | Objectif |
|-------|---------|---------|
| **F0** | Pré-dev | Setup environnement et fondations |
| **F1** | Mois 1–2 | Parsers MV/MZ + UI skeleton |
| **F2** | Mois 3–4 | LLM pipeline + TM + **MVP vendable** |
| **F3** | Mois 5–6 | Polissage + Glossaire + TM fuzzy + beta privée |
| **F4** | Mois 7–9 | Wolf RPG (priorité absolue) + diff-aware + lancement public |
| **F5** | Mois 10–12 | Wolf v3/WolfX + Bakin + consolidation |

---

## Moteurs — ordre de priorité

| Moteur | Statut | Priorité |
|--------|--------|---------|
| RPG Maker MV/MZ | ✅ Supporté | — |
| Wolf RPG v1/v2/v3 | 🔜 F4 | **Absolue** |
| RPG Maker VX Ace | ⏸ Code prêt, désactivé | Post-Wolf RPG |
| RPG Developer Bakin | 🔜 F5 | Basse |
| Autres (Ren'Py, Kirikiri) | 🔜 Backlog | Si demande |

---

## F0 — Setup & fondations
**Statut : [~] En cours**

### Environnement système (CachyOS)
- [x] `webkit2gtk-4.1`, `base-devel`, `libappindicator-gtk3`, `librsvg`, `xdotool` installés via pacman
- [x] `pnpm` installé via pacman (repo `extra`)
- [x] `rustup` installé, toolchain stable active
- [x] Workaround NVIDIA documenté dans `~/.zshrc` si GPU NVIDIA
  - X11 : `WEBKIT_DISABLE_DMABUF_RENDERER=1`
  - Wayland : `__NV_DISABLE_EXPLICIT_SYNC=1`

### Projet Tauri v2 initialisé
- [x] `pnpm create tauri-app` — React + TypeScript + pnpm
- [x] Vérification `lib.rs` comme entrée (pas `main.rs`)
- [x] `pnpm dlx shadcn@latest init` (CSS variables: yes, base color: zinc, preset Nova)
- [x] Tailwind v4 configuré (`@tailwindcss/vite` plugin + `@import "tailwindcss"`)
- [x] `pnpm add zustand @tanstack/react-table @tanstack/react-virtual @tanstack/react-query`
- [x] Rust deps ajoutés : `tokio` (full), `sqlx` (sqlite + runtime-tokio + macros), `serde` (derive), `serde_json`, `thiserror`
- [x] `src-tauri/capabilities/default.json` créé avec `core:default`
- [x] `src-tauri/migrations/` créé (vide avec `.gitkeep`)
- [x] `src-tauri/src/state.rs` créé (AppState vide — complété en F1)
- [x] `src-tauri/src/commands/mod.rs` créé (vide — commands ajoutées en F1)
- [x] Structure `src/` créée : `components/editor/`, `features/`, `stores/`, `lib/`, `hooks/`

### Outillage Claude Code
- [x] `CONTEXT.md` à la racine
- [x] `ROADMAP.md` à la racine (ce fichier)
- [x] `.mcp.json` créé (`context7`, `filesystem`, `github`, `rust-docs`, `tauri`)
- [x] `.claude/settings.json` avec hooks :
  - PostToolUse Edit/Write `.rs` → `cargo fmt`
  - PostToolUse Edit/Write `.ts/.tsx` → `prettier --write`
  - PreToolUse Bash → bloquer `npm install`, `yarn add`, `git push --force`
- [ ] Skills installés : `shadcn`, `tdd`, `vercel-react-best-practices`, `webapp-testing`

### ADRs initiaux rédigés
- [x] `docs/adr/ADR-001.md` — SQLite via sqlx async
- [x] `docs/adr/ADR-002.md` — Placeholder tokenisation Rust-side
- [x] `docs/adr/ADR-003.md` — TM globale cross-projet
- [x] `docs/adr/ADR-004.md` — MVP MV/MZ uniquement
- [x] `docs/adr/ADR-005.md` — lib.rs comme entrée app

### VSCode configuré
- [ ] Extensions : rust-analyzer, tauri-vscode, CodeLLDB, Even Better TOML, Tailwind CSS IntelliSense
- [ ] `.vscode/settings.json` : rust-analyzer linkedProjects, check.command clippy, formatOnSave
- [ ] `.vscode/launch.json` : config debug Rust LLDB + pnpm dev pre-task

---

## F1 — Parsers MV/MZ + UI skeleton
**Statut : [x] Complet**
**Critère de sortie : Ouvrir un jeu MV/MZ et afficher ses segments dans la grille.**

### Engine Layer — RPG Maker MV/MZ
- [-] Intégration lib Rust `rvpacker-txt-rs` — implémentation custom retenue (plus de contrôle)
- [x] `src-tauri/src/engines/mv_mz/extractor.rs` — lecture `data/*.json` du jeu
- [x] `src-tauri/src/engines/mv_mz/injector.rs` — réécriture `data/*.json` traduit
- [x] `src-tauri/src/engines/mv_mz/decryptor.rs` — décryptage `.rpgmvp/.rpgmvo` (XOR + clé `System.json`)
- [x] `src-tauri/src/engines/detector.rs` — détection automatique du moteur (présence de fichiers caractéristiques)
- [x] Tests unitaires Rust : extraction round-trip (extract → inject → même contenu)

### Core Layer — DB + State
- [x] `src-tauri/migrations/0001_initial.sql` — tables `projects`, `source_files`, `segments`
- [x] `src-tauri/src/db/pool.rs` — init SqlitePool, run migrations au démarrage
- [x] `src-tauri/src/state.rs` — `AppState { db: SqlitePool }`
- [x] Setup dans `lib.rs` avec `.manage(AppState { db })`

### Commands Tauri — F1
- [x] `open_project(path: String)` — détecte moteur, extrait segments, insère en DB
- [x] `get_source_files(project_id)` — liste les fichiers d'un projet
- [x] `get_segments(project_id, file_id)` — segments paginés avec statut
- [x] `update_segment(id, target_text)` — save traduction manuelle
- [x] `export_project(project_id)` — réinjection dans les fichiers du jeu

### CAT UI — skeleton
- [x] Layout 3 colonnes : FileTree | SegmentGrid | SidePanel (shadcn `ResizablePanelGroup`)
- [x] `src/components/editor/SegmentGrid.tsx` — TanStack Table + Virtual scroll
  - Colonnes : #, Source, Target (éditable inline), Status, QA Score
  - Row selection, keyboard navigation (Tab pour passer au segment suivant)
- [x] `src/components/editor/FileTree.tsx` — arbre fichiers du projet
- [x] `src/stores/editor.ts` — Zustand : `activeProjectId`, `activeFileId`, `activeSegmentId`
- [x] `src/stores/project.ts` — Zustand : projets ouverts, metadata
- [-] TanStack Query — reporté F2 (useEffect direct utilisé pour F1)
- [x] Import projet (dialog file picker via `tauri-plugin-dialog`)

---

## F2 — LLM pipeline + TM + MVP vendable
**Statut : [x] Complet**
**Critère de sortie : Pré-traduire un jeu MV/MZ avec Ollama local, TM exact match fonctionnelle, QA placeholders live. Ce milestone = MVP vendable.**

### LLM Layer
- [x] `src-tauri/src/llm/tokenizer.rs` — détection et remplacement placeholders par UUID opaques (`⟦ph_001⟧`) — implémenté en F1
- [x] `src-tauri/src/llm/provider.rs` — trait `LlmProvider` + implémentations :
  - [x] `OllamaProvider` (local, priorité MVP)
  - [ ] `OpenAIProvider` (clé user fournie)
  - [ ] `DeepSeekProvider`
- [x] `src-tauri/src/llm/pipeline.rs` — orchestration passes :
  - [x] Passe 1 : translate (avec glossaire injecté dans le prompt)
  - [ ] Passe 2 : review (consistency sur fenêtre de 10 segments)
  - [ ] Passe 3 (optionnel) : tone
- [x] `src-tauri/src/llm/batch.rs` — groupement segments (batch de 20–50), déduplication par hash
- [x] Validation post-LLM : restauration UUIDs → placeholders originaux, rejet si UUID manquant

### Core Layer — TM v1
- [x] `src-tauri/migrations/0002_tm.sql` — table `tm_entries (source_hash, source_text, target_text, engine, lang_pair, confidence)`
- [x] `src-tauri/src/core/tm.rs` — insert, lookup exact match (hash SHA-256 du segment normalisé)
- [x] TM auto-alimentée à chaque validation manuelle de segment

### Core Layer — QA v1
- [x] `src-tauri/src/core/qa.rs` — checks sur chaque segment sauvegardé :
  - [x] Placeholders : tous ceux du source présents dans le target
  - [x] Longueur message box : MV/MZ max ~50 chars/ligne × 4 lignes (configurable)
  - [x] BOM UTF-8 détecté dans le target
- [x] QA score par segment (0–100) retourné avec chaque `update_segment`

### Commands Tauri — F2
- [x] `translate_segments(ids: Vec<String>, provider_config)` — lance pipeline LLM en background (tokio::spawn), émet events de progression
- [x] `get_tm_suggestions(source_text, lang_pair)` — exact match TM
- [x] `get_qa_report(project_id)` — résumé des erreurs QA du projet

### CAT UI — F2
- [x] `src/components/editor/TMPanel.tsx` — sidebar TM : affiche suggestions exact match
- [x] `src/components/editor/QAPanel.tsx` — erreurs QA live sur le segment actif
- [x] `src/stores/llm.ts` — Zustand : `isTranslating`, `translationProgress`, `providerConfig`
- [x] Settings panel : configuration provider LLM (URL Ollama, clé API, modèle)
- [x] Indicateur progression traduction batch (event `h2s://llm/progress`)
- [x] Highlight placeholders dans le source (couleur distincte — composant HighlightedSource, F2 partiel)

### Distribution MVP
- [x] Build Windows `.msi` (GitHub Actions — `tauri-apps/tauri-action@v0`)
- [x] Build Linux `.AppImage` et `.deb` (GitHub Actions — ubuntu-22.04)
- [x] Page de téléchargement GitHub Releases (release draft auto sur push `v*`)

---

## F3 — Polissage + Glossaire + TM fuzzy + beta privée
**Statut : [~] En cours**
**Critère de sortie : 20–30 beta testeurs actifs, feedback collecté, TM fuzzy + glossaire fonctionnels.**

### Engine Layer — VX Ace
- [-] VX Ace reporté — code disponible dans engines/vx_ace/
      mais désactivé. Réactivation prévue post-Wolf RPG stable.

### Core Layer — TM v2 (fuzzy matching)
- [x] Levenshtein distance normalisée sur les segments (seuil 80 % configurable)
- [x] `src-tauri/src/core/tm.rs` — `lookup_fuzzy(source_text, threshold)` → `Vec<TmSuggestion>`
- [x] Export TM au format TMX standard (compatibilité OmegaT/Trados)

### Core Layer — Glossaire v1
- [x] `src-tauri/migrations/0003_glossary.sql` — `glossary_terms (source, target, lang_pair, domain, project_id nullable)`
- [x] `src-tauri/src/core/glossary.rs` — CRUD termes, deux niveaux (global + projet-local)
- [x] Injection des termes dans le prompt de traduction (passe 1)

### CAT UI — F3
- [x] `src/components/editor/GlossaryPanel.tsx` — affichage termes reconnus dans le segment actif
- [x] Highlight inline des termes glossaire dans le source
- [x] TM sidebar avec fuzzy suggestions (% match affiché)
- [x] QA : warning si terme glossaire non respecté dans le target
- [x] Rapport QA exportable HTML

### Persistence projet
- [x] Manifest `.hoshi2star.json` — écriture à l'ouverture, réouverture sans ré-extraction, stats auto-mises à jour
- [x] `translation_secs` per-file in DB (`0004_source_files_translation_secs.sql`) — durée de traduction persistée entre sessions

### Gestion des projets
- [x] `list_projects` Tauri command — liste tous les projets connus en DB (triés par dernière mise à jour)
- [x] `delete_project` Tauri command — supprime projet + fichiers + segments (cascade) + `.hoshi2star.json`
- [x] `ProjectList` panel — affiché à l'ouverture si aucun projet actif, avec boutons Continuer / Supprimer

### Tokenizer — Groupe E
- [x] Patterns `\+word[n]` / `\-word[n]` ajoutés à `RE_MVMZ` (community plugins courants : `\+switch[n]`, `\+variable[n]`, etc.)

### SegmentGrid — UX traduction
- [x] Bouton Traduire par ligne (colonne `actions`) — retraduit un segment individuel sans ouvrir le modal
- [x] Colonne checkbox de sélection — sélectionner ≥2 lignes affiche un bouton "Traduire N lignes" dans la toolbar

### Beta privée
- [ ] Recrutement 20–30 testeurs via Discord fan-trad / F95zone
- [ ] Feedback form intégré à l'app (event `h2s://feedback/submit`)
- [ ] Suivi des bugs critiques (GitHub Issues via `to-issues` skill)

---

## F4 — Wolf RPG (priorité absolue) + diff-aware merge + lancement public
**Statut : [ ] À démarrer**
**Critère de sortie : Lancement payant, Wolf RPG v1/v2 fonctionnel, diff-aware merge disponible.**

> Wolf RPG est la priorité absolue — représente ~40% des jeux JP
> non traduits sur DLsite. RuneTranslate le supporte déjà.
> À compléter avant tout autre moteur.

### Engine Layer — Wolf RPG v1/v2
- [ ] Intégration `rewolf-trans` (TypeScript) via sidecar Tauri ou bindings WASM
- [ ] `src-tauri/src/engines/wolf/extractor.rs` — lecture `.dat/.mps`
- [ ] `src-tauri/src/engines/wolf/decryptor.rs` — décryptage `.wolf` (DXA) via WolfDec/UberWolf bindings
- [ ] `src-tauri/src/engines/wolf/injector.rs` — réécriture + repack `.wolf`
- [ ] Tests round-trip Wolf v1/v2

### Core Layer — Diff-aware merge
- [ ] `src-tauri/src/core/diff.rs` — comparaison `old_project` vs `new_project` (hash par segment)
  - Identique → conserver traduction
  - Modifié → marquer `needs_review` + conserver ancienne trad comme référence
  - Nouveau → status `untranslated`
- [ ] Command `update_project_version(project_id, new_game_path)` — merge intelligent
- [ ] UI : badge "needs review" sur segments modifiés + diff side-by-side

### LLM Layer — Passe tone (optionnel par projet)
- [ ] Config par projet : registre (familier / formel / médiéval / contemporain)
- [ ] Passe 3 activable/désactivable dans les settings projet

### Monétisation
- [ ] Intégration système de licence (Polar.sh ou LemonSqueezy — one-shot 29 $ + 9 $/6 mois)
- [ ] Free tier : MV/MZ uniquement, 1 projet actif, Ollama local, sans TM cross-projet
- [ ] Indie tier (29 $ one-shot) : tous moteurs dispo, TM cross-projet, QA complet, 6 mois updates
- [ ] Lancement public avec prix intro 19 $ pendant 30 jours

---

## F5 — Wolf v3/WolfX + Bakin + consolidation
**Statut : [ ] À démarrer**
**Critère de sortie : Couverture moteurs complète, 500 utilisateurs cible.**

### Engine Layer — Wolf RPG v3/WolfX
- [ ] `src-tauri/src/engines/wolf/decryptor_v3.rs` — support WolfX (hash-based, UberWolf v3.5+)
- [ ] Tests sur jeux Wolf v3.x réels
- [ ] Documentation format WolfX dans `docs/engines.md`

### Engine Layer — RPG Developer Bakin
- [ ] Évaluer adoption DLC Localization Toolkit (SmileBoom) — si > 200 jeux traduits : go
- [ ] `src-tauri/src/engines/bakin/extractor.rs` — via DLC string-table export OU reverse BakinUnpack
- [ ] `src-tauri/src/engines/bakin/injector.rs`
- [ ] Tests Bakin

### Langues sources additionnelles (add-ons)
- [ ] Support Korean source (DLsite Korea) — pack "Korean Source" 9 $
- [ ] Support Chinese source (Wolf RPG CN) — pack "Chinese Source" 9 $
- [ ] Détection automatique langue source dans `detector.rs`

### Collaboration (Git sync)
- [ ] `src-tauri/src/sync/git.rs` — wrapper `git2` crate pour sync projet de traduction
- [ ] Merge de projets `.h2s` entre 2–3 traducteurs
- [ ] Résolution de conflits segment par segment dans l'UI

### Consolidation
- [ ] Skill `improve-codebase-architecture` lancé — audit deep modules
- [ ] ADRs mis à jour pour toutes les décisions prises en F3/F4/F5
- [ ] `docs/engines.md` complet pour tous les moteurs supportés
- [ ] `CONTEXT.md` mis à jour avec les nouveaux patterns

---

## Backlog — idées futures (non planifiées)

- [ ] Traduction d'images (OCR + inpainting + ré-encryption `.rpgmvp`) — complexité élevée
- [ ] Plugin VSCode pour éditer les segments directement dans l'IDE
- [ ] Export format `.po/.pot` (interopérabilité avec d'autres outils CAT)
- [ ] Cloud TM partagé opt-in (anonymisé) — contribution communautaire
- [ ] Support RPG Maker 2000/2003 (vgperson workflow, niche)
- [ ] App mobile companion (lecture seule du projet, validation segments)

---

## Métriques de validation produit

| Milestone | Signal go/pivot |
|-----------|----------------|
| Fin F2 (MVP) | 20+ beta testeurs actifs → continuer F3 |
| Lancement F4 | 50 payants en 3 mois → continuer ; sinon pivoter vers SaaS Bakin uniquement |
| Fin F5 | 200 payants cumulés → revenu d'appoint validé |
| 12 mois | 500 payants = ~2 400–6 000 $/mois récurrent selon mix tiers |
