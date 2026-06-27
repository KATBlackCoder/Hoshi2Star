# Hoshi2Star — ROADMAP

> Tâches **à venir uniquement**. Pour l'historique des phases complètes, voir `git log`.
> Format : `[ ]` à faire · `[~]` en cours · `[-]` abandonné/reporté

---

## État des phases

| Phase | Objectif | Statut |
|-------|---------|--------|
| **F0** | Setup & fondations | [~] Quasi-complet |
| **F1** | Parsers MV/MZ + UI skeleton | ✅ Complet |
| **F2** | LLM pipeline + TM + MVP | ✅ Complet |
| **F3** | Polissage + Glossaire + TM fuzzy + beta privée | [~] En cours |
| **F4** | Wolf RPG + diff-aware + lancement public | [~] En cours |
| **F5** | Wolf v3/WolfX + Bakin + consolidation | [~] En cours |

## Moteurs

| Moteur | Statut | Priorité |
|--------|--------|---------|
| RPG Maker MV/MZ | ✅ Supporté | — |
| Wolf RPG v1/v2/v3 | ✅ Supporté | — |
| RPG Maker VX Ace | ⏸ Code prêt, désactivé | Post-consolidation |
| RPG Developer Bakin | 🔜 F5 | Basse |
| Autres (Ren'Py, Kirikiri) | 🔜 Backlog | Si demande |

---

## F0 — Setup restant

- [ ] Skills installés : `shadcn`, `tdd`, `vercel-react-best-practices`, `webapp-testing`
- [ ] `.vscode/settings.json` : rust-analyzer linkedProjects, check.command clippy, formatOnSave
- [ ] `.vscode/launch.json` : config debug Rust LLDB + pnpm dev pre-task

---

## F3 — Beta privée

**Critère de sortie : 20–30 beta testeurs actifs, feedback collecté.**

- [ ] Recrutement 20–30 testeurs via Discord fan-trad / F95zone
- [ ] Feedback form intégré à l'app (event `h2s://feedback/submit`)
- [ ] Suivi des bugs critiques (GitHub Issues via `to-issues` skill)

---

## F4 — Lancement public

**Critère de sortie : Lancement payant, diff-aware merge disponible.**

### Core Layer — Project Sync (mise à jour de jeu)

> Permet de continuer un projet existant après la mise à jour du jeu source,
> sans perdre les traductions déjà validées.

**Principe** : `segment_key` (identifiant stable) + `source_hash` (SHA-256).
Au re-import, comparaison par clé puis par hash :

| Cas | Action |
|-----|--------|
| Même clé + même hash | Conserver traduction + statut inchangés |
| Même clé + hash différent | `NeedsReview`, garder ancienne trad comme référence |
| Nouvelle clé | Créer segment `Untranslated` |
| Clé disparue | Archiver (soft-delete — TM garde la traduction) |

- [ ] `src-tauri/migrations/XXXX_segment_key.sql` — colonnes `segment_key TEXT` et `source_hash TEXT`
- [ ] `src-tauri/src/core/diff.rs` — `diff_projects(old, new) -> DiffReport { unchanged, modified, added, removed }`
- [ ] Clés stables par moteur : `engines/mv_mz/segment_key.rs`, `engines/wolf/segment_key.rs`
- [ ] Command `sync_project_version(project_id, new_game_path)` — re-extrait + merge intelligent
- [ ] UI : dialog pré-sync avec résumé `X inchangés · Y modifiés · Z nouveaux · W supprimés`
- [ ] UI : badge `NeedsReview` (⚠) sur segments modifiés + vue diff source old/new side-by-side
- [ ] UI : filtre "Afficher uniquement les segments modifiés" dans SegmentGrid

### LLM Layer — Langues cibles additionnelles

> Prérequis : externalisation des prompts LLM vers TOML (dossier `prompts/translate/` +
> `prompts/glossary/`) — terrain préparé dès F3 : `{{target_lang}}` paramétrable,
> `translate_for(lang)` route vers le bon `.toml`, `lang_code_to_name()` disponible.

- [ ] Settings UI : sélecteur langue cible (FR / EN / ES / DE / …) persisté dans le store Zustand
- [ ] `commands/translate.rs` : recevoir `target_lang` depuis le frontend (plus de hardcode `"en"`)
- [ ] `lang_pair` dynamique (`ja-fr`, `ja-es`, …) propagé dans TM + glossaire
- [ ] `prompts/translate/fr.toml` — règles typographiques FR + register (tutoiement/vouvoiement)
- [ ] `prompts/translate/es.toml` — castillan vs latam + vosotros/ustedes
- [ ] `prompts/translate/de.toml` — majuscules noms communs + Sie/du
- [ ] `prompts/glossary/fr.toml`, `es.toml`, `de.toml` — instructions extraction adaptées si nécessaire

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

**Critère de sortie : Couverture moteurs complète, 500 utilisateurs cible.**

### Wolf RPG v3 / WolfX

- [ ] Documentation workflow WolfX (pré-étape UberWolf) dans `docs/engines.md`
      + message de guidage visible côté UI quand `PossibleWolfX` est détecté

### Engine Layer — RPG Developer Bakin

- [ ] Évaluer adoption DLC Localization Toolkit (SmileBoom) — si > 200 jeux traduits : go
- [ ] `src-tauri/src/engines/bakin/extractor.rs` — via DLC string-table export OU reverse BakinUnpack
- [ ] `src-tauri/src/engines/bakin/injector.rs`
- [ ] Tests Bakin

### Langues sources additionnelles (add-ons)

> Distinct des langues **cibles** (FR/ES/DE) — voir F4. Ici : autres langues *sources* (≠ japonais).

- [ ] Support Korean source (DLsite Korea) — pack "Korean Source" 9 $
- [ ] Support Chinese source (Wolf RPG CN) — pack "Chinese Source" 9 $
- [ ] Détection automatique langue source dans `detector.rs`

### Collaboration (Git sync)

- [ ] `src-tauri/src/sync/git.rs` — wrapper `git2` crate pour sync projet de traduction
- [ ] Merge de projets `.h2s` entre 2–3 traducteurs
- [ ] Résolution de conflits segment par segment dans l'UI

### Consolidation

- [ ] ADRs mis à jour pour toutes les décisions prises en F3/F4/F5
- [ ] `docs/engines.md` complet pour tous les moteurs supportés
- [ ] `CONTEXT.md` mis à jour avec les nouveaux patterns

---

## Métriques de validation produit

| Milestone | Signal go/pivot |
|-----------|----------------|
| Lancement F4 | 50 payants en 3 mois → continuer ; sinon pivoter vers SaaS Bakin uniquement |
| Fin F5 | 200 payants cumulés → revenu d'appoint validé |
| 12 mois | 500 payants = ~2 400–6 000 $/mois récurrent selon mix tiers |
