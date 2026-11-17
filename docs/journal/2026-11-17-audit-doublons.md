# Journal — 2026-11-17 — Audit des doublons (front et back)

**Phase** : F5
**Durée estimée** : 1h
**Statut** : ✅ Complété (audit seul ; aucun lot commencé)
**Outil** : Claude (session locale, lancée par une autre session Claude)
**Branche et commit de départ** : `main` @ `1f2b841`, arbre de travail propre
**Objectif / plan / story** : audit statique des doublons et répétitions de `src/` et `src-tauri/src/`, sans modification du code applicatif, sans commit. Livrable : `docs/audit/2026-11-17-audit-doublons.md`.

---

## Ce qui a été fait

- Reprise selon `AGENTS.md` : `tasks/handoff.md`, `tasks/lessons.md`, `CONTEXT.md`, état Git.
- Relecture de ce qui était déjà fait : Phases 6 et 8 (`tasks/todo.md:2354-2520`), lot P5 (`docs/audit/2026-10-29-audit-frontend.md` §7, `docs/journal/2026-10-29-fe-lots.md`).
- Recherche de copier-coller par un script Python local (lignes normalisées, fenêtres de 7 lignes en Rust et 6 en TypeScript, tests exclus) : 76 paires en Rust, 93 en TypeScript.
- Recherche de logique dupliquée par deux sous-agents (back, front), puis revérification des constats retenus (`diff`, `rg`, lecture).
- Lecture directe et complète du cas `TextBox` : `engines/mv_mz/layout/fit.rs`, `engines/wolf/layout/{fit,mod}.rs`, `core/text_fit.rs`, `commands/text_fit.rs`, `commands/wolf_text_fit.rs`.
- Rédaction du rapport : constats, ressemblances trompeuses, classement gain/risque, 7 lots proposés (`TextBox` en premier), 4 décisions à prendre.

## Fichiers créés

- `docs/audit/2026-11-17-audit-doublons.md`
- `docs/journal/2026-11-17-audit-doublons.md`

## Fichiers modifiés

- `tasks/handoff.md` — section en tête qui pointe vers le rapport.

## Fichiers supprimés

- Aucun.

## Dépendances ajoutées

- Aucune. `jscpd` n'a pas été téléchargé.

## Décisions prises

- `jscpd` remplacé par un script local, pour ne rien télécharger sans accord. Limite : il ne voit pas les clones dont un identifiant a été renommé.
- `MvTextBox` exclu du lot 1 : MV est refusé aujourd'hui (`commands/text_fit.rs:131-135`), il n'y a donc aucun cas réel MV sur lequel bâtir.
- Type d'erreur commun non proposé : `Result<T, String>` à la frontière IPC est une convention du projet ; la changer relève d'un ADR.
- Extracteurs MV/MZ et VX Ace laissés séparés malgré 150 lignes identiques : VX Ace est une copie figée et désactivée.

## Problèmes rencontrés

- Le sous-agent du front annonçait 27 `String(err)` ; `rg` en trouve 25 hors `lib/`. Le rapport retient 25.
- Le sous-agent du back situait `write_export` en `export.rs:511` ; la fonction commence en `:430`, le bloc en double est en `:505-640`. Corrigé dans le rapport.

## Corrections et erreurs à ne plus répéter

- Aucune correction de l'utilisateur pendant cette session. Pas de nouvelle leçon.

## Vérifications et limites

- 2026-11-17 : aucun build, aucun test, aucun `clippy` (interdits sur ce poste). Analyse statique seulement.
- Vérifié par `diff` : `legacy_xor.rs:1380-1433` ↔ `:1440-1493`, `llm/split.rs:121-150` ↔ `:194-223`, `core/report.rs:436-460` ↔ `utils/time.rs:4-26`.
- Vérifié par `rg` : 24 lignes d'import `mv_mz::layout` hors de `mv_mz` ; 33 comparaisons de moteur en chaîne ; aucun appel à `debug_inject_file` hors `lib.rs` et `export.rs` ; `extract_wolf_project` appelé par les tests seulement.
- Supposé, non relu ligne à ligne : voir §7 du rapport (B9 à B12, D4, D9, F7, F8, plusieurs comptes de tests).
- `git status --short` en fin de session : deux fichiers nouveaux et `tasks/handoff.md` modifié. Rien de commité.

## Tâches ROADMAP cochées

- Aucune.

## Prochaine session

- Faire trancher les 4 décisions du §6.3 du rapport (`debug_inject_file`, repli `fr`/`en`, champs `wolf_*`, `jscpd`).
- Puis écrire le plan du lot L1 (`TextBox`) dans `tasks/todo.md`, en commençant par le corpus de caractérisation MZ et Wolf. Gate en session cloud seulement.

---
*Journal partagé Codex / Claude — Hoshi2Star*
