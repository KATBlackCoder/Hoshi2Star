# Journal — 2026-05-25 — F2 QA Line Width (pixel-based)

**Phase** : F2 (polish QA engine)
**Durée estimée** : 30 min
**Statut** : ✅ Complété

---

## Contexte

Le check QA "ligne trop longue" utilisait une limite fixe de 50 caractères bruts.
Cette valeur était incorrecte : elle ne tient pas compte de la différence de largeur
entre les caractères japonais (full-width) et les caractères latins (half-width),
ni des dimensions réelles de la boîte de dialogue RPG Maker MV/MZ.

---

## Pourquoi l'ancienne limite (50 chars) était fausse

- Elle traitait 'A' et '日' comme identiques en largeur.
- En réalité, un kanji occupe ~2× plus de place qu'une lettre latine.
- Une ligne de 50 kanjis débordait largement ; une ligne de 50 lettres ASCII
  restait dans les limites.

---

## Nouveau système pixel-based

### Dimensions RPG Maker MV/MZ

| Élément | Valeur |
|---------|--------|
| Largeur fenêtre | 816 px |
| Marges horizontales | ~48 px total |
| Largeur utile | **720 px** |
| Largeur caractère full-width (kanji, kana) | **26 px** |
| Largeur caractère half-width (ASCII, latin) | **13 px** |

### Limite normalisée

En unités half-width (half-width = 1, full-width = 2) :

```
max_units = 720 / 13 ≈ 55.38 unités par ligne
```

Concrètement :
- 27 kanjis = 54 unités ✅ (dans la limite)
- 28 kanjis = 56 unités ❌ (trop large)
- 55 lettres ASCII = 55 unités ✅ (dans la limite)
- 56 lettres ASCII = 56 unités ❌ (trop large)

### Plages Unicode full-width reconnues

| Plage | Contenu |
|-------|---------|
| U+4E00–U+9FFF | CJK Unified Ideographs (kanji) |
| U+3040–U+309F | Hiragana |
| U+30A0–U+30FF | Katakana |
| U+FF00–U+FFEF | ASCII pleine largeur + formes mixtes |
| U+F900–U+FAFF | CJK Compatibility Ideographs |
| U+3400–U+4DBF | CJK Extension A |
| U+3000–U+303F | Symboles et ponctuation CJK |

---

## Ce qui a été fait

### `src-tauri/src/core/qa.rs`

- **Supprimé** : constante `MAX_CHARS_PER_LINE: usize = 50`
- **Supprimé** : dérivation `Eq` de `QaError` (f32 n'implémente pas `Eq`)
- **Ajouté** : struct `LineWidthConfig` avec valeurs par défaut calibrées MV/MZ
- **Ajouté** : méthode `max_halfwidth_units()` → 720.0 / 13.0 ≈ 55.38
- **Ajouté** : `fn is_fullwidth(c: char) -> bool` — détecte les caractères full-width
- **Ajouté** : `pub fn measure_line_units(line: &str) -> f32` — mesure la largeur d'une ligne
- **Ajouté** : `fn check_line_length(text: &str, config: &LineWidthConfig) -> Vec<QaError>`
- **Mis à jour** : `QaError::LineTooLong` — nouveaux champs `units: f32`, `max_units: f32`, `char_count: usize`
- **Mis à jour** : `check()` appelle `check_line_length` avec `LineWidthConfig::default()`
- **Mis à jour** : les tests existants (lignes longues avec "A".repeat(56) au lieu de 51)
- **Ajoutés** : 10 nouveaux tests (is_fullwidth, measure_line_units, check_line_length)

### `src/lib/types.ts`

- `QaErrorType` variante `line_too_long` : remplacé `length: number; max: number`
  par `units: number; max_units: number; char_count: number`

### `src/components/editor/QAPanel.tsx`

- `errorLabel` pour `line_too_long` : utilise désormais `units.toFixed(1)`,
  `max_units.toFixed(1)`, et `char_count` avec les clés i18n `units`/`maxUnits`/`chars`

### `src/locales/en.json` + `fr.json`

- EN : `"Line {{line}} too wide ({{units}} / {{maxUnits}} units — {{chars}} chars)"`
- FR : `"Ligne {{line}} trop large ({{units}} / {{maxUnits}} unités — {{chars}} caractères)"`

---

## Note : évolution en F3

`LineWidthConfig` est actuellement créée avec `Default` dans `check()`.
En F3, elle sera exposée comme paramètre de projet (persisté en DB),
permettant aux traducteurs de calibrer selon la police et le moteur choisis.

---

## Fichiers créés

- `docs/journal/2026-05-25-f2-qa-linewidth.md` — ce fichier

## Fichiers modifiés

- `src-tauri/src/core/qa.rs`
- `src/lib/types.ts`
- `src/components/editor/QAPanel.tsx`
- `src/locales/en.json`
- `src/locales/fr.json`

---

## Tests

- `cargo test` : ✅ **103/103** (93 existants + 10 nouveaux)
- `cargo clippy -D warnings` : ✅ 0 warning
- `pnpm typecheck` : ✅ 0 erreurs

## Tâches ROADMAP cochées

- aucune (amélioration qualitative du QA engine F2)

## Prochaine session

**F3 — Polissage + VX Ace + beta privée** :
1. Intégration `marshal-rs` pour les fichiers `.rvdata2` (VX Ace)
2. TM v2 — fuzzy matching (Levenshtein, seuil 80 %)
3. Glossaire v1 — CRUD + injection prompt
4. `LineWidthConfig` exposée comme paramètre de projet (DB)
5. Recrutement 20–30 beta testeurs (Discord / F95zone)

---
*Généré par Claude Code — Hoshi2Star*
