# RPG Maker MV/MZ — Référence Placeholders

> Fichier de référence pour `src-tauri/src/llm/tokenizer.rs`.
> Tout placeholder listé ici DOIT être tokenisé avant envoi au LLM
> et restauré après réponse avec validation stricte.

---

## Règle fondamentale

```
Source : "Vous avez \V[12] pièces et \N[1] vous attend."

① Tokenizer :  "Vous avez ⟦ph_0⟧ pièces et ⟦ph_1⟧ vous attend."
② LLM traduit : "You have ⟦ph_0⟧ coins and ⟦ph_1⟧ is waiting."
③ Restauration : "You have \V[12] coins and \N[1] is waiting."

Si ⟦ph_0⟧ ou ⟦ph_1⟧ absent de la réponse → REJET, retry.
Si ⟦ph_0⟧ dupliqué → REJET, retry.
```

---

## Patterns à tokeniser — Regex Rust

### Groupe A — Avec argument numérique (communs MV + MZ)
```rust
// \V[n]  \N[n]  \P[n]  \C[n]  \I[n]
r"\\[VNPCI]\[\d+\]"
```

### Groupe B — Sans argument (communs MV + MZ)
```rust
// \G  \\  \$  \.  \|  \!  \>  \<  \^  \{  \}
// ✅ Correct pour le crate `regex` — pas d'escape dans les char classes
r"\\[G\\$.|!><^{}]"

// ❌ NE PAS utiliser — escape sequences invalides dans char class
// r"\\[G\\\$\.\|\!\>\<\^\{\}]"
```

### Groupe C — Spécifiques MZ uniquement
```rust
// \PX[n]  \PY[n]  \FS[n]
r"\\(?:PX|PY|FS)\[\d+\]"
```

### Groupe D — Substitutions dynamiques (termes BD)
```rust
// MV : [%1]  [%2]  [%3] ...
r"\[%\d+\]"

// MZ : %1  %2  %3 ... (sans crochets)
// ATTENTION : ne pas confondre avec du texte normal
// Contexte : uniquement dans Terms > Settings > Messages
r"%\d+"
```

### Regex combinée complète (ordre important)
```rust
// Toujours tester Groupe C avant Groupe A pour éviter
// que \P capture \PX et \PY
const PLACEHOLDER_REGEX: &str = r"(?x)
    \\(?:PX|PY|FS)\[\d+\]  # Groupe C — MZ spécifique (avant Groupe A)
  | \\[VNPCI]\[\d+\]        # Groupe A — avec argument
  | \\[G\\$.|!><^{}]        # Groupe B — sans argument (✅ valide crate regex)
  | \[%\d+\]                # Groupe D — MV substitution
  | %\d+                    # Groupe D — MZ substitution
";
```

---

## Tableau complet des placeholders

| Code | Moteur | Tokeniser ? | Résultat traduisible ? | Note |
|------|--------|------------|----------------------|------|
| `\V[n]` | MV + MZ | ✅ Oui | ⚠️ Parfois | Si la variable contient du texte JP → à traduire en DB séparément |
| `\N[n]` | MV + MZ | ✅ Oui | ✅ Oui | Nom d'acteur → traduire dans la DB acteurs |
| `\P[n]` | MV + MZ | ✅ Oui | ✅ Oui | Nom du membre d'équipe → idem \N |
| `\G` | MV + MZ | ✅ Oui | ✅ Oui | Unité monétaire → traduire dans Terms > Currency |
| `\C[n]` | MV + MZ | ✅ Oui | ❌ Non | Couleur uniquement — aucun texte |
| `\I[n]` | MV + MZ | ✅ Oui | ❌ Non | Icône — aucun texte |
| `\{` | MV + MZ | ✅ Oui | ❌ Non | Augmente taille texte |
| `\}` | MV + MZ | ✅ Oui | ❌ Non | Diminue taille texte |
| `\\` | MV + MZ | ✅ Oui | ❌ Non | Backslash littéral |
| `\$` | MV + MZ | ✅ Oui | ❌ Non | Ouvre fenêtre or |
| `\.` | MV + MZ | ✅ Oui | ❌ Non | Pause 1/4 seconde |
| `\|` | MV + MZ | ✅ Oui | ❌ Non | Pause 1 seconde |
| `\!` | MV + MZ | ✅ Oui | ❌ Non | Attend bouton |
| `\>` | MV + MZ | ✅ Oui | ❌ Non | Affichage instantané |
| `\<` | MV + MZ | ✅ Oui | ❌ Non | Annule affichage instantané |
| `\^` | MV + MZ | ✅ Oui | ❌ Non | Pas de validation en fin |
| `\PX[n]` | MZ only | ✅ Oui | ❌ Non | Position X du texte |
| `\PY[n]` | MZ only | ✅ Oui | ❌ Non | Position Y du texte |
| `\FS[n]` | MZ only | ✅ Oui | ❌ Non | Taille de police |
| `[%n]` | MV only | ✅ Oui | ✅ Oui | Terms > Messages — valeur dynamique |
| `%n` | MZ only | ✅ Oui | ✅ Oui | Terms Settings — valeur dynamique |

---

## Cas limites à gérer

### 1. Ordre de priorité regex
`\PX[n]` doit être testé AVANT `\P[n]` — sinon `\P` capture le `P`
de `\PX` et laisse `X[n]` comme texte brut.

### 2. `%n` en MZ — risque de faux positifs
`%1` en contexte Terms est un placeholder.
`100%` dans du texte libre (ex: "efficacité à 100%") n'en est PAS un.
Stratégie : tokeniser `%\d+` uniquement dans les fichiers
`data/System.json` (section `terms`) et non dans les dialogues.

### 3. `\\` (double backslash)
Représente un backslash littéral dans le jeu.
Dans le JSON RPG Maker, il est stocké `\\\\` (4 backslashes).
Le tokenizer travaille sur la valeur déjà parsée du JSON
(donc `\\` = 2 chars après parsing JSON).

### 4. Placeholders imbriqués
Cas théorique : `\C[\V[3]]` — peu fréquent mais possible dans
certains jeux avec plugins. Gérer avec une regex non-greedy.

### 5. Regex Groupe B — note d'implémentation
Dans le crate `regex` de Rust, les caractères spéciaux suivants
n'ont PAS besoin d'être échappés à l'intérieur d'une classe `[...]` :
`!`, `>`, `<`, `^` (sauf en début de classe), `{`, `}`, `|`, `.`
Utiliser les caractères directs — les backslashes superflus
déclenchent `clippy::invalid_regex`.

---

## Comportement attendu du tokenizer Rust

```rust
// Interface publique attendue
pub struct Tokenizer;

impl Tokenizer {
    // Remplace tous les placeholders par des tokens opaques ⟦ph_N⟧
    // Retourne le texte tokenisé + la map index→placeholder original
    pub fn tokenize(text: &str, engine: Engine) -> Tokenized;

    // Restaure les tokens vers les placeholders originaux
    // Retourne Err si un token manque ou est dupliqué
    pub fn restore(tokenized: &str, map: &PlaceholderMap) -> Result<String, TokenizerError>;

    // Valide qu'une réponse LLM contient tous les tokens attendus
    pub fn validate(response: &str, map: &PlaceholderMap) -> Result<(), TokenizerError>;
}

pub enum Engine {
    MvMz,   // Patterns A + B + D[%n]
    MzOnly, // Patterns C + A + B + D[%n sans crochets]
}

pub enum TokenizerError {
    MissingPlaceholder { token: String, original: String },
    DuplicatePlaceholder { token: String },
    InvalidUtf8,
}
```

---

## Tests unitaires obligatoires

```rust
#[cfg(test)]
mod tests {
    // 1. Tokenisation basique \V[n]
    // 2. Tokenisation \PX[n] (MZ) sans capturer \P[n]
    // 3. Round-trip : tokenize → restore = texte original
    // 4. Rejet si token manquant en sortie LLM
    // 5. Rejet si token dupliqué
    // 6. Texte sans placeholder → inchangé
    // 7. Placeholders multiples dans un seul segment
    // 8. %1 dans Terms MZ vs 100% dans texte libre
    // 9. \\ (double backslash) préservé correctement
    // 10. Segment vide → pas d'erreur
}
```

---

## TODO — Core Layer (hors scope tokenizer)

Les placeholders suivants ont un **résultat traduisible en DB** :
- `\N[n]` → noms d'acteurs dans `data/Actors.json`
- `\G` → devise dans `data/System.json` > `currencyUnit`
- `[%1]` / `%1` → valeurs Terms dans `data/System.json` > `terms`

Ces champs doivent être extraits et traduits séparément
par le module `engines/mv_mz/extractor.rs`, pas par le tokenizer.
Le tokenizer les préserve dans les dialogues mais ne les traduit pas.

---

## Historique des corrections

| Date | Correction |
|------|-----------|
| 2026-05-24 | Regex Groupe B : `\\[G\\\$\.\|\!\>\<\^\{\}]` → `\\[G\\$.\|!><^{}]` — escape sequences invalides dans char class détectées par clippy lors de l'implémentation du tokenizer |