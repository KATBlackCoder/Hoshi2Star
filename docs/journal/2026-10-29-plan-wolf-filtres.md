# Journal — 2026-10-29 — Plan WF : skipper les textes Wolf non traduisibles

**Phase** : F5 (Wolf)
**Durée estimée** : 2h
**Statut** : 🔄 En cours (plan écrit, à valider)
**Outil** : Claude (session locale)
**Branche et commit de départ** : `docs/wolf-filter-plan` depuis `origin/main` @ `8e32c99`
**Objectif / plan / story** : demande de l'utilisateur — des filtres Wolf qui ne gardent que les textes à traduire ; plan WF dans `tasks/todo.md`

---

## Ce qui a été fait

- Mesuré, sur les trois projets en base, une douzaine de règles candidates, en se limitant à ce que voit
  l'extracteur :
  - le texte et le fichier ;
  - la clé (base, type, ligne, champ) ;
  - les noms de types et de données lus dans `.project` ;
  - les noms des événements communs.
- Mesuré la pureté des groupes sur le 3.658 : 344 groupes sur 352 sont purs. Les règles doivent donc
  décider par champ, par événement et par carte.
- Écrit le plan WF dans `tasks/todo.md`, avec les décisions de l'utilisateur :
  - skip à l'extraction ;
  - un fichier de règles `engines/wolf/filter.rs` ;
  - seulement les règles fiables à 100 % dans ce lot.

## Fichiers créés

- `docs/journal/2026-10-29-plan-wolf-filtres.md`.
- Hors Git : `RAE-PLAN/essais/wolf-filtres-proposes.md`.

## Fichiers modifiés

- `tasks/todo.md` — « Plan — WF ».
- `tasks/handoff.md` — statut et section WF.

## Décisions prises

- Par l'utilisateur :
  - un texte non traduisible est skippé à l'extraction, sans aucun segment créé ;
  - les règles vont dans `engines/wolf/filter.rs` ;
  - les deux règles moins fiables sont mises de côté.
- Le reste (WF-D1 à WF-D6) attend sa validation.

## Problèmes rencontrés

- Le motif large des messages d'erreur (`警告`) attrapait deux répliques de l'histoire dans とのかじ.
  Le plan retient le motif strict : `ｴﾗｰ`, ou `エラー`/`警告` suivi de `!`, `！`, `:` ou `：`.
- La règle « doublon système » a d'abord exclu les deux champs du doublon (erreur de priorité dans le
  script de mesure). Elle a été corrigée et remesurée.
- J'avais d'abord classé `ｼｽﾃﾑ効果名` (とのかじ) « à traduire ». C'est en fait un doublon interne de
  `ｽﾃｰﾀｽ効果名`.

## Vérifications et limites

- Documentation seule : aucun build ni test lancé.
- Trois jeux seulement. Les étiquettes de とのかじ et ポンコツ sont en partie circulaires, puisque leur
  classement s'appuie sur des règles.
- Les mesures portent sur des projets extraits avec l'ancien `main` (`e95bb38`).

## Tâches ROADMAP cochées

- [ ] Aucune.

## Prochaine session

- Validation de WF-D1 à WF-D6 par l'utilisateur, puis WF-1 à WF-6, avec le gate en cloud.

---
*Journal partagé Codex / Claude — Hoshi2Star*
