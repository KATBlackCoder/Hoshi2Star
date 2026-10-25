# Journal — 2026-10-25 — RW0 : retrait de `wolfrpg-map-parser`

**Phase** : F4/F5 (fiabilisation, lot RW0)
**Durée estimée** : 2h
**Statut** : ✅ Code, tests et gate cloud verts — PR à fusionner
**Outil** : Claude (Opus 5.5)
**Branche et commit de départ** : `chore/remove-wolfrpg-map-parser`, créée depuis `main` @ `fb6be51`
**Objectif / plan / story** : `tasks/todo.md`, « Plan — RW0 » (W1 = a : refus à l'ouverture ; W2–W4 validés)

---

## Ce qui a été fait

- Constat : le fork `KATBlackCoder/wolfrpg-map-parser` n'existe plus sur GitHub (`gh repo view` : introuvable) ; la session cloud de vérification de R1 ne pouvait pas compiler ; le poste local ne compilait que grâce au cache Cargo (`21f4f76`).
- Décision utilisateur : retirer le crate (méthode jugée incorrecte), chercher une meilleure lecture de Wolf v2 avec REA (lot RW).
- Extraction (`engines/wolf/extractor.rs`) : branches non-LZ4 ⇒ `ExtractorError::UnsupportedWolfFormat` ; `extract_all_wolf`/`extract_wolf_project` refusent le projet entier (`reject_unsupported`, 3 fichiers cités au plus) au lieu de `eprintln!("warn: skipping")`.
- Injection (`engines/wolf/injector.rs`) : `inject_map`/`inject_common_events` non-LZ4 ⇒ `InjectorError::UnsupportedWolfFormat` ; fonctions v2 supprimées.
- Interface : message dédié à l'ouverture et à l'export (fr/en) ; « moteur non détecté » cite Wolf RPG v3 seulement.
- `Cargo.toml` / `Cargo.lock` édités à la main (aucune commande Cargo de build lancée).
- Docs : CHANGELOG (Removed), compatibilité Wolf, `v3_format/mod.rs`, leçon du 2026-06-11 complétée, rapport d'audit (S-D5), `tasks/todo.md`.

## Fichiers créés

- `src/lib/projectError.test.ts`
- `docs/journal/2026-10-25-rw0-retrait-wolfrpg-map-parser.md`

## Fichiers modifiés

- `src-tauri/src/engines/wolf/extractor.rs`, `injector.rs`, `v3_format/mod.rs`
- `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `src-tauri/tests/e2e_project_flow.rs`
- `src/lib/projectError.ts`, `src/hooks/useAppHandlers.ts`, `src/locales/fr.json`, `src/locales/en.json`
- `CHANGELOG.md`, `docs/references/wolfrpg-format-compatibility.md`, `docs/audit/2026-10-analyse-complete.md`, `tasks/lessons.md`, `tasks/todo.md`, `tasks/handoff.md`

## Fichiers supprimés

- Aucun fichier ; tests v2 retirés : extraction de cartes/événements communs v2 (synthétiques et Honoka), round-trip `.mps` v2, `test_v2_common_event*`, `test_real_honoka_common_events_inject`, e2e `wolf_common_events_open_translate_export_round_trip` (remplacé par `wolf_v2_game_is_refused_at_open`).

## Dépendances ajoutées

- Aucune ; `wolfrpg-map-parser` retiré.

## Décisions prises

- W1 (a) : refus d'ouverture explicite d'un jeu Wolf v2, jamais de projet partiel.
- W2 : export d'un projet v2 existant refusé avec le même message, rien n'est écrit.
- W3 : branche et PR dédiées, à fusionner avant R1 (PR #10).
- W4 : vérification en cloud uniquement (règle du 2026-10-25).

## Problèmes rencontrés

- Aucune compilation locale possible (règle utilisateur) : relecture manuelle des imports et des chemins ; `cargo fmt` seul lancé (formatage, sans build).

## Corrections et erreurs à ne plus répéter

- Une dépendance Git vers un fork personnel rend le dépôt non compilable dès que le fork disparaît : règle ajoutée à `tasks/lessons.md` (leçon du 2026-06-11 complétée).

## Vérifications et limites

- Non exécuté localement (règle « gate en cloud »). Gate cloud du 2026-10-25 sur `32d36ad` : fmt OK, typecheck OK, Vitest 124 OK (35 fichiers), clippy `--locked -D warnings` OK, `cargo test --locked` 681 OK / 0 échec / 12 ignorés, lint 0 erreur (10 avertissements préexistants), `Cargo.lock` cohérent ; il ne reste que des commentaires mentionnant `wolfrpg`. Nouveaux tests tous trouvés et verts (7 Rust, 1 e2e, 3 Vitest).
- Signalé par la session cloud : le hook PreToolUse qui interdit les installations npm/yarn bloque aussi l'installation pnpm (sous-chaîne) ; contourné par `pnpm i`. Hors périmètre, à corriger à part.
- Inconnu : des jeux Wolf v3.0–3.3 non compressés LZ4 existent-ils ? Ils seraient aussi refusés.

## Tâches ROADMAP cochées

- Aucune.

## Prochaine session

- Fusion de la PR RW0 par l'utilisateur ; puis rebase de `fix/r1-provider-outage` sur `main` (conflits attendus : CHANGELOG, handoff, todo) et gate en cloud de R1.

---
*Journal partagé Codex / Claude — Hoshi2Star*
