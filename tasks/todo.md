# Tasks — Hoshi2Star

> **Remédiation audit 2026-07-01 — avancement**
> Ordre : 1 → 2 → 3 → 4 → 5 → 7 → 6 → 8 → 9 (`docs/audit-remediation-plan-2026-07-01.md`).
> - Phase 1 : ✅ **terminée & mergée sur `main`** (2026-07-01).
> - Phase 2 : ✅ **terminée & mergée sur `main`** (2026-07-01).
> - 👉 **PROCHAINE : Phase 3 — 🟠 robustesse des promesses UI.**

## ✅ Phase 2 — Éliminer les paniques des parsers binaires Wolf (P1, crash) — TERMINÉE

> Plan de `docs/audit-remediation-plan-2026-07-01.md` §Phase 2 + complément §2.
> **Vérifié sur le code réel (2026-07-01)** : tous les sites sont encore aux lignes de
> l'audit (le code n'a pas bougé). Tous les champs `DxFileEntry` sont `u64` attaquant
> (`legacy_xor.rs:62-66`) → les casts `as usize` + additions brutes peuvent réellement
> wrapper même en 64-bit. Branche : `fix/wolf-parser-panics`.
> **NE PAS toucher** : les `catch_unwind` d'`extractor.rs` (garde-fou voulu).
> Convention erreur : réutiliser `DecryptorError::HeaderTooShort` (déjà utilisé pour
> toutes les bornes voisines :555/582/1185/1196…) — pas de nouveau variant.

### Étape A — Racine : allocation Huffman non bornée (`legacy_xor.rs:731`)
- [x] Test rouge : `huffman_decode(&[0xFF;16])` (orig_bits=64, orig_size≈u64::MAX) →
      panique « capacity overflow » prouvée
- [x] Fix : const `MAX_DECODE_OUTPUT = 1 GiB` (partagée Huffman/LZ) → `return None`
      (les appelants mappent déjà `None` → `Err`)
- [x] Même classe : `lz_decode` — `dest_size` plafonné (rouge prouvé : retournait
      `Some(vec 4 GiB)`)

### Étape B — Slice TOC non bornée (`legacy_xor.rs:591`)
- [x] Test rouge : `make_v5_archive` + helper `patch_v5_toc` (XOR symétrique) avec
      `name_offset=0xFFFF` → panique OOB prouvée
- [x] Fix : `usize::try_from(name_offset)` + borne `ns ≤ toc_data.len()` → `Err(HeaderTooShort)`

### Étape C — Slices Huffman v8 (`legacy_xor.rs:1214/1216` + `:1267/1269`)
- [x] Tests rouges : appels directs `extract_v8_huffman_only`/`assemble_v8_lz_stream`
      avec blob `[0u8;16]` (décode vers tampon VIDE) + `unpacked/press_sz > 2*huff_kb`
      → panique « range end index 1024 out of range for slice of length 0 » prouvée ×2
- [x] Fix : `decoded.len() < huff_kb * 2` → `Err(HeaderTooShort)` dans les 2 fonctions

### Étape D — Tier arithmétique : additions d'offsets en `checked_add`
- [x] `:541` (`saturating_add`), `:554` (check u64 `index_offset+index_size` avant cast),
      `:580` (checked u64 + `try_from`), `:1387` (idem v8)
- [x] Sites frères mêmes fonctions : `:1185`, `:1204-1205`, `:1238`, `:1257-1258`,
      `:1396`, `:1419` + casts `u64→usize` non tronquants (`unp`, `pz`, `hz`) +
      `key_offset + huff_sz` (u64) en `checked_add`
- [x] **Ajouts vérifiés hors liste audit (même classe, même chemin de dispatch v8)** :
      `read_original_name` (`:1016/:1020` saturating), `build_per_file_key_str`
      (`:1049` saturating), `find_parent_dir` (`:992` `checked_mul`+`checked_add`,
      skip du dir forgé)
- [x] Tests rouges : v6 `data_offset=u64::MAX` via `patch_v6_toc` (panique add overflow
      prouvée) + `extract_v8_huffman_only(file_start≈usize::MAX)` (panique add prouvée).
      `:554` non testable en rouge (champs v5/v6 u32 → pas de wrap 64-bit) → défensif.

### Étape E — `v3_format/map.rs:355/369` : multiplication tiles non vérifiée
- [x] Test rouge : dump d'une map valide + patch `width=0x4000_0000, height=4` →
      « attempt to multiply with overflow » prouvé (map.rs:369)
- [x] Fix : `tile_data_len(width,height,layer_cnt)` en `checked_mul` u64 → nouveau
      variant `V3FormatError::TileSizeOverflow` ; branche utf8 : `read_bytes` (borné,
      sans alloc) AVANT `Vec::with_capacity(tile_len)`

### Étape F — `dat_parser.rs` : allocations pilotées par l'entrée
- [x] `read_bytes` : refuse `n > bytes restants` AVANT `vec![0u8; n]` (couvre aussi
      `n*4`/`cnt*4`, passés en `saturating_mul`)
- [x] Helper `bounded_cap(cursor, count)` (`count.min(remaining/4)`) appliqué aux 5
      `with_capacity` attaquants : `parse_project` ×3 + `parse_dat_types` `fields_size`
      + `data_count`
- [x] Tests rouges : `read_bytes(n=usize::MAX)` → « capacity overflow » prouvé ;
      `type_count=u32::MAX` → **SIGABRT alloc 309 Go** prouvé (tuait tout le process !) ;
      `fields_size=u32::MAX` → **SIGABRT alloc 137 Go** prouvé

### Vérification (gate obligatoire)
- [x] 11 tests rouges prouvés AVANT fix (9 paniques + 2 SIGABRT), 11/11 verts après
- [x] Non-régression fixtures réelles : `test_real_honoka_*` (v2 + archive v8 DXA) +
      `test_real_inko_*` (v3.5) tous verts
- [x] Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ (seul warning :
      toolchain `pclmul` pré-existant, hors code) · `cargo test` = **363 pass / 0 fail / 4 ignored**
- [x] CHANGELOG.md (Fixed)
- [x] docs/architecture.md : PAS touché — aucun changement d'architecture (fns internes
      durcies ; seul ajout : variant interne `V3FormatError::TileSizeOverflow`, pas un type IPC)
- [x] docs/journal/ : entrée de session
- [x] tasks/todo.md : coché

**Phase 2 : livrée — 11 sites de panique/abort éliminés (legacy_xor.rs, v3_format/map.rs,
dat_parser.rs), `catch_unwind` d'extractor.rs intacts, prouvé rouge→vert site par site.**

## ✅ Phase 1 — Réinjecter les Common Events Wolf à l'export (P0, perte de données) — TERMINÉE

> Plan de `docs/audit-remediation-plan-2026-07-01.md` §Phase 1. Vérifié sur le code réel
> (2026-07-01). Le bug est confirmé : `inject_all_to_memory`/`inject_all` (injector.rs:483/521)
> ne matchent que `MapData`/`Database`, `_ => {}` (514/568). Aucun `inject_common_events`.

### Étape 1 — Verify read-only + test rouge (priorité absolue)
- [x] Pas de fixture Wolf réelle dans ce checkout (`test/性処理係のある学校` = MV/MZ) →
      reproduction via CE v3 synthétique (`CommonEventsV3::synthetic_messages`), pas de fixture chiffrée
- [x] Test **rouge** via le **regroupement** (extract→`injection_bucket`→`inject_all_to_memory`)
      avec 2 event_names → prouvé rouge (compile-fail sur `injection_bucket` absent, puis
      assert perte). `test_common_events_translations_survive_export`

### Contraintes de correction (silencieuses si ratées — cf. advisor)
- [x] **C1 — Bucket unique** : `injection_bucket()` collapse tous les `CommonEvents/*` en
      `COMMON_EVENTS_BUCKET = "CommonEvents/CommonEvent"`. Les 2 sites de bucketing d'`export.rs`
      (`collect_wolf_zip_entries` + `debug_inject_file`) l'utilisent. Clé complète conservée.
- [x] **C2 — Parité de clés** : injecteur régénère `CommonEvents/{event_name}/{event_idx}/{cmd_idx}`
      (+`/choices/{choice_idx}`), pas de `pages`, `event_name = event.event_name()` (v2) / `event.name` (v3).
- [x] **C3** : test rouge passe par `injection_bucket` + `inject_all_to_memory` (2 event_names).

### Implémentation
- [x] `inject_common_events(bytes, translations, version)` : v2 → `common_events_parser::parse_bytes`
      (catch_unwind) + `patch_common_events_strings` (remplacements ordonnés, dst=src si non traduit) +
      `splice_wolf_strings` (splice partagé extrait de `patch_mps_strings`). v3 → `inject_common_events_v3`
      (`is_lz4_v3` → `decompress` → `CommonEventsV3::parse` → mute `string_args` → `dump` → `recompress`).
- [x] Arm `"CommonEvents"` dans **`inject_all_to_memory` ET `inject_all`** → `load_common_event_bytes`,
      écrit/emet `Data/BasicData/CommonEvent.dat`.
- [x] `export.rs` : les 2 sites routent via `injection_bucket`.

### Vérification
- [x] **v3 (Inko)** vert : `test_common_events_translations_survive_export` (2 CE, both survive) via `CommonEventsV3::synthetic_messages`
- [x] **v2 (Honoka)** vert : fixture v2 synthétique construite à la main (`make_v2_common_event_dat`) →
      `test_v2_common_event_fixture_extracts` + `test_v2_common_events_translations_survive_export` (2 CE) +
      `test_v2_common_events_identity` (byte-exact, doublons → pas de dé-alignement du splice)
- [x] `test_injection_bucket_collapses_common_events`
- [x] **Vérif fichiers réels** (fixtures fournies par l'utilisateur, renommées en `Densyanai_Inko_ver2.0` /
      `月咲流ホノカver1.03` pour matcher les 13 refs de tests existantes) :
      `test_real_honoka_common_events_inject` (v2 : identity byte-exact + trad round-trip) +
      `test_real_inko_common_events_inject` (v3.5 : identity payload décompressé + trad round-trip)
- [x] Gate : `pnpm typecheck` ✅ · `cargo clippy -- -D warnings` ✅ · `cargo test` = **352 pass / 0 fail / 4 ignored**
- [x] `docs/architecture.md` : section `wolf/injector.rs` + date 2026-07-01
- [x] CHANGELOG.md (Fixed)

**Phase 1 : livrée — v2 (Honoka) ET v3 (Inko) vérifiés sur fixtures synthétiques ET sur les jeux réels. Commitée + mergée sur `main` (2 commits + merge --no-ff).**

### Résolu par les fixtures fournies
- [x] Les 2 tests `test_real_inko_*_round_trip` qui paniquaient (fixture Inko absente) passent maintenant
      (fixtures présentes). Note portabilité : ils paniquent toujours (au lieu de skip) sur un checkout sans
      fixtures — aligner sur `if !path.exists() { return; }` reste un petit nettoyage optionnel.

---

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
