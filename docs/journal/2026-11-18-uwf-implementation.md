# Journal — 2026-11-18 — UWF : déchiffrer un jeu Wolf dans l'app avec unwolf (UWF-1 à UWF-6)

**Phase** : F5
**Durée estimée** : session cloud
**Statut** : 🔄 En cours
**Outil** : Claude (session cloud)
**Branche et commit de départ** : `feat/wolf-unwolf` depuis `main` @ `76f3d5a` (« docs: code map in docs/architecture.md (DD V8) »)
**Objectif / plan / story** : étapes UWF-1 à UWF-6 du « Plan — UWF » (en tête de `tasks/todo.md`), une PR vers
`main`, sans fusion. UWF-7 (retrait de `decrypt/`) et UWF-V (essai réel) hors session. `tasks/todo.md` et
`tasks/handoff.md` ne sont pas modifiés par cette session.

---

## Reprise

- Lus : `tasks/handoff.md`, `tasks/lessons.md`, `CONTEXT.md`, `docs/architecture.md`, le plan UWF en entier,
  ADR-008, `docs/engines/wolf-uberwolf.md`.
- Préalables vérifiés sur `main` @ `76f3d5a` : `src/lib/api/`, `src-tauri/src/core/text_fit/` et
  `src-tauri/src/db/project_context.rs` présents. ADR-009 existe : ADR-010 est libre.
- unwolf 0.2.0 publié sur crates.io (API crates.io : versions `0.2.0`, `0.1.2`, `0.1.1`, `0.1.0`). Sources de la
  0.2.0 téléchargées depuis crates.io dans le dossier temporaire de la session pour lire l'API (lecture seule,
  hors dépôt).
- Environnement cloud : `rustc 1.97.0` (≥ 1.85), Node 22, pnpm 10.28. Les bibliothèques système de Tauri
  manquaient (`gdk-sys` ne compilait pas) : installées comme dans `.github/workflows/release.yml`
  (`libwebkit2gtk-4.1-dev`, `libappindicator3-dev`, `librsvg2-dev`, `patchelf`). `lindera.dev` accessible : le
  dictionnaire japonais se télécharge, aucun test en échec attendu.

## Gate de référence sur `main` @ `76f3d5a` (avant tout changement)

| Contrôle | Résultat |
|---|---|
| `cargo test --locked --no-fail-fast` | 850 réussis, 0 échec, 15 ignorés |
| `pnpm typecheck` | OK |
| `pnpm lint` | 0 erreur, 10 avertissements |
| `pnpm test` (Vitest) | 44 fichiers, 201 tests réussis |

(`cargo fmt --check` et `cargo clippy --all-targets` passés pour la première fois avec UWF-1, ci-dessous.)

## UWF-1 — Dépendance

**Fait.**
- `src-tauri/Cargo.toml` : `unwolf = { version = "0.2", default-features = false }` (UWF-D8), depuis crates.io,
  sans chemin ni Git. `Cargo.lock` : une seule entrée ajoutée, `unwolf 0.2.0` (dépendances `encoding_rs`, `log`,
  `lz4_flex`, `memmap2`, `serde`, `serde_json`, `thiserror 2.0.20`, toutes déjà présentes) ; aucune autre version
  changée (`diff` du lockfile vérifié).
- Mentions de licence : la recherche `rg -i 'third.party|NOTICE|licen'` montre que les avis tiers vivent dans
  `src-tauri/resources/licenses/` (avis de `lindera-ipadic`), inclus dans les bundles par `bundle.resources` de
  `src-tauri/tauri.conf.json`. Même mécanisme pour unwolf : `src-tauri/resources/licenses/unwolf-NOTICE.txt`
  (texte `LICENSE` MIT et `NOTICE` de la crate 0.2.0, à l'identique) ajouté à `bundle.resources`. Pas de
  `THIRD_PARTY_NOTICES.md` à la racine, puisqu'un emplacement existait.

**Écart au plan.** Aucun (`rustc 1.97.0`, publication vérifiée par l'API crates.io plutôt que `cargo search`).

**Gate (UWF-1).** `cargo fmt --check` OK ; `cargo clippy --all-targets --locked -D warnings` OK ;
`cargo test --locked --no-fail-fast` 850 réussis, 0 échec, 15 ignorés ; `pnpm typecheck` OK ; `pnpm lint`
0 erreur (10 avertissements, inchangés) ; Vitest 201/201.
