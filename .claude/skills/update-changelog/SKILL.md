---
name: update-changelog
description: >
  Met à jour CHANGELOG.md après chaque fix ou amélioration.
  Utiliser à la fin de chaque session de développement
  et avant chaque release.
---

# SKILL : Update CHANGELOG — Hoshi2Star

## Format : Keep a Changelog

Fichier : CHANGELOG.md à la racine du projet.
Standard : https://keepachangelog.com

## Structure

```
# Changelog

## [Unreleased]
### Added
### Changed
### Fixed
### Removed

## [X.Y.Z] - YYYY-MM-DD
### Added
- Description courte, verbe d'action, en anglais

### Changed
- ...

### Fixed
- ...
```

## Règles

1. Les nouveaux changements vont TOUJOURS sous [Unreleased]
2. Lors d'une release :
   - Renommer [Unreleased] en [version] - date
   - Créer un nouveau [Unreleased] vide au-dessus
3. Chaque entrée : verbe d'action + description courte en anglais
4. Supprimer les sections vides (ex: si pas de Removed, l'omettre)

## Types

- Added : nouvelles features, nouveaux moteurs, nouveaux composants
- Changed : modifications de comportement, mises à jour deps
- Fixed : bugs corrigés
- Removed : features supprimées

## Exemples corrects

✅ Add fuzzy TM matching with Levenshtein distance
✅ Fix qwen3 thinking mode polluting parser output
✅ Change QA line check from char count to pixel units
✅ Add RPG Maker VX Ace engine support
❌ Updated some stuff
❌ Fixed bug
❌ Improvements

## Liens releases (toujours en bas du fichier)

Format :
[0.3.0]: https://github.com/KATBlackCoder/Hoshi2Star/releases/tag/v0.3.0

## Quand utiliser ce skill

Déclencher à la fin de chaque session qui produit
des changements notables — avant le commit final.
Claude Code doit demander : "Shall I update the
CHANGELOG?" si ce skill est actif.
