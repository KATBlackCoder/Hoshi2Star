# Session — Fix wolfrpg-map-parser 0x09D20000 panic

**Date:** 2026-06-07  
**Status:** Complete — merged to main, pushed

---

## Ce qui a été fait

Corrigé le panic `Unknown command code 09d20000` qui bloquait le parsing de tous les fichiers `.mps` de 月咲流ホノカver1.03.

### Stratégie appliquée

`[patch.crates-io]` avec une copie locale de `wolfrpg-map-parser 0.6.0` dans `src-tauri/vendor/wolfrpg-map-parser/`.

### Fichiers modifiés dans la crate vendored

**`src/command/signature.rs`** — 4 variantes ajoutées dans l'enum `Signature` ET dans le `match` de `Signature::new()` :

| Variante | Signature | Int args |
|----------|-----------|----------|
| `CallEvent4` | `0x04D20000` | 3 |
| `CallEvent5` | `0x08D20000` | 7 |
| `CallEvent6` | `0x09D20000` | **8 ← source du panic** |
| `CallEvent7` | `0x0AD20000` | 9 |

**`src/command/command.rs`** — `CallEvent4 | CallEvent5 | CallEvent6 | CallEvent7` ajoutés au match arm qui dispatch vers `parse_call_common_event`.

### Fichiers modifiés dans le projet

- `src-tauri/Cargo.toml` — section `[patch.crates-io]` ajoutée :
  ```toml
  [patch.crates-io]
  wolfrpg-map-parser = { path = "vendor/wolfrpg-map-parser" }
  ```
- `src-tauri/Cargo.lock` — mis à jour automatiquement par Cargo

---

## Analyse du risque

- `CommonEvent` (type 210, cmd `0xD2`) est du **pur control-flow** — aucun texte traduisible.
- `options.rs` ne paniquera pas : le `string_arguments` byte est 0 pour ces appels, donc la boucle `is_arg_string` n'est jamais invoquée.
- Risque du fix : **zéro** — le parseur existant `parse_call_common_event` → `CommonEventCommand::parse_call_event` → `Event::parse` gère déjà les counts variables via `ArgumentCount::new(argument_count)`.

---

## Résultat des tests

- `cargo test` : **247/247 verts** (même base qu'avant le fix)
- `cargo check` : aucune erreur, 0 warning lié au patch

---

## PR upstream

Non ouvert lors de cette session. À faire :

**Issue title :** "Unknown command code panic for 0x09D20000 (CallCommonEvent with 8 int args)"

**Issue body :**
```
The command signature 0x09D20000 (CommonEvent type 210, 8 integer arguments)
is not in the signature table, causing a panic!() in command.rs.

This is a Call Common Event command — no translatable text, pure control flow.
The fix is to add the missing variant to the Signature enum and dispatch it
the same way as CallEvent2/3.

Affected game: 月咲流ホノカver1.03 (Wolf RPG v2).
```

Also add `0x04D20000`, `0x08D20000`, `0x0AD20000` for completeness (3, 7, 9 int-arg variants).

---

## Prochaine session

- Tester sur les fichiers `.mps` réels de 月咲流ホノカver1.03 (si disponibles via `pnpm tauri dev`)
- Ouvrir la PR upstream sur G1org1owo/wolfrpg-map-parser
- Avancer vers F5 (voir ROADMAP.md)
