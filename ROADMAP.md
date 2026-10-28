# Hoshi2Star — ROADMAP

> Tâches à venir, puis ce qui a été livré depuis la dernière mise à jour. Détail : `git log`, `docs/journal/`.
> Format : `[ ]` à faire · `[~]` en cours · `[x]` fait · `[-]` abandonné/reporté
>
> Mis à jour le 2026-10-28 à partir du code. Orientation : **usage personnel**
> (`docs/superpowers/plans/2026-08-25-hoshi2star-lean-multilingual-providers.md`) : monétisation, formules
> payantes, bêta publique, métriques commerciales, collaboration Git et passe de ton sont retirées.

---

## État des phases

| Phase | Objectif | Statut |
|-------|---------|--------|
| **F0** | Setup & fondations | [~] Quasi-complet |
| **F1** | Parsers MV/MZ + UI skeleton | ✅ Complet |
| **F2** | LLM pipeline + TM + MVP | ✅ Complet |
| **F3** | Polissage + terminologie + TM fuzzy + QA | ✅ Complet (bêta retirée) |
| **F4** | Wolf RPG + mise à jour de jeu + langues + fournisseurs | [~] En cours |
| **F5** | WolfX + Bakin + consolidation | [~] En cours |

## Moteurs

| Moteur | Statut | Priorité |
|--------|--------|---------|
| RPG Maker MV/MZ | ✅ Supporté | — |
| Wolf RPG v2/v3 | ✅ Supporté : archives DXA v6/v8 (lues et reconstruites) ou `Data/` déchiffré | — |
| Wolf RPG 1.x, 3.31+, WolfX | ⚠ Refusés à l'ouverture (archive DXA v5, nouveau chiffrement, WolfX) : passer par UberWolf | — |
| RPG Maker VX Ace | ⏸ Code prêt, désactivé (détection commentée) | Post-consolidation |
| RPG Developer Bakin | 🔜 Étude faite, pas de code | Basse |
| Autres (Ren'Py, Kirikiri) | 🔜 Backlog | Si demande |

---

## F0 — Setup restant

- [~] Skills installés : `shadcn` fait ; `tdd`, `vercel-react-best-practices`, `webapp-testing` absents
- [ ] `.vscode/settings.json` : rust-analyzer linkedProjects, check.command clippy, formatOnSave (local, ignoré par Git)
- [ ] `.vscode/launch.json` : config debug Rust LLDB + pnpm dev pre-task (local, ignoré par Git)

---

## F4 — Wolf, mise à jour de jeu, langues, fournisseurs

### Mise à jour de jeu (resync)

> Fait autrement que prévu : pas de colonnes `segment_key`/`source_hash` ni de `core/diff.rs`.
> Les segments restent identifiés par leur `json_key` ; `engines/mv_mz/resync.rs` réaligne les
> pointeurs (LCS par liste de commandes) sur l'instantané d'origine du jeu.

- [x] Instantané d'origine des fichiers du jeu et garde d'intégrité à l'export (`core/originals.rs`,
      `engines/mv_mz/integrity.rs`, `ExportIntegrityDialog.tsx`)
- [x] Commande `resync_project(project_id, apply)` avec aperçu puis application (`commands/resync.rs`)
- [x] Dialogue de résumé avant application : déplacés · ajoutés · disparus · sources changées · orphelins
      (`ResyncDialog.tsx`)
- [x] Traductions perdues conservées : table `orphaned_segments` (migration 0012) + TM
- [ ] Source changée ⇒ segment `needs_review` avec l'ancienne traduction en référence (aujourd'hui : nouvelle
      ligne non traduite, ancienne traduction en orphelin)
- [ ] Vue de comparaison ancienne/nouvelle source
- [ ] Filtre « segments modifiés par la mise à jour » dans `SegmentGrid`
- [ ] Lecture et restauration des orphelins dans l'interface
- [ ] Resync Wolf (MV/MZ seulement ; garde « jeu mis à jour » Wolf = lot séparé, copie d'origine)

### Langues

- [x] Paire de langues par projet (`projects.source_language`/`target_language`, migration 0006), choisie à la
      création ; langue cible par défaut dans les réglages (`LanguageSettings.tsx`)
- [x] `lang_pair` dynamique dans la TM et la terminologie (`llm/pipeline.rs`, migration 0009)
- [ ] Prompts par langue cible : `src-tauri/prompts/translate/{fr,es,de}.toml` ; `translate_for` renvoie
      toujours `default.toml` (`llm/prompts.rs`)
- [ ] Prompts de terminologie par langue (`terminology_for`, même limite)
- [ ] Changer la paire de langues d'un projet existant (aucune commande aujourd'hui)
- [ ] Repli incohérent de la langue cible : `"fr"` dans `commands/translate.rs:143` contre `'en'` en SQL

### Fournisseurs LLM

- [x] Un seul protocole OpenAI-compatible (`OpenAiCompatibleProvider`, `llm/provider.rs`) : `/v1/chat/completions`,
      `/v1/models`, `Authorization: Bearer` ; Ollama passe par son `/v1`
- [x] Préréglages côté front (`PROVIDER_PRESETS` : Ollama, LM Studio, Hugging Face, OpenAI, personnalisé),
      clé masquée, saisie libre du modèle si la liste est vide
- [x] Clé API gardée en mémoire seulement, jamais écrite sur disque
- [x] Message d'erreur générique (URL + erreur) ; arrêt propre sur panne du fournisseur et annulation (R1)
- [ ] Retirer les clés i18n mortes `llmModal.urlLabel`/`llmModal.modelError` et le texte « Ollama » restant
      dans `llmRun.stopHint`

---

## F5 — WolfX, Bakin, consolidation

### Wolf RPG v3 / WolfX

- [~] Guidage WolfX : texte d'erreur « utiliser UberWolf » côté Rust (`PossibleWolfX`) ; message d'interface
      encore générique (`wolfUnreadableArchive`) ; documentation à écrire
- [ ] Essai en jeu de l'export de l'app sur Wolf 3.658
- [ ] Coupe de ligne après un trait d'union (« H- ⏎ scenes ») dans la recoupe des messages

### Engine Layer — RPG Developer Bakin

- [x] Étude préalable (handoff 2026-10-26, `RAE-PLAN/bakin/`) : livraison en archive reconstruite préférée
- [ ] Évaluer adoption DLC Localization Toolkit (SmileBoom)
- [ ] `src-tauri/src/engines/bakin/extractor.rs` — via DLC string-table export OU reverse BakinUnpack
- [ ] `src-tauri/src/engines/bakin/injector.rs`
- [ ] Tests Bakin

### Langues sources additionnelles

- [~] Coréen et chinois sélectionnables comme langue source ; découpage terminologique japonais/anglais/français
      seulement ; Wolf en Shift-JIS ou UTF-8 (pas de GBK ni CP949)
- [ ] Détection automatique de la langue source dans `detector.rs`

### Consolidation

- [ ] ADR-008 (usage personnel, fournisseur unique) et ADR pour les décisions F4/F5 : terminologie, resync et
      orphelins, reconstruction d'archive Wolf, décision WolfX, export d'essai (dernier ADR : ADR-007)
- [ ] `docs/engines.md` : aujourd'hui seulement `docs/engines/` MV/MZ ; Wolf à documenter
- [ ] `CONTEXT.md` à jour (terminologie, resync, mise en page, `WolfSources`)
- [ ] `docs/architecture.md` : retirer la monétisation et le support natif WolfX annoncés

---

## Performance — optimisations conditionnelles (sur signal, pas prématuré)

> **Phase 9 du plan de remédiation audit** (`docs/audit-remediation-plan-2026-07-01.md`). À déclencher
> uniquement sur un **point chaud mesuré**, jamais à l'aveugle.

- [x] `llm/pipeline.rs` `persist_batch_results` — une seule transaction (un `UPDATE` par ligne à l'intérieur)
- [x] `llm/pipeline.rs` `translate_batch` — `tm::lookup_exact_many` : `WHERE source_hash IN (...)` en une requête
- [x] Extraction — `insert_segments` en `INSERT` multi-valeurs par paquets de 90 (`commands/project.rs`)
- [ ] **NE PAS** optimiser `core/tm.rs` `lookup_fuzzy` (scan O(n)) avant ~5k entrées (index trigram au backlog)
- [x] `update_segment` : recalcul des stats unique (`manifest::refresh_stats`, Phase 6)

---

## Livré depuis juillet 2026 (absent de l'ancienne roadmap)

- [x] **Terminologie** (remplace le glossaire) : `core/terminology/`, `src/features/terminology/`, libellés fr/en
- [x] **QA et nouvel essai sémantique** : `core/qa.rs`, `llm/semantic_retry.rs`, rapport HTML `core/report.rs`
- [x] **Pilote A/B MV/MZ** sur base jetable : `commands/pilot.rs`, `PilotComparisonWorkspace.tsx`
- [x] **Packs d'échange `.h2s`** (export, import avec aperçu, politiques de fusion, sauvegarde automatique) :
      `core/h2s_pack.rs`, `commands/pack.rs`
- [x] **Export d'essai** pour tester en jeu avant la fin : `export_debug_project`
- [x] **Mise en page MV/MZ** (ajustement et retour à la ligne) : `engines/mv_mz/layout/`, `TextFitDialog.tsx`
- [x] **Mise en page Wolf** et retours à la ligne souples côté LLM : `engines/wolf/layout/`,
      `engines/wolf/line_breaks.rs`, taille de police du jeu proposée à l'export
- [x] **Textes optionnels MV/MZ** (plugins.js, Keke, note tags) : `commands/coverage.rs`
- [x] **Bascule de langue du jeu à l'export** : `engines/mv_mz/locale.rs`
- [x] **Archives Wolf DXA v6/v8** lues et reconstruites, archive avant fichiers libres : `engines/wolf/sources.rs`,
      `engines/wolf/decrypt/writer.rs` ; formats v3 (`engines/wolf/v3_format/`)
- [x] **Contexte de scène et d'orateur** dans les prompts (`llm/context/`) et découpage adaptatif des lots
      (`llm/split.rs`)
- [x] **Recherche globale** : `commands/search.rs`, `GlobalSearchResults.tsx`
- [x] **Mise à jour automatique** : `tauri_plugin_updater`, ADR-007
