# Journal — 2026-05-26 — Changelog skill

**Phase** : Infrastructure / Documentation
**Durée estimée** : 5 min
**Statut** : ✅ Complété

---

## Contexte

Mise en place du suivi des changements via Keep a Changelog et création
d'un skill local pour automatiser la mise à jour du CHANGELOG.

---

## Ce qui a été fait

### Étape 1 — Skill update-changelog

Créé `.claude/skills/update-changelog/SKILL.md` avec :
- Format Keep a Changelog (structure, règles, types)
- Exemples corrects / incorrects
- Règle de déclenchement en fin de session

### Étape 2 — CHANGELOG.md initial

Créé `CHANGELOG.md` à la racine avec l'historique complet :
- `[Unreleased]` vide (prêt pour F3)
- `[0.2.1] - 2026-05-25` (11 fixes, 5 ajouts, 3 changements)
- `[0.2.0] - 2026-05-24` (fondations MV/MZ + LLM + TM + QA)
- Liens releases GitHub en bas de fichier

### Étape 3 — CONTEXT.md

Ajouté section `## Changelog` avant `## Progression du développement`
avec référence au fichier et au skill.

---

## Fichiers créés

- `CHANGELOG.md` — historique versions v0.2.0 et v0.2.1
- `.claude/skills/update-changelog/SKILL.md` — skill local
- `docs/journal/2026-05-26-changelog-skill.md` — ce fichier

## Fichiers modifiés

- `CONTEXT.md` — section Changelog ajoutée

---

## Tâches ROADMAP cochées

- aucune (infrastructure uniquement)

## Prochaine session

**F3 — Polissage + VX Ace + beta privée** :
1. Intégration `marshal-rs` pour les fichiers `.rvdata2` (VX Ace)
2. TM v2 — fuzzy matching (Levenshtein, seuil 80 %)
3. Glossaire v1 — CRUD + injection prompt
4. Recrutement 20–30 beta testeurs (Discord / F95zone)

---
*Généré par Claude Code — Hoshi2Star*
