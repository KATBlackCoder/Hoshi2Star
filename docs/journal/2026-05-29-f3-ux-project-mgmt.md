# Journal — 2026-05-29 — F3 UX : traduction individuelle, sélection, gestion projets, persistance durée, Groupe E

**Phase** : F3
**Durée estimée** : 2h
**Statut** : ✅ Complété

---

## Ce qui a été fait

- Ajout d'un bouton Traduire par ligne dans SegmentGrid (colonne `actions`)
- Ajout d'une colonne checkbox pour la sélection multiple + bouton "Traduire N lignes"
- Ajout du panneau `ProjectList` affiché quand aucun projet n'est ouvert
- Ajout des commandes `list_projects` et `delete_project` côté Rust
- Migration `0004` : colonne `translation_secs` sur `source_files` pour persister la durée par fichier
- Fix : la durée de traduction disparaissait à la réouverture (Zustand éphémère → DB)
- Ajout Groupe E tokenizer : patterns `\+word[n]` / `\-word[n]` (community plugins)
- Fix : erreur de compilation `use of partially moved value: file_id` dans `translate_segments`

## Fichiers créés

- `src-tauri/migrations/0004_source_files_translation_secs.sql`
- `src/components/editor/ProjectList.tsx`

## Fichiers modifiés

- `src-tauri/src/commands/project.rs` — `SourceFile` struct, `get_source_files`, `translate_segments` (timing + fix borrow), `list_projects`, `delete_project`
- `src-tauri/src/lib.rs` — enregistrement `list_projects` et `delete_project`
- `src-tauri/src/llm/tokenizer.rs` — Groupe E ajouté à `RE_MVMZ`, test `test_plugin_codes_tokenized`
- `src/lib/types.ts` — `translationSecs?: number` dans `SourceFile`
- `src/features/editor/columns.tsx` — colonne `select` (checkbox) + colonne `actions` (bouton traduire)
- `src/components/editor/SegmentGrid.tsx` — state de sélection, bouton batch, rechargement `sourceFiles` sur `h2s://llm/completed`
- `src/components/editor/FileTree.tsx` — lecture `file.translationSecs` depuis DB (au lieu de Zustand)
- `src/stores/llm.ts` — suppression `fileTranslationTimes` / `stopTimer` / `clearFileTime`
- `src/stores/project.ts` — fonction `deleteProject`, rechargement `sourceFiles` après `open_project`
- `src/App.tsx` — montage conditionnel `ProjectList` vs `SegmentGrid`
- `src/locales/en.json` / `src/locales/fr.json` — clés `segmentGrid.translateRow`, `translateSelected`, `noModelConfigured`, `projectList.*`

## Fichiers supprimés

- (aucun)

## Dépendances ajoutées

- (aucune)

## Décisions prises

- La durée de traduction par fichier est stockée en DB (`source_files.translation_secs`) plutôt qu'en mémoire Zustand. Raison : Zustand est éphémère — l'info était perdue à chaque réouverture.
- Le bouton Traduire par ligne réutilise directement la `providerConfig` courante du store LLM. Si aucun modèle n'est configuré, un toast avertit l'utilisateur plutôt que d'ouvrir le modal.
- `delete_project` supprime également le `.hoshi2star.json` (best-effort, ignoré si absent) pour éviter une restauration fantôme au prochain `open_project` sur le même chemin.
- Groupe E limité à `[A-Za-z]{1,20}` pour éviter les faux positifs sur du texte libre.

## Problèmes rencontrés

- Erreur `use of partially moved value: file_id` dans `translate_segments` : `if let Some(fid) = file_id` consommait la valeur, puis `file_id` était réutilisé dans le bloc `tokio::spawn`. Corrigé avec `if let Some(ref fid) = file_id`.

## Tâches ROADMAP cochées

- [x] `translation_secs` per-file in DB — durée persistée entre sessions
- [x] `list_projects` Tauri command
- [x] `delete_project` Tauri command
- [x] `ProjectList` panel
- [x] Groupe E tokenizer — `\+word[n]` / `\-word[n]`
- [x] Bouton Traduire par ligne
- [x] Colonne checkbox + bouton "Traduire N lignes"

## Prochaine session

- Beta privée : recrutement testeurs, feedback form
- TM fuzzy : affinage seuil, tests sur vrais jeux
- LLM passe 2 (review / consistency)

---
*Généré par Claude Code — Hoshi2Star*
