# Journal — 2026-10-30 — Préfixe de visage Wolf `@N⏎` ancré en tête (plan FP)

**Phase** : F5
**Durée estimée** : 2h
**Statut** : ✅ Complété (FP-1 à FP-6) ; FP-7 (essai réel) à faire en local après la fusion
**Outil** : Claude (session cloud)
**Branche et commit de départ** : `fix/wolf-face-prefix`, partie de `main` @ `2d6a59f`
**Objectif / plan / story** : `tasks/todo.md`, « Plan — FP », étapes FP-1 à FP-6

---

## Ce qui a été fait

- **Défaut (constaté le 2026-10-30, app réelle, トロイメライ `CommonEvent.dat`).** Le préfixe `@1⏎`
  d'un message Wolf v3 était tokenisé (`⟦ph_0⟧`) et envoyé au LLM. Le LLM l'a déplacé :
  `@1⏎「\cself[8]」⏎を失った。` est devenu `Lost \cself[8]「@1⏎」.`. `Tokenizer::validate` ne contrôle
  que la présence d'un jeton, pas sa position, et la QA a donné 100.
- **Cause confirmée (lecture du code).** `RE_WOLF` avait une alternative `^@\d+\n` qui faisait du
  préfixe un placeholder ordinaire, et rien ne vérifiait qu'il restait en tête.
- **FP-1, tokenizer (`llm/tokenizer.rs`).** Nouveau champ `Tokenized::face_prefix`.
  - Pour `Engine::Wolf` et `Engine::WolfMessage`, `Tokenizer::tokenize` détache le préfixe avec
    `engines::wolf::layout::text::split_face_prefix` avant tout le reste. La protection des bords, les
    retours souples et la regex ne voient que le corps.
  - `Tokenized::restore` restaure, applique `rebreak`, puis remet le préfixe en tête.
  - Une réponse vide reste vide (voir « Problèmes rencontrés »).
- **Seul chemin de restauration, revérifié par grep.** `Tokenized::restore` n'est appelé qu'à
  `llm/split.rs:178`. La relance sémantique passe par `llm_translate_with_split`, donc par ce même appel.
  `Tokenizer::restore` et `Tokenizer::validate` ne sont appelés que dans les tests et par
  `Tokenized::restore`. Les autres appels à `Tokenizer::tokenize` ont été relus :
  - `engines/filter.rs::needs_translation` : le corps sans préfixe donne le même verdict ;
  - `core/qa.rs` : le préfixe est désormais contrôlé par la nouvelle règle ;
  - les autres appels sont MV/MZ seulement (`mv_mz/terminology.rs`, `terminology/analyzer/japanese.rs`,
    `commands/pilot.rs`, `tests/real_game_coverage.rs`, `tests/e2e_project_flow.rs`).
- **FP-3, règle QA (`core/qa.rs`).** Nouvelle erreur `QaError::MisplacedFacePrefix { expected, found }`,
  critique, pénalité 25, contrôlée seulement pour les segments Wolf (`TokEngine::Wolf | WolfMessage`).
  `expected` est le préfixe de la source, `found` le préfixe fautif, tous deux sous la forme `@N`
  (chaîne vide : aucun).
  - Préfixe absent de la tête de la cible : `expected = "@1"`, `found = ""`.
  - Préfixe déplacé, ou répété dans le texte : `expected = found = "@1"`.
  - Autre `N` : `expected = "@1"`, `found = "@2"`.
  - Préfixe dans la cible alors que la source n'en a pas : `expected = ""`, `found = "@3"`.
  - Un `@N⏎` déjà présent dans le corps de la source est du texte ordinaire : la règle ne se déclenche
    que si le corps de la cible en contient davantage.
- **Branchements.**
  - Rust : `type_key` (`misplaced_face_prefix`), `penalty`, `is_critical`, libellés fr/en ;
  - `core/report.rs` : compteur des placeholders, pastille rouge, case de filtre, libellés fr/en ;
  - front : union `QaErrorType` (`src/lib/types.ts`), icône rouge et libellé dans `QAPanel.tsx`
    (4 cas, comme côté Rust) ;
  - i18n fr et en : `qaPanel.errors.misplaced_face_prefix.*`, `llmRun.qaTypes`, `pilot.qaError`.
- **FP-2 et FP-4, tests.** Voir « Vérifications ».
- **FP-5.** Une ligne « Fixed » dans `CHANGELOG.md`, ce journal, et une leçon dans `tasks/lessons.md`.

## Fichiers créés

- `docs/journal/2026-10-30-wolf-face-prefix.md`

## Fichiers modifiés

- `src-tauri/src/llm/tokenizer.rs` — champ `face_prefix`, détachement dans `tokenize`, remise en tête
  dans `Tokenized::restore`, réponse vide conservée vide ; 5 tests ajoutés, 1 adapté.
- `src-tauri/src/engines/wolf/placeholders.rs` — alternative `^@\d+\n` retirée de `RE_WOLF` ; 2 tests
  réécrits (le préfixe est dans `face_prefix`, plus dans `map`), 1 assertion ajoutée.
- `src-tauri/src/core/qa.rs` — `MisplacedFacePrefix`, `check_face_prefix`, branchements ; 6 tests.
- `src-tauri/src/core/report.rs` — compteur, couleur, filtre, libellés ; 1 test.
- `src-tauri/src/llm/pipeline.rs` — test WR-E6 adapté (la réponse simulée n'a plus de `⟦ph_0⟧`) ;
  3 tests ajoutés avec faux fournisseur.
- `src-tauri/src/commands/export.rs` — commentaire seulement : renvoi vers `split_face_prefix` au lieu
  de `RE_WOLF`.
- `src/lib/types.ts`, `src/components/editor/QAPanel.tsx`, `src/components/editor/QAPanel.test.tsx`
  (4 cas), `src/locales/fr.json`, `src/locales/en.json`.
- `CHANGELOG.md`, `tasks/todo.md` (FP-1 à FP-6 cochés), `tasks/lessons.md`.

## Fichiers supprimés

- Aucun.

## Dépendances ajoutées

- Aucune. `pnpm-lock.yaml` n'est pas modifié.

## Décisions prises

- **Alternative `^@\d+\n` de `RE_WOLF` retirée**, plutôt que gardée avec un commentaire.
  - `RE_WOLF` n'a qu'un utilisateur, `Tokenizer::tokenize`, qui détache désormais le préfixe avant la regex.
  - L'alternative n'était pas tout à fait inatteignable : elle aurait encore pris un second `@N⏎` en tête
    du corps (`@1⏎@2⏎…`). Le moteur affiche ce second `@2` comme du texte. En faire un placeholder
    obligatoire reproduirait le défaut corrigé : un jeton qui n'a de sens qu'en tête, envoyé au LLM.
  - Le plan disait que l'alternative servait de référence au `PH_RE_WOLF` du front. C'est faux :
    `src/lib/constants.ts:83` ne l'a jamais contenue. La garder n'aurait servi à rien et aurait induit en
    erreur.
  - La doc de `RE_WOLF` dit maintenant où le préfixe est traité, et le commentaire de
    `commands/export.rs:54` renvoie à `split_face_prefix`.
- **Le préfixe est aussi détaché pour `Engine::Wolf`** (champs de base de données, choix), comme demandé.
  Un `@N⏎` en tête d'un tel texte n'est jamais envoyé au LLM, et la règle QA vaut pour tous les segments
  Wolf.
- **Libellé à 4 cas, calculé des deux côtés** (Rust pour le rapport HTML, TS pour `QAPanel`). Une seule
  variante `{ expected, found }`, comme demandé par le plan, sans champ supplémentaire.
- **Symbole `@` ASCII.** La consigne écrivait `＠N` (pleine chasse). Le code (`RE_FACE_PREFIX`,
  `layout/text.rs:23`) et les données réelles utilisent `@` ASCII : j'ai gardé `@`. Aucun `＠` n'apparaît
  dans le code source.

## Problèmes rencontrés

- **Réponse vide (régression évitée).** Avant, une réponse vide pour un message préfixé perdait le jeton
  `⟦ph_0⟧` : la restauration échouait, puis le segment était relancé ou marqué en échec. Avec la remise
  en tête seule, elle aurait donné `@1⏎`. Ce texte passe le filtre « cible vide » du pipeline
  (`pipeline.rs:666`, `trim` donne `@1`), et `EmptyTranslation` ne se déclenche pas non plus. Le segment
  aurait été écrit comme traduit.
  - Correction : `Tokenized::restore` renvoie la réponse telle quelle si son corps est vide.
  - Preuves : assertions dans `wolf_face_prefix_is_put_back_in_front_of_the_response`, et test
    `wolf_empty_answer_is_not_saved_as_a_bare_face_prefix` (segment en échec, rien d'écrit).
- **Environnement cloud.** Il a fallu installer `libwebkit2gtk-4.1-dev`, `libgtk-3-dev` et leurs
  dépendances (`apt-get update` d'abord), puis `pnpm install --frozen-lockfile`. `lindera` a compilé sans
  refus réseau.
- **Prettier.** `prettier --write` sur `QAPanel.tsx` reformatait aussi deux lignes existantes. Je les ai
  remises comme sur `main` pour rester dans le périmètre. `pnpm lint` (ESLint seul) n'est pas concerné.

## Corrections et erreurs à ne plus répéter

- Symptôme : le préfixe `@1⏎` est déplacé par le LLM, et la QA donne 100.
- Cause confirmée : un code que le moteur ne lit qu'en tête était envoyé comme placeholder ordinaire. La
  validation ne contrôle que la présence du jeton.
- Correction : le préfixe est détaché avant la tokenisation, remis en tête après la restauration, et
  contrôlé par une règle QA critique.
- Preuves : les tests de ce lot (ci-dessous). Leçon ajoutée dans `tasks/lessons.md` (2026-10-30).

## Vérifications et limites

- Gate complet (2026-10-30, cloud, relancé dans l'ordre sur l'arbre final) :
  - `cargo fmt --manifest-path src-tauri/Cargo.toml -- --check` : propre ;
  - `cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings` : exit 0 ;
  - `cargo test --manifest-path src-tauri/Cargo.toml --no-fail-fast` : 841 OK, 0 échec, 15 ignorés,
    sur 17 suites (lib : 768 OK, 7 ignorés) ;
  - `pnpm typecheck` : exit 0 ;
  - `pnpm lint` : 0 erreur, 10 avertissements, tous dans des fichiers non touchés par ce lot ;
  - `pnpm test` : 195/195, 42 fichiers (191 avant le lot, +4 cas `QAPanel`).
- Tests ajoutés ou adaptés :
  - tokenizer : préfixe absent de `text` et de `map` (Wolf et WolfMessage) ; remis en tête à la
    restauration ; `rebreak` avec préfixe sur `@2⏎魚淫さん：⏎…` ; `@10⏎` ; réponse vide ; protection
    des bords après le préfixe (`@1⏎-----エネミー`) ; message sans préfixe et MV/MZ inchangés ;
  - QA : un test par cas, plus le cas sans erreur, MV/MZ non concerné, champ de base Wolf contrôlé,
    `type_key`, pénalité, criticité et libellés ;
  - rapport HTML : type, pastille, filtre, libellé fr ;
  - pipeline (faux fournisseur, base SQLite réelle) :
    - réponse sans préfixe : cible persistée `@1⏎Lost 「\cself[8]」.`, statut `translated`, aucune
      erreur de préfixe ;
    - réponse avec `@1⏎` au milieu : trois appels (réponse, puis deux relances sémantiques), statut
      `needs_review`, `MisplacedFacePrefix { "@1", "@1" }` ;
    - réponse vide : segment en échec, rien d'écrit ;
  - Vitest : 4 libellés de `QAPanel`, icône rouge.
- Non exécuté : FP-7, l'essai dans l'app réelle, à faire en local après la fusion.
- Limite : un `@N` sans retour à la ligne, écrit à la main dans une cible, n'est pas signalé. Le plan
  définit le préfixe comme `@N⏎`, et le LLM ne voit plus le préfixe, donc il ne peut plus le produire.
- Hors périmètre, noté sans être modifié : `commands/export.rs` a sa propre regex
  `RE_WOLF_SPEAKER_PREFIX`, qui double `split_face_prefix`.

## Tâches ROADMAP cochées

- Aucune (correctif hors roadmap).

## Prochaine session

- Relire et fusionner la PR `fix/wolf-face-prefix` → `main` (utilisateur).
- Puis FP-7 en local (pont MCP) : retraduire les lignes 2 à 4 de `CommonEvent.dat` (トロイメライ) ;
  chaque cible doit commencer par `@1⏎`, avec une QA cohérente. Remettre ensuite les lignes vides, et
  vérifier en lecture seule que la TM n'a rien gagné.

---
*Journal partagé Codex / Claude — Hoshi2Star*
