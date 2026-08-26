# Hoshi2Star Operational Patch Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Livrer un mode Patch personnel fiable de bout en bout avant tout travail fonctionnel sur Player ou Images/OCR.

**Architecture:** Le backend Tauri reste la source de vérité pour la détection, l’extraction, la traduction, la QA et l’export. La compatibilité SQLite accepte une base créée par une version plus récente uniquement si le schéma minimal du mode Patch est présent; elle ne réécrit jamais l’historique SQLx. Le frontend orchestre un seul parcours explicite, avec un rail d’inspection progressif et des états locaux testables.

**Tech Stack:** Tauri v2, Rust 2021, SQLx/SQLite, React 19, TypeScript strict, Zustand, TanStack Table/Virtual, Tailwind v4, Vitest/Testing Library, MCP debug bridge.

---

## Périmètre et critères de fin

- Patch couvre : ouvrir/restaurer un projet, parcourir ses fichiers, modifier/traduire des segments, consulter QA/TM/Glossaire et exporter un patch.
- Un projet neuf utilise par défaut `ja → fr`; les projets existants conservent leur paire.
- Une base SQLx possédant des migrations futures démarre sans modification de son historique si son schéma Patch minimal est compatible.
- Les erreurs d’ouverture, chargement, traduction, sauvegarde et export sont visibles dans le contexte de l’action.
- Player et Images/OCR restent des placeholders sans backend ni dépendances supplémentaires.
- La phase est terminée quand les tests Rust/frontend, le build, le lint sans erreur et un parcours MCP Tauri passent.

## Structure de fichiers

- Modify: `src-tauri/src/db/pool.rs` — stratégie de migration et validation du schéma.
- Modify: `src-tauri/migrations/0006_project_languages.sql` — noms canoniques des langues.
- Modify: `src-tauri/src/commands/project.rs` — cycle projet et langues.
- Modify: `src-tauri/src/commands/translate.rs` — contexte de traduction.
- Modify: `src-tauri/src/commands/qa.rs` — contexte QA.
- Modify: `src-tauri/src/commands/export.rs` — métadonnées d’export.
- Modify: `src-tauri/tests/e2e_project_flow.rs` — parcours natif complet.
- Create: `src/components/shell/InspectorRail.tsx` — inspection TM/QA/Glossaire.
- Modify: `src/App.tsx` — composition du workspace Patch.
- Modify: `src/stores/ui.ts` — état du rail.
- Modify: `src/components/editor/SegmentGrid.tsx` — états et feedback de grille.
- Modify: `src/components/AppToolbar.tsx` — actions Patch contextualisées.
- Test: fichiers `*.test.tsx` adjacents et tests Rust des modules concernés.

### Task 1: Accepter sans perte les bases futures compatibles

- [ ] Écrire un test qui crée une base à jour, altère le checksum de la migration 6, ajoute une migration 7 inconnue, puis vérifie que `init()` redémarre et conserve les données.
- [ ] Faire échouer le test avec le migrateur SQLx strict actuel.
- [ ] Détecter la version embarquée maximale et la version appliquée maximale.
- [ ] Si la base est plus récente, vérifier les tables et colonnes minimales puis sauter le migrateur sans modifier `_sqlx_migrations`.
- [ ] Si la base n’est pas plus récente, conserver la validation stricte SQLx.
- [ ] Vérifier avec `cargo test db::pool`.

### Task 2: Unifier le stockage des langues de projet

- [ ] Modifier la migration 6 neuve pour créer `source_language` et `target_language` avec les défauts historiques `ja` et `en`.
- [ ] Garder les champs API `sourceLang`/`targetLang` via les alias SQL `AS source_lang` et `AS target_lang`.
- [ ] Remplacer les lectures/insertions backend dans projet, traduction, QA et export.
- [ ] Utiliser `COALESCE(..., 'ja'/'en')` pour les anciennes lignes nulles.
- [ ] Vérifier les tests de création `ja → fr`, de restauration et de langues historiques.

### Task 3: Verrouiller le parcours natif Patch

- [ ] Étendre le test d’intégration pour ouvrir un fixture MV/MZ, modifier un segment, exécuter la QA et exporter l’archive.
- [ ] Ajouter un test de restauration garantissant l’absence de ré-extraction destructive.
- [ ] Vérifier que l’archive contient les fichiers injectés et que les sources originales restent inchangées.
- [ ] Exécuter `cargo test --manifest-path src-tauri/Cargo.toml` et `cargo clippy --manifest-path src-tauri/Cargo.toml --lib --bins -- -D warnings`.

### Task 4: Remplacer l’inspecteur permanent par un rail progressif

- [ ] Écrire les tests d’ouverture, fermeture, onglets et attributs ARIA du rail.
- [ ] Afficher un seul outil parmi TM, QA et Glossaire dans une largeur redimensionnable.
- [ ] Mémoriser `inspectorOpen` et `inspectorTab` dans le store UI sans changer automatiquement le mode.
- [ ] Ajouter un bouton 40×40 px dans la barre Patch et conserver le focus clavier.
- [ ] Vérifier les rayons concentriques, les transitions ciblées et `prefers-reduced-motion`.

### Task 5: Rendre les états Patch explicites

- [ ] Ajouter aux chargements de fichiers/segments un état d’erreur avec action Réessayer.
- [ ] Distinguer projet vide, fichier non sélectionné et fichier sans segment.
- [ ] Afficher localement l’échec de sauvegarde/traduction/export et garder les toasts pour le résultat global.
- [ ] Ajouter les traductions françaises et anglaises correspondantes.
- [ ] Vérifier par tests Testing Library avec IPC Tauri simulé et nettoyage des mocks.

### Task 6: Valider la tranche opérationnelle

- [ ] Exécuter `pnpm typecheck`, `pnpm test`, `pnpm lint` et `pnpm build`.
- [ ] Exécuter les tests et Clippy Rust.
- [ ] Lancer Tauri avec une base temporaire et parcourir Bibliothèque → Patch → fichier → segment → paramètres via MCP.
- [ ] Lancer Tauri avec une copie temporaire de la base future et vérifier le démarrage sans changement de checksum/historique.
- [ ] Capturer l’écran Patch final et documenter les limites restantes avant Player.
