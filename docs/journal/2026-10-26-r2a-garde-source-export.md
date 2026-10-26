# Journal — 2026-10-26 — R2a : export refusé après une mise à jour du jeu

**Phase** : F5 (lots R de l'analyse complète)
**Durée estimée** : 3h
**Statut** : 🔄 En cours — R2a et étape 6 faites ; gate complet bloqué par le réseau (voir « Vérifications et limites ») ; R2b en attente de validation
**Outil** : Claude (session cloud)
**Branche et commit de départ** : `fix/r2a-export-source-guard` @ `e8a97a9` (plan seul)
**Objectif / plan / story** : `tasks/todo.md`, « Plan détaillé — R2 », étapes 1 à 6 (S-X1 / audit F, scénario P-2). Décisions validées : E1 = (a), E2 oui, E3 = (a), E4 oui, E5 oui.

---

## Ce qui a été fait

- Test d'intégration P-2 écrit **avant** le correctif (`src-tauri/tests/export_game_update.rs`), par le chemin réel : `open_project` sur un jeu synthétique, copie de référence prise à l'ouverture, traduction de tous les segments (`update_segment`), 401 inséré dans `Map001.json` sur disque, puis `export_project`.
  - MV : deux blocs Afficher un message, chacun avec son visage ; une ligne insérée avant le second.
  - MZ : deux locuteurs (`101[4]`) puis un choix (`102[0][n]`) ; une ligne insérée après la première réplique.
  - Non-régression : jeu inchangé, MV et MZ ⇒ chaque traduction à sa place, visages intacts, réexport identique après décompression de l'archive sur le jeu.
- **Rouge sur le code de départ** (sortie : section « Vérifications »). MV : export accepté, `Actor2` (nom de visage) devient `EN2`, la réplique `こんばんは、旅の方。` reste en japonais. MZ : `inject Map001.json: key not found in JSON: …/list/2/parameters/4`.
- Étape 1 : `IntegrityIssue::SourceChanged { file, pointer, expected, found }` (`engines/mv_mz/integrity.rs`), rendue sur une seule ligne avec les textes entre guillemets JSON ; `verify_sources` compare la valeur d'origine à `source_text` pour chaque ligne à écrire.
- Étapes 2 et 3 : contrôle avant `inject_to_bytes` dans `export_project` (tous les fichiers sont contrôlés, l'export est refusé à la fin par la porte d'intégrité existante, aucune archive) et dans `debug_inject_file` (refus, rien n'est écrit). Les lignes de mise en page gardent leur pointeur : les sources sont relevées avant `split_fitted`. Étiquettes Keke et `plugins.js` : inchangés (contrôles propres).
- **Point 3 tranché par la preuve** : avec les étapes 1–3 seules (règle `stale_export` inchangée), le test MV reste rouge à l'identique (visage traduit) et le test MZ ne liste pas `…/list/3/parameters/0` : la reconstruction de la copie de référence remet `こんばんは…` / `どこへ…` sous l'ancien pointeur, qui tombe sur le nom de visage du `101` suivant ; la garde compare alors à une copie qui lui donne raison. Étape 6 faite dans cette branche.
- Étape 6 (E2) : `reconstruct_json` ne remet plus la source (ni `was_exported`, ni `stale_export`) sous un pointeur dont le **rôle** a changé. `same_role` : dans une liste de commandes, le `code` trouvé doit être celui de l'extraction (401 dialogue, 405 défilant, 101 `[4]` locuteur, 102 `[0][n]` choix, 357 `[3]…` commande de plugin) ; le type de segment (`segment_kind`, désormais lu par `recorded_segments`) distingue 401 et 405, les anciennes lignes `unknown` acceptent l'un ou l'autre. Hors liste de commandes, le chemin fixe le rôle. Sinon : dérive, valeur laissée telle quelle.
- Étape 4 : `ExportIntegrityDialog.tsx` reconnaît ces lignes (`parseSourceChange`, `src/lib/exportIntegrity.ts`) : emplacement, « attendu », « trouvé » ou « rien (emplacement disparu) », et la phrase « Le jeu a changé depuis l'extraction (mise à jour ?) … » à la place de la description générique. Textes fr/en.

## Fichiers créés

- `src-tauri/tests/export_game_update.rs`
- `src/lib/exportIntegrity.ts`, `src/lib/exportIntegrity.test.ts`
- `docs/journal/2026-10-26-r2a-garde-source-export.md`

## Fichiers modifiés

- `src-tauri/src/engines/mv_mz/integrity.rs` — variante `SourceChanged`, `verify_sources`, tests.
- `src-tauri/src/commands/export.rs` — `recorded_sources`, contrôle dans `export_project` et `debug_inject_file`.
- `src-tauri/src/engines/mv_mz/originals.rs` — `CoreSegment.kind`, `same_role`, règle E2 dans `reconstruct_json`, tests.
- `src-tauri/src/core/originals.rs` — `recorded_segments` lit `segment_kind`.
- `src/components/ExportIntegrityDialog.tsx`, `src/components/ExportLocaleOption.test.tsx`, `src/locales/fr.json`, `src/locales/en.json`.
- `tasks/todo.md`, `tasks/handoff.md`.

## Fichiers supprimés

- Aucun.

## Dépendances ajoutées

- Aucune.

## Décisions prises

- Rôle déduit du pointeur et du `code` de la commande sur disque, affiné par `segment_kind` (401/405) : pas de migration, et les projets anciens (`unknown`) restent couverts.
- La règle E2 s'applique aussi à `was_exported` (valeur égale à la cible) : une valeur égale à la cible mais au mauvais rôle n'est pas une exportation de Hoshi2Star.
- Le contrôle ne plafonne pas le nombre d'écarts par fichier : le compte affiché est celui des segments concernés ; la liste reste bornée par `summarize` (10 lignes).
- Étape 7 (dérive rendue par `update_snapshot`) non faite : hors de la demande de cette session ; le refus de l'étape 2 suffit à bloquer P-2.

## Problèmes rencontrés

- `lindera-ipadic` télécharge son dictionnaire depuis `lindera.dev` au build ; la politique réseau de l'environnement cloud refuse ce domaine (403). Contournement pour compiler et tester : `DOCS_RS=1`, qui fait générer un dictionnaire factice par le build script. Conséquence : les 2 tests de l'analyseur japonais échouent (vérifié identique sur `e8a97a9`) et le binaire n'est pas celui de production. Le gate complet reste à refaire avec `lindera.dev` autorisé.

## Corrections et erreurs à ne plus répéter

- Symptôme : P-2 (traduction écrite dans un nom de visage, ou `key not found in JSON`). Cause confirmée par le test : l'export ne comparait jamais la valeur d'origine à `source_text`, et la reconstruction remettait la source sous un pointeur décalé. Solution : étapes 1–3 et 6. Preuve : `export_game_update.rs` rouge puis vert ; essai « garde seule » rouge.
- Règle durable ajoutée à `tasks/lessons.md` : une garde qui compare à une copie reconstruite doit être testée contre cette reconstruction.

## Vérifications et limites

Toutes avec `CARGO_TARGET_DIR=/home/user/.cache/h2s-target DOCS_RS=1` (dictionnaire lindera factice), le 2026-10-26.

- Rouge (code de `e8a97a9` + nouveau test) : `cargo test --locked --test export_game_update` ⇒ 1 réussi, 2 échecs :
  - MV : « export after a game update must be refused, but it wrote …/hoshi2star.zip » ; page exportée `101 ["Actor1",0,0,2]`, `401 ["EN1"]`, `401 ["新しく追加された台詞です。"]`, `101 ["EN2",1,0,2]`, `401 ["こんばんは、旅の方。"]`, `0 []` ;
  - MZ : « got: inject Map001.json: key not found in JSON: /events/1/pages/0/list/2/parameters/4 ».
- Garde seule (étapes 1–3, sans E2) : MV identique au rouge ; MZ refusé mais 3 écarts seulement (`list/2/parameters/4`, `list/4/parameters/0/0` et `/1`), `list/3/parameters/0` absent.
- Vert (correctif complet) : 3/3 réussis ; tests unitaires `integrity` + `originals` : 29/29.
- `cargo fmt --check` OK ; `cargo clippy --locked --all-targets -- -D warnings` OK.
- `cargo test --locked --no-fail-fast` : 701 réussis, 2 échecs (`core::terminology::analyzer::japanese` : `analyzer_reports_stable_version_and_offsets`, `fixture_exposes_expected_lemmas_and_excludes_engine_codes`, mêmes échecs sur `e8a97a9` sous `DOCS_RS=1`), 12 ignorés.
- `pnpm i --frozen-lockfile` OK ; `pnpm typecheck` OK ; `pnpm test` : 137 réussis (36 fichiers) ; `pnpm lint` : 0 erreur, 10 avertissements (les mêmes qu'avant).
- Non exécuté : gate avec le vrai dictionnaire ; faux positifs sur les 3 jeux réels de la session 2 (36 352 pointeurs, `test/` absent du cloud) ; essai en jeu (E5, demande l'accord de l'utilisateur).

## Tâches ROADMAP cochées

- Aucune (lot R2 suivi dans `tasks/todo.md`).

## Prochaine session

- Autoriser `lindera.dev` dans l'accès réseau de l'environnement cloud, puis relancer le gate complet sans `DOCS_RS`.
- Contrôle des faux positifs sur les jeux réels (local, lecture seule de la base, ou test `#[ignore]` existant adapté), puis validation de l'utilisateur avant R2b (réalignement, étapes 8–10).
