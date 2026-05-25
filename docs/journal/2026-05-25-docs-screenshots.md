# Journal — 2026-05-25 — Docs screenshots

**Phase** : Documentation
**Durée estimée** : 5 min
**Statut** : ✅ Complété

---

## Contexte

Ajout des 6 screenshots de démonstration dans les READMEs (EN + FR).

---

## Ce qui a été fait

### Étape 1 — Renommage des screenshots

6 fichiers renommés dans `docs/screenshots/` :

| Ancien nom | Nouveau nom |
|---|---|
| `2026-05-25_13-12.png` | `01-game-original-jp.png` |
| `2026-05-25_13-13.png` | `02-hoshi2star-empty.png` |
| `2026-05-25_13-14.png` | `03-segments-before.png` |
| `2026-05-25_13-15.png` | `04-segments-translated.png` |
| `2026-05-25_13-15_1.png` | `05-game-translated-en.png` |
| `2026-05-25_13-18.png` | `06-llm-config.png` |

### Étape 2 — README.md (EN)

Remplacé `> Screenshot coming soon` par une section **Screenshots** complète :
- Tableau Before/After (JP original vs EN traduit)
- 4 étapes illustrées (empty state → segments → config LLM → segments traduits)

### Étape 3 — README.fr.md (FR)

Même structure en français : `> Capture d'écran à venir` remplacé par section **Screenshots** FR.

---

## Fichiers modifiés

- `docs/screenshots/` — 6 fichiers renommés
- `README.md` — section Screenshots ajoutée
- `README.fr.md` — section Screenshots ajoutée (FR)

## Fichiers créés

- `docs/journal/2026-05-25-docs-screenshots.md` — ce fichier

---

## Tâches ROADMAP cochées

- aucune (documentation uniquement)

## Prochaine session

**F3 — Polissage + VX Ace + beta privée** :
1. Intégration `marshal-rs` pour les fichiers `.rvdata2` (VX Ace)
2. TM v2 — fuzzy matching (Levenshtein, seuil 80 %)
3. Glossaire v1 — CRUD + injection prompt
4. Recrutement 20–30 beta testeurs (Discord / F95zone)

---
*Généré par Claude Code — Hoshi2Star*
