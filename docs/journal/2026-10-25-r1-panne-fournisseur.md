# Journal — 2026-10-25 — Lot R1 : panne du fournisseur, une seule notification, annulation

**Phase** : F4/F5 (fiabilisation, lot R1)
**Durée estimée** : 3h
**Statut** : ✅ Code, tests et essai réel faits — non commité
**Outil** : Claude (Opus 5.5, effort high)
**Branche et commit de départ** : `fix/r1-provider-outage`, créée depuis `main` @ `fb6be51`
**Objectif / plan / story** : `tasks/todo.md`, « Plan détaillé — R1 » (décisions D1–D7 validées, D6 = oui, annulation incluse)

---

## Ce qui a été fait

- Étape 1 : classement des erreurs HTTP (`classify_status`) commun à `translate` et `chat` ; 401/403/404 sans relance ⇒ `Unavailable` ; connexion, délai, 408/429/5xx relancés avec attente croissante (1 s, 2 s ; `Retry-After` respecté, plafond 30 s) puis `Unavailable` ; 400/413/422 ⇒ `TranslationFailed` limité au lot. Attente injectable (`with_retry_base_delay`). D7 : `health_check` vérifie que le modèle est listé (`name` = `name:latest` accepté).
- Étape 2 : `split.rs` renvoie `Err` sur `Unavailable` ; résultat par segment `Option<String>` (`None` = échec, rien d'écrit).
- Étape 3 : `pipeline.rs` sans repli source ⇒ cible ; `TranslationResult.failed` ; un échec n'est ni contrôlé par la QA ni enregistré ; `PipelineError::ProviderUnavailable { processed }`. `semantic_retry` arrête ses relances sur panne.
- Étapes 4 et 7 : `translate.rs` émet un seul `h2s://llm/error` (`LlmErrorPayload`) ou un seul `h2s://llm/completed` (`RunSummary` : traduits, à réviser par type, échecs, annulé) ; reprise D6 (`RESUMABLE_SEGMENT`) ; commande `cancel_translation` + `AppState.translation_cancel`, vérifié entre deux lots et pendant la pause de refroidissement.
- Étapes 5 et 6 : `stores/llm.ts` sans toast par segment ; un toast récapitulatif ou d'erreur (id fixe `llm-run`) ; bouton « Arrêter » (`AppToolbar.tsx`) ; textes `llmRun.*` fr/en ; `PlaceholderWarningPayload` et l'événement `placeholder-warning` supprimés.
- CHANGELOG ([Unreleased] : Fixed ×3, Added ×2).

## Fichiers créés

- `docs/journal/2026-10-25-r1-panne-fournisseur.md`

## Fichiers modifiés

- `src-tauri/src/llm/provider.rs`, `split.rs`, `pipeline.rs`, `progress.rs`, `semantic_retry.rs`
- `src-tauri/src/commands/translate.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/state.rs`
- `src/stores/llm.ts`, `src/stores/llm.test.ts`, `src/components/AppToolbar.tsx`, `src/locales/fr.json`, `src/locales/en.json`
- `src-tauri/src/domain/types.rs`, `src-tauri/src/commands/project.rs` (`resumable_count`), `src/lib/types.ts`, `src/components/AppDialogs.tsx`, mocks de tests (`useAppHandlers.test.ts`, `project.test.ts`, `ProjectList.test.tsx`)
- `CHANGELOG.md`, `tasks/todo.md`, `tasks/handoff.md`

## Fichiers supprimés

- Aucun.

## Dépendances ajoutées

- Aucune.

## Décisions prises

- D2 étendu à l'échec final des placeholders (aucune écriture plutôt que source ⇒ cible).
- D6 appliqué aussi à « Traduire » sur un fichier. Compteur `resumableCount` ajouté à `get_project_stats` pour la boîte « Tout traduire » (sinon elle annonçait moins que l'exécution) ; `untranslatedCount` (blocage d'export) inchangé.
- D7 limité au préréglage Ollama : le routeur Hugging Face accepte des ids absents de sa liste (`:fastest`, `model:provider`).
- Quatre tests décrivaient l'ancien repli (`test_placeholder_failure_falls_back_to_needs_review`, `test_response_format_exhausted_falls_back_to_needs_review`, `test_split_partial_success`, `later_canonical_success_repairs_an_earlier_source_fallback`) : relus, attentes inversées ; le dernier simule désormais l'état laissé par les anciennes versions (la réparation des noms reste utile pour ces lignes).

## Problèmes rencontrés

- Un test « 503 puis 200 » n'est pas déterministe avec httpmock : remplacé par des tests purs de la politique (`status_classes_follow_the_outage_policy`, `retry_delay_grows_and_honours_retry_after_within_cap`).
- Interruption de session pendant le gate : relancé en arrière-plan.

## Corrections et erreurs à ne plus répéter

- Aucune correction utilisateur pendant cette session.

## Vérifications et limites

- 2026-10-25, gate complet : `pnpm typecheck` OK ; `pnpm test` 129 tests OK ; `cargo clippy -- -D warnings` OK ; `cargo test` 705 OK, 12 ignorés ; `pnpm lint` 0 erreur, 10 avertissements (déjà présents).
- Après l'ajout de l'annulation pendant la pause, de `resumableCount` et de la restriction D7 : gate complet relancé le 2026-10-25 : typecheck OK, 129 tests Vitest OK, clippy `-D warnings` OK, 705 tests Rust OK (12 ignorés), lint 0 erreur (10 avertissements déjà présents).
- Étape 9 faite (section « Essai réel »). La sortie anticipée de la pause de refroidissement reste sans test automatique ni essai réel.
- Risque connu : un délai dépassé (180 s) arrête l'exécution comme une panne.
- Pilotes réels ignorés (`#[ignore]`, non lancés) : `tests/support/full_pilot/translation.rs::retry_needs_review` reprend `status = 'needs_review'` ; les segments en échec restent désormais `untranslated`, leurs comptes changeront.

## Essai réel (étape 9), 2026-10-25, 20:00–20:09 UTC

Copie de la session 2 (`XDG_DATA_HOME=…/test/_session2/xdg-data`), projet jetable `MZ_standgirl_fresh` (`ed4060fc…`), gemma4:e4b. `/proc/<pid>/fd` : seule la base copiée ouverte ; sha256 de la base originale identique avant/après (`fc698f5f…`). Détail brut hors Git : `test/_session2/r1/` (relevés SQL `00`–`06`, mémoire WebKit `03`, captures `01`, `02`).

- État initial : 1 119 lignes `needs_review` cible = source (panne de la session 2), 72 `translated`.
- Boîte « Tout traduire » : **1 119** segments annoncés (`resumableCount`, D6).
- **Arrêter** : un seul toast info « Traduction arrêtée : 60 traduit(s), 0 à réviser, 0 en échec » ; exactement 60 lignes changées (anciennes lignes D6 ⇒ `translated`). Relance : 1 059 annoncés.
- **Panne** (Ollama arrêté par l'utilisateur pendant l'exécution) : un seul toast d'erreur « Fournisseur IA injoignable : traduction arrêtée après 20 segment(s) sur 1059 » ; 20 lignes changées (lot terminé avant la coupure), aucune nouvelle ligne cible = source ; WebKit 266–271 Mo (session 2 : 8,7 Go puis tué).
- **Retraduction par sélection** de 5 segments `translated`, Ollama arrêté : arrêt au contrôle de démarrage, un seul toast, 0 ligne changée. La panne au milieu d'une sélection n'est couverte que par le test `failed_retranslation_never_overwrites_a_reviewed_target`.
- **Reprise** (Ollama relancé) : 1 039 annoncés ; arrêt volontaire après environ 1 min ; toast « 80 traduit(s) » ; 162 lignes changées, toutes d'anciennes lignes D6 ⇒ `translated`. Pic mémoire WebKit sur tout l'essai : 289 Mo. Aucun `panicked` dans `app.log`.
- **Écart relevé** : le récapitulatif ne compte pas les lignes réparées par la réparation des noms canoniques (`pipeline.rs`, requête « later validated translation repairs that fallback ») : 82 des 162 lignes (93 `speaker` de même source, une seule traduction). Écritures correctes, mais le toast d'arrêt dit « Le reste n'a pas été modifié ». Suite proposée : compter ces réparations dans `RunSummary` (petit).
- **Corrigé après l'essai** (demande utilisateur) : `persist_batch_results` renvoie le nombre de lignes réparées ; `PipelineRunOutcome.repaired` ⇒ `RunSummary.repaired` ⇒ ligne « N autre(s) ligne(s) du même nom… » dans le toast. Tests : `later_canonical_success_repairs_an_earlier_source_fallback` (repaired = 1), Vitest « repaired name rows are reported ».
- **Corrigé pendant l'essai** : le message de panne conseillait « Tout traduire » même pour une sélection ⇒ « relancez la traduction » (fr/en). Vitest `llm.test.ts` : 13 OK.
- Délai de 180 s : jamais atteint pendant l'essai.

## Gate cloud (2026-10-25)

- Le gate ne tourne plus sur le poste de l'utilisateur (il gèle) : vérification en session cloud.
- Premier essai bloqué : le fork `wolfrpg-map-parser` n'existait plus ; réglé par RW0 (PR #11), puis `main` fusionné dans la branche (`98236f1`).
- Gate cloud du 2026-10-25 sur `98236f1` (R1 + `main` avec RW0) : fmt OK, typecheck OK, Vitest 133 OK (35 fichiers), clippy `--locked -D warnings` OK, `cargo test --locked` 695 OK / 0 échec / 12 ignorés, lint 0 erreur (10 avertissements, les mêmes que sur `main`) ; tests R1 ciblés et tests RW0 tous verts. Le commit `53d6217` (compte des noms réparés), jamais compilé avant, passe.

## Tâches ROADMAP cochées

- Aucune.

## Prochaine session

- Commit `7be55e4` et PR #10 faits ; compte des réparations ajouté ensuite. Reste : relecture et fusion de la PR par l'utilisateur, puis plan du lot R2.

---
*Journal partagé Codex / Claude — Hoshi2Star*
