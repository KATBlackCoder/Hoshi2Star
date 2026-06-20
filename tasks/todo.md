# Tasks — Hoshi2Star

## Complétées (session 2026-06-17/18)

- [x] Wolf extractor skip filters : `X[`/`zz` events, `自動ｼｽﾃﾑ初期化` DB, `@N\n` tokenizer (317 tests)
- [x] `extract_wolf_speakers` Tauri command + bouton "Speakers" dans GlossaryPanel
- [x] `SourceFile.translated_count` / `total_count` + requête SQL `get_source_files`
- [x] `debug_inject_file` Tauri command (Wolf, complétude enforced)
- [x] `scan_font_status` Tauri command + `FontSizeDialog` + `\f[N]` prefix management
- [x] `export_project` étendu avec `fontSize` / `replaceExisting`
- [x] Bouton Debug Inject par fichier dans `FileTree.tsx` (hover, `FlaskConical`)
- [x] Docs CHANGELOG + ROADMAP + architecture.md mis à jour

## Complété — MV/MZ placeholder codes custom + GameTitle (2026-06-19)

- [x] 1. `tokenizer.rs` — Groupe F : `\FF[...]`, `\F[...]`, `\AA[...]` dans RE_MVMZ + RE_MZONLY
- [x] 2. `mv_mz/extractor.rs` — GameTitle + " by Hoshi2Star"
- [x] 3. 6 tests Groupe F + test GameTitle mis à jour
- [x] 4. 323 tests ✓ · clippy ✓

## Complété — Debug Extraction universelle (2026-06-19)

Objectif : rendre le bouton Bug (debug dump JSON) disponible pour tous les moteurs,
pas uniquement Wolf. Le JSON doit avoir un format unifié pour que Claude puisse
l'analyser et identifier ce qui mérite traduction vs ce qui peut être skippé.

- [x] 1. `commands/project.rs` — ajouter `debug_dump_segments` générique (dispatch par moteur)
- [x] 2. `commands/project.rs` — supprimer `debug_dump_wolf_segments` (remplacé)
- [x] 3. `lib.rs` — remplacer `debug_dump_wolf_segments` par `debug_dump_segments`
- [x] 4. `AppToolbar.tsx` — retirer la condition `engine === "wolf"`, appeler `debug_dump_segments`
- [x] 5. Vérification : cargo clippy ✓ · pnpm typecheck ✓

## Backlog

- [ ] Anneaux de progression par fichier dans FileTree (FileTree rings) — `translated_count`/
      `total_count` maintenant disponibles; rend la tâche dormante Tenmon réalisable
- [ ] Documentation workflow WolfX (pré-étape UberWolf) dans `docs/engines.md` +
      message UI quand `PossibleWolfX` est détecté (ROADMAP F5)
- [ ] Recrutement beta testeurs (Discord fan-trad / F95zone) — ROADMAP F3
- [ ] Diff-aware merge (`core/diff.rs`) — ROADMAP F4
