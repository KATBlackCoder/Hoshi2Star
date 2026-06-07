# Session — Diagnostic extraction Wolf + migration fork git

**Date:** 2026-06-07  
**Status:** Complete — mergé main, pushé

---

## Contexte

Après le fix du panic `0x09D20000` (session précédente), l'extraction de
月咲流ホノカver1.03 et Densyanai Inko ver2.0 retournait toujours 0 segments.
Objectif : comprendre pourquoi et corriger ce qui pouvait l'être. Objectif
secondaire : remplacer le répertoire `vendor/` (219 fichiers) par une
solution moins lourde.

---

## Diagnostic — pourquoi rien n'est extrait

### Cause 1 (principale, non résolue) — CommonEvent.dat absent du flux

`extract_all_wolf()` n'appelle pas `extract_common_events()`. Cette fonction
est un stub (`Ok(vec![])`) depuis F4-05.

Dans la quasi-totalité des jeux Wolf RPG, **le dialogue est stocké dans
des CommonEvents**, pas dans les événements de map. Les fichiers `.mps`
contiennent principalement des commandes `CallCommonEvent` (pure control
flow, aucun texte). Résultat : même si tous les `.mps` parsent correctement,
l'extraction renvoie 0 segments de dialogue.

**C'est le vrai bloqueur.** L'extraction de `CommonEvent.dat` est
indispensable pour obtenir du texte traduisible sur ces jeux.

### Cause 2 (secondaire) — autres codes inconnus silencieux

Le fix d'hier n'avait ajouté que 4 variantes `0xD2` spécifiques. D'autres
signatures inconnues peuvent encore déclencher un `panic!()`, absorbé par
`catch_unwind` puis logué via `eprintln!` invisible à l'UI. Le fichier est
silencieusement ignoré.

### Cause 3 (potentielle) — databases sans texte extractable

Les `.dat` peuvent parser sans erreur mais ne contenir aucun champ japonais
dans les noms connus (`name`, `description`, etc.), ou les archives ne sont
pas trouvées.

---

## Corrections apportées

### Fix 1 — dispatch D2/D3 générique dans la crate patchée

**Problème :** le wildcard `_` dans `command.rs` panique sur toute signature
inconnue, y compris les variantes `0xD2`/`0xD3` avec des arg counts non
listés dans le tableau des signatures.

**Solution :** ajout de `dispatch_unknown_by_type(sig: u32)` dans `impl Command` :

```rust
fn dispatch_unknown_by_type(sig: u32) -> fn(&[u8], u32) -> (usize, u32, Self) {
    match (sig >> 16) & 0xFF {
        0xD2 => Self::parse_call_common_event,
        0xD3 => Self::parse_reserve_common_event,
        _ => |_bytes, s| panic!("Unknown command code {:08x}", s),
    }
}
```

Le wildcard `_ => panic!(...)` pointe maintenant sur ce helper.
**Toute** signature `0x??D20000` / `0x??D30000` (quel que soit le nombre
d'args) est routée sans panique. Les familles vraiment inconnues paniquent
encore (on ne peut pas les sauter sans connaître leur longueur binaire).

### Fix 2 — migration vendor/ → fork git

**Problème :** `src-tauri/vendor/wolfrpg-map-parser/` ajoutait 219 fichiers
au repo pour un fix de 2 lignes. Lourd, difficile à maintenir.

**Solution :**

1. Fork créé : `https://github.com/KATBlackCoder/wolfrpg-map-parser`  
   Branche : `fix/unknown-call-event-variants`  
   Rev : `efc3b4d`

2. `[patch.crates-io]` dans `src-tauri/Cargo.toml` mis à jour :
   ```toml
   [patch.crates-io]
   wolfrpg-map-parser = { git = "https://github.com/KATBlackCoder/wolfrpg-map-parser", rev = "efc3b4d" }
   ```

3. `src-tauri/vendor/` supprimé du repo (−219 fichiers).

**Procédure de sortie** : quand l'upstream G1org1owo/wolfrpg-map-parser merge
un fix ou sort une `0.6.x`, supprimer la section `[patch.crates-io]` et
bumper `wolfrpg-map-parser = "0.6.x"` dans les dépendances.

---

## Fichiers modifiés

| Fichier | Action |
|---------|--------|
| `src-tauri/Cargo.toml` | `[patch.crates-io]` basculé de `path = vendor/` vers `git = fork` |
| `src-tauri/Cargo.lock` | Mis à jour automatiquement |
| `src-tauri/vendor/` | Supprimé (−219 fichiers) |
| Fork `command.rs` | Wildcard → `dispatch_unknown_by_type()` |
| Fork `signature.rs` | 4 variantes CallEvent4–7 (inchangées depuis hier) |

---

## Tests

- `cargo check` : ✅ propre, aucun warning
- `cargo test` : ✅ **247/247** verts (base inchangée)

---

## Ce qui reste à faire pour extraire du texte

Le vrai travail est `extract_common_events()`. Le stub actuel (`Ok(vec![])`)
doit être remplacé par un parseur réel de `CommonEvent.dat`.

Le format est documenté partiellement dans l'extractor (`extractor.rs:570–580`) :

```
// Per-event: indicator 0x8E, 7 unknown bytes, commands identical to .mps,
// then 100 fixed strings, plus 0x8F/0x91/0x92 sections.
```

La crate `wolfrpg-map-parser` expose déjà un parseur de CommonEvents via
`db_parser/parsers/common_events_parser.rs` — à évaluer si réutilisable.

---

## Prochaine session

- Implémenter l'extraction de `CommonEvent.dat` (vrai bloqueur des 0 segments)
- Évaluer si `wolfrpg_map_parser::db_parser::parsers::common_events_parser` 
  est utilisable ou si une implémentation custom est nécessaire
- Tester sur Honoka et Inko après implémentation
