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

## En cours — Stats de segments (2026-06-26)

Trois emplacements : toast post-extraction · barre dans ProjectList · % dans toolbar pill.

- [x] 1. Rust `domain/types.rs` — ajouter `translated_count` + `needs_review_count` à `ProjectStats`
- [x] 2. Rust `commands/project.rs` — étendre la query SQL de `get_project_stats` (5 sous-requêtes)
- [x] 3. TS `lib/types.ts` — ajouter interface `ProjectStats` partagée
- [x] 4. TS `stores/project.ts` — ajouter `activeProjectStats`, fetch après open, toast si `!wasRestored`
- [x] 5. TS `useAppHandlers.ts` — supprimer interface locale, importer depuis `lib/types`
- [x] 6. TS `AppToolbar.tsx` — afficher `37%` dans la pill projet
- [x] 7. TS `ProjectList.tsx` — fetch stats par carte, afficher barre + compteurs
- [x] 8. i18n `en.json` + `fr.json` — ajouter clé `project.extracted`
- [x] 9. Vérification : `cargo clippy` ✓ · `pnpm typecheck` ✓

## En cours — Tokenizer Groupe G + \# (2026-06-26)

- [x] 1. `tokenizer.rs` — ajouter Groupe G (`\\n<[^>]+>`) dans RE_MVMZ et RE_MZONLY
- [x] 2. `tokenizer.rs` — ajouter `\#` (échappé) dans Groupe B des deux regex
- [x] 3. `tokenizer.rs` — 4 tests : tokenize `\n<Name>`, tokenize `\#`, round-trip, pas de conflit avec `\n[N]`
- [x] 4. Vérification : 27 tests ✓ · clippy ✓

## Complété — Export ZIP (2026-06-26)

- [x] 1. `Cargo.toml` — ajouter crate `zip = "2"`
- [x] 2. `engines/mv_mz/injector.rs` — ajouter `inject_to_bytes(raw_json, pairs) -> Vec<u8>`
- [x] 3. `engines/wolf/injector.rs` — ajouter `inject_all_to_memory(...) -> Vec<(String, Vec<u8>)>`
- [x] 4. `commands/export.rs` — refactorer `export_project` → zip `hoshi2star.zip` + `collect_wolf_zip_entries` + `write_zip`
- [x] 5. `commands/export.rs` — return type `Result<String, String>` (retourne le chemin zip)
- [x] 6. `useAppHandlers.ts` — `invoke<string>`, toast description = chemin zip
- [x] 7. i18n `en.json` + `fr.json` — messages mis à jour (ZIP / hoshi2star.zip)
- [x] 8. Vérification : clippy ✓ · typecheck ✓ · 336 tests ✓ (2 échecs pré-existants Inko)

## Complété — Font size MV/MZ (\\FS[N]) (2026-06-27)

`\FS[N]` MZ-natif (Groupe C). `\f[N]` Wolf-only. Dialog gate wolf/mv_mz.
Hint affiché pour mv_mz : nécessite MZ ou plugin messages pour MV.
clippy ✓ · typecheck ✓ · 336 tests ✓ (2 Inko pré-existants)

- [x] 1. `export.rs` — `RE_FONT_PREFIX_MVMZ` + `engine: String` dans `FontScanResult`
- [x] 2. `export.rs` — `apply_font_prefix(text, n, replace, engine)` engine-aware
- [x] 3. `export.rs` — `scan_font_status` fetch engine depuis DB, retourne engine
- [x] 4. `export.rs` — `persist_font_size(…, engine)` passe engine à apply
- [x] 5. `export.rs` — `export_project` fetch engine avec game_path
- [x] 6. `export.rs` — `debug_inject_file` utilise engine (plus `_engine`)
- [x] 7. `lib/types.ts` — `engine: string` dans `FontScanResult`
- [x] 8. `useAppHandlers.ts` — gate wolf/mv_mz uniquement
- [x] 9. `FontSizeDialog.tsx` — `code` calculé, hint mv_mz
- [x] 10. `en.json` + `fr.json` — `{{code}}` interpolé + clé `hintMvMz`
- [x] 11. Vérification ✓

## En cours — Externalisation des prompts LLM vers TOML

Objectif : sortir les prompts hardcodés de `provider.rs` et `glossary.rs` vers des
fichiers `.toml` embarqués à la compilation via `include_str!()`. Structure dossier
dès maintenant pour accueillir les langues cibles futures sans refactor.

**Architecture :**
```
src-tauri/prompts/
  translate/
    default.toml    ← fallback générique (ja→en aujourd'hui)
  glossary/
    default.toml    ← fallback générique
```
Quand on ajoutera FR : créer `translate/fr.toml` + bras `"fr"` dans le `match`.

**Variables dans les templates :**
- `translate/default.toml` : `{{source_lang}}` / `{{target_lang}}` / `{{glossary}}` / `{{segments}}`
- `glossary/default.toml` : `{{target_lang}}` / `{{source_list}}`
- Les valeurs sont des noms complets (`"Japanese"`, `"English"`) — pas des codes courts (`"ja"`, `"en"`)
  → `translate.rs` conserve `"ja"`/`"en"` en interne ; `prompts.rs` expose `lang_code_to_name()`

**Fichiers créés :**
- `src-tauri/prompts/translate/default.toml`
- `src-tauri/prompts/glossary/default.toml`
- `src-tauri/src/llm/prompts.rs`

**Fichiers modifiés :**
- `src-tauri/Cargo.toml` — ajouter `toml = "0.8"`
- `src-tauri/src/llm/mod.rs` — `pub mod prompts`
- `src-tauri/src/llm/provider.rs` — remplacer `format!()` hardcodé
- `src-tauri/src/core/glossary.rs` — remplacer strings hardcodées

- [x] 1. `Cargo.toml` — ajouter dépendance `toml = { version = "0.8", features = ["parse"] }`
- [x] 2. Créer `src-tauri/prompts/translate/default.toml` — system + user avec variables ci-dessus
- [x] 3. Créer `src-tauri/prompts/glossary/default.toml` — system + user avec variables ci-dessus
- [x] 4. Créer `src-tauri/src/llm/prompts.rs` :
         · `PromptTemplate { system, user }` + `render(part, vars)`
         · `fn lang_code_to_name(code: &str) -> &str` (`"ja"` → `"Japanese"`, `"en"` → `"English"`, …)
         · `LazyLock` `TRANSLATE_DEFAULT` + `GLOSSARY_DEFAULT`
         · `fn translate_for(target_lang: &str) -> &'static PromptTemplate` (fallback default)
         · `fn glossary_for(target_lang: &str) -> &'static PromptTemplate` (idem)
- [x] 5. `src-tauri/src/llm/mod.rs` — ajouter `pub mod prompts`
- [x] 6. `src-tauri/src/llm/provider.rs` — appeler `prompts::translate_for(&context.target_lang)`,
         passer `lang_code_to_name(source_lang)` et `lang_code_to_name(target_lang)` au `render()`
- [x] 7. `src-tauri/src/core/glossary.rs` — appeler `prompts::glossary_for(lang_target)`,
         passer `lang_code_to_name(lang_target)` au `render()`
- [x] 8. Vérification : clippy ✓ · 343/349 tests ✓ (2 échecs Inko pré-existants, 4 ignored)

## Backlog

- [ ] Anneaux de progression par fichier dans FileTree (FileTree rings) — `translated_count`/
      `total_count` maintenant disponibles; rend la tâche dormante Tenmon réalisable
- [ ] Documentation workflow WolfX (pré-étape UberWolf) dans `docs/engines.md` +
      message UI quand `PossibleWolfX` est détecté (ROADMAP F5)
- [ ] Recrutement beta testeurs (Discord fan-trad / F95zone) — ROADMAP F3
- [ ] Diff-aware merge (`core/diff.rs`) — ROADMAP F4
