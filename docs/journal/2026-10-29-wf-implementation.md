# Journal — 2026-10-29 — WF : textes Wolf non traduisibles skippés (implémentation)

**Phase** : F5 (Wolf)
**Durée estimée** : 3h
**Statut** : ✅ Complété (WF-1 à WF-6) ; WF-7 à faire par l'utilisateur en local
**Outil** : Claude (session cloud)
**Branche et commit de départ** : `feat/wolf-filter` depuis `origin/docs/wolf-filter-plan` @ `0806fec`
**Objectif / plan / story** : « Plan — WF » de `tasks/todo.md`, WF-D1 à WF-D6 validées telles que recommandées ; plan : `docs/journal/2026-10-29-plan-wolf-filtres.md`

---

## Ce qui a été fait

- **WF-1** (`d94530d`) : règles Wolf déplacées dans `engines/wolf/filter.rs`, sans changer le
  comportement : `is_translatable` (chemins de ressources d'UW-4), `skip_common_event` (`X[`, `zz`),
  `skip_database_value` (`自動ｼｽﾃﾑ初期化`, champ connu ou japonais). Trois tests déplacés tels quels.
- **WF-2** (`30706a4`) : `skip_map`, `skip_common_event` (+ débogage) et `skip_message` (+ motif strict
  d'erreur), branchés dans `extract_map_segments` et `extract_common_events`. La carte est lue avant
  d'être sautée : une carte de débogage illisible refuse toujours le projet.
- **WF-3** (`91db75d`) : `skipped_fields` (champs de noms d'événements communs, doublons « système ») ;
  `extract_database_segments` reçoit les noms d'événements ; `extract_all_wolf` et
  `extract_wolf_project` lisent `CommonEvent.dat` une seule fois, avant les bases, son entrée restant
  la dernière.
- **WF-4** (`c6c7ef4`) : jeu complet testé dans `extractor.rs` (extraction et export par
  `inject_all_to_memory`), cartes testées par `open_project`/`export_project`
  (`tests/e2e_project_flow.rs`).
- **WF-5** (`b06dd9a` et commit de fin) : `docs/engines/wolf-filter.md`, `CONTEXT.md`, `CHANGELOG.md`,
  `BACKLOG.md`.
- **WF-6** : relecture indépendante (sous-agent) du diff `src-tauri/`, corrections, gate final.
- PR [#27](https://github.com/KATBlackCoder/Hoshi2Star/pull/27) `feat/wolf-filter` → `main` ouverte, non fusionnée.

## Fichiers créés

- `src-tauri/src/engines/wolf/filter.rs`
- `docs/engines/wolf-filter.md`
- `docs/journal/2026-10-29-wf-implementation.md`

## Fichiers modifiés

- `src-tauri/src/engines/wolf/mod.rs` — module `filter`.
- `src-tauri/src/engines/wolf/extractor.rs` — appels à `filter::…`, lecture de `CommonEvent.dat`
  d'abord (`parse_common_events`, `common_event_segments`, `names_of`), nouveaux tests.
- `src-tauri/src/engines/wolf/dat_parser.rs` — constructeur de test `make_string_database_pub`.
- `src-tauri/src/engines/wolf/injector.rs` — deux appels de test à la nouvelle signature.
- `src-tauri/tests/e2e_project_flow.rs` — `wolf_non_translatable_texts_are_skipped_at_open`.
- `CONTEXT.md`, `CHANGELOG.md`, `BACKLOG.md`, `tasks/todo.md`, `tasks/handoff.md`.

## Fichiers supprimés

- Aucun.

## Dépendances ajoutées

- Aucune.

## Décisions prises

- Aucune décision WF-D1 à WF-D6 changée. Trois précisions de lecture, à valider par l'utilisateur :
  1. champ de noms d'événements communs : il faut aussi qu'**au moins une valeur nomme un événement**
     (un champ moitié nombres, moitié texte affiché ne doit pas être sauté) ;
  2. doublon « système » : on compte les lignes où le champ système n'est pas vide (au moins 3),
     l'égalité est exacte, et l'autre champ est le même sur toutes ces lignes ;
  3. **champ homonyme** (ajout après relecture) : un champ qui porte le nom d'un champ sauté placé
     avant lui est sauté aussi. La clé d'un segment de base désigne son champ par son nom, et
     l'injecteur comme la garde d'intégrité prennent le premier champ de ce nom : sans cela, la
     traduction du second serait écrite dans le champ sauté (WF-D1 non tenu, appel d'événement cassé).
- Nom de la carte = nom du fichier (`SampleMapA`), comme les clés `MapData/{stem}` mesurées.

## Problèmes rencontrés

- Le premier `cargo test` (cache vide) a tourné pendant mes modifications de WF-1 : il a compilé WF-1
  (808 OK, même total qu'avant le déplacement), puis le gate complet a été relancé proprement.
- Un remplacement scripté de la correction e2e a échoué à moitié (lignes reformatées par rustfmt) :
  compilation en échec au gate, corrigée avant tout commit.

## Corrections et erreurs à ne plus répéter

- Relecture indépendante : deux assertions du test e2e vérifiaient que le dossier du jeu restait
  inchangé, ce qui est toujours vrai (l'export écrit un zip) ; remplacées par l'assertion sur le
  contenu du zip. Ajout du test « carte de débogage illisible ⇒ erreur ». Pas de nouvelle leçon :
  aucune correction de l'utilisateur.

## Vérifications et limites

- Gate à chaque étape (fmt `--check`, typecheck, Vitest, lint, clippy `--all-targets --locked -D
  warnings`, `cargo test --locked --no-fail-fast`), tous verts :
  - WF-1 : 808 OK / 0 / 15 ignorés ; WF-2 : 815 ; WF-3 : 822 ; WF-4 : 825 ;
  - **final** (après relecture) : **826 OK / 0 échec / 15 ignorés**, Vitest **158/158**, lint 0 erreur
    (10 avertissements connus), clippy 0.
- Non vérifié : les vrais jeux (hors Git ; tests `#[ignore]` non lancés). Les gains attendus de WF-7
  (288 / 469 / 36 textes) ne sont pas mesurés par cette session.
- Relecture : risques **non corrigés** car ils toucheraient aux motifs validés de WF-D5 (consignés dans
  `BACKLOG.md`) : `デバ`/`ﾃﾞﾊﾞ` attrapent `デバフ`, `テスト` attrape `コンテスト`, `test` attrape `Contest` ;
  une réplique `「警告！侵入者を発見！」` serait sautée ; aucun log de ce qui est sauté.

## Tâches ROADMAP cochées

- [ ] Aucune.

## Prochaine session

- Utilisateur : relire la PR, trancher les trois précisions de lecture et les risques de motifs, puis
  WF-7 en local (recréer les trois projets, comparer les segments, export et essai en jeu, UW-9).

---
*Journal partagé Codex / Claude — Hoshi2Star*
