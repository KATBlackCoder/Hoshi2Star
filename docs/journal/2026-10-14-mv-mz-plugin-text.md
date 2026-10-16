# Journal — 2026-10-14 — MV/MZ : textes de plugins et listes System, sans casser le jeu

**Phase** : F4 (consolidation moteur MV/MZ)
**Durée estimée** : ~3 sessions (2026-10-13 → 2026-10-14)
**Statut** : ✅ Complété (branche `feat/mv-mz-plugin-text`, non mergée)

---

## Ce qui a été fait

Déclencheur : 魔術師「恵」と禁断の魔術 (MZ) gardait du japonais dans le menu (`エネミー図鑑`), les
Options (`戦闘ボイス（ＯＮ推奨）`), le bestiaire, le log et les commandes de combat. L'extraction MV/MZ ne
lisait que `data/*.json`. Exigence : conséquences détaillées avant activation, et aucune casse du jeu.

- Phase 0 — `serde_json` `preserve_order` + `float_roundtrip` explicites ; `AppState.data_dir` ;
  découpage des `IN (...)` à 400.
- Phase 1 — Tokenizer par segment (`TokEngine::for_segment`, `MzOnly` pour `system_term` et templates
  `%n`) branché pipeline/QA/rapport/dédup.
- Phase 2 — Originaux vierges (`<app-data>/originals/<id>/` + `snapshot.json`), reconstruction des
  dossiers déjà exportés, export idempotent, gate d'intégrité de base.
- Phase 3 — Catégories optionnelles (migration `0010`), scanner de références, listes System,
  `displayName`, `TextCoverageDialog`, hook post-ouverture.
- Phase 4 — Codec `plugins.js` à l'octet près, 16 profils TOML vérifiés sur le source des plugins,
  arguments 357, fill-ins NUUN, `ConstraintViolation` en QA.
- Phase 5 — Libellés Keke `<コマンド:…>`, renommage lié des types de compétence dans toutes les notes,
  simulation différentielle Keke dans le gate.
- Phase 6 — Option `locale` facultative (`preflight_export` + `setLocale`), `ExportIntegrityDialog`,
  pilote réel étendu, docs.

Pilote réel (données JP reconstruites) : 18 catégories, 6 763 segments faussement traduits, 0 erreur
critique, deux exports identiques, `plugins.js` : 7 lignes sur 80 modifiées (vérifié avec Node `vm`),
option `locale` : 8 lignes lisant le réglage, seule `locale` change.

## Fichiers créés

- `src-tauri/src/engines/mv_mz/{coverage/mod.rs, coverage/sites.rs, coverage/plugins.rs, coverage/keke.rs, profiles.rs, keke.rs, plugins_js.rs, json_span.rs, originals.rs, integrity.rs, locale.rs}`
- `src-tauri/src/core/originals.rs`, `src-tauri/src/commands/coverage.rs`
- `src-tauri/profiles/mv_mz/*.toml` (16 profils)
- `src-tauri/migrations/0010_text_categories.sql`
- `src-tauri/tests/{text_coverage.rs, real_game_coverage.rs}`, fixture `src-tauri/tests/fixtures/mv_mz/mz_coverage/`
- `src/components/{TextCoverageDialog.tsx, ExportLocaleOption.tsx, ExportIntegrityDialog.tsx}` (+ tests), `src/stores/coverage.ts`
- `docs/engines/mv-mz-optional-text.md`

## Fichiers modifiés

- `src-tauri/src/commands/export.rs` — export depuis le snapshot, `FileClass::PluginsJs`, réécritures
  Keke, gate d'intégrité, `preflight_export`, paramètre `set_locale`
- `src-tauri/src/llm/{tokenizer.rs, pipeline.rs, batch.rs, context/mv_mz.rs}`, `src-tauri/src/core/{qa.rs, report.rs}` — syntaxe par segment, contraintes
- `src-tauri/src/engines/mv_mz/{extractor.rs, injector.rs, terminology.rs}` — nouveaux kinds, préfixe `\FS` limité au cœur
- `src-tauri/src/state.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/db/pool.rs`, `src-tauri/src/engines/filter.rs`, `src-tauri/src/core/h2s_pack.rs` (`text_category`)
- `src/hooks/useAppHandlers.ts`, `src/components/{AppDialogs.tsx, AppToolbar.tsx, editor/QAPanel.tsx}`, `src/stores/project.ts`, `src/lib/types.ts`, `src/locales/{en,fr}.json`
- `ROADMAP.md`, `CHANGELOG.md`, `tasks/todo.md`, `tasks/lessons.md`

## Fichiers supprimés

- Aucun

## Dépendances ajoutées

- Aucune nouvelle crate (features `serde_json` explicitées)

## Décisions prises

- « Segment existe ⇔ catégorie activée » : désactiver parke les traductions au lieu de les perdre.
- Plugins sans profil vérifié : rapport seulement, jamais extraits (un paramètre texte peut être une
  énumération, une clé de note-tag ou une touche).
- Renommages liés (types de compétence ⇄ tags Keke) : opt-in par catégorie, conséquences confirmées.
- `locale` : facultatif, décoché à chaque export, conséquences tirées du code du jeu lui-même.
- `FormatSpecifierMismatch` abandonné : avec `%n` tokenisé par segment, un `%n` manquant est déjà
  un `MissingPlaceholder` critique.
- Migration numérotée `0010` (0009 pris sur une autre branche).
- Pas de « dry-run » structuré dans `preflight_export` : le dialogue d'intégrité lit les lignes du gate.

## Problèmes rencontrés

- Fork `wolfrpg-map-parser` inaccessible sur la machine Windows → override local non commité
  (`Cargo.lock` local restauré avant commit).
- Binaires de test Tauri Windows : `STATUS_ENTRYPOINT_NOT_FOUND` → manifeste side-by-side (script local).
- Faux positifs du pilote réel (littéraux du plugin propriétaire, mémos de notes, positions Keke)
  → exclusion `owner_plugin`, `readable_note_text`, masquage.
- Données du jeu déjà traduites (zip dézippé par-dessus `data/`) → reconstruction des originaux.
- 2 échecs de base Windows hors périmètre : `test_find_wolf_data_dir_lowercase_fallback`,
  `upgrades_pre_translation_secs_database`.

## Tâches ROADMAP cochées

- (ROADMAP ne liste que l'à-venir) — suivis ajoutés : segments fusionnés à cible vide,
  avertissement CharacterPictureManager `Speaker`, nouveaux profils sur demande.

## Prochaine session

- Vérification en jeu (copie du jeu en scratch, dézipper l'export, lancer) avec l'accord de l'utilisateur.
- Bug des segments multi-lignes fusionnés à cible vide (`Map109.json /events/5/pages/0/list/115`).
- PR de `feat/mv-mz-plugin-text` vers `main`.

---
*Généré par Claude Code — Hoshi2Star*
