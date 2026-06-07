# Session — Wolf CommonEvent.dat extraction

**Date:** 2026-06-07  
**Status:** Complet — mergé main, pushé

---

## Contexte

Suite de la session précédente. Deux décisions prises avant de coder :

1. **Suppression du fork git + `[patch.crates-io]`** — les commandes D2/D3
   (CallCommonEvent/ReserveCommonEvent) sont du control flow pur. `catch_unwind`
   absorbe déjà les panics. Fork inutile pour le vrai bloqueur.

2. **Implémentation de `extract_common_events()`** — le vrai bloqueur des
   0 segments extraits.

---

## Analyse — common_events_parser (wolfrpg-map-parser 0.6.0 vanilla)

### Q1 — API publique

`wolfrpg_map_parser::common_events_parser::parse_bytes(bytes: &[u8]) -> Vec<CommonEvent>`
— `pub`, exposé via `pub use` dans `lib.rs`. Prend les bytes directement.

### Q2 — Accès aux commandes

`CommonEvent::commands() -> &Vec<Command>` — `pub`. C'est le **même enum
`Command`** que dans les `.mps`. `Command::ShowMessage` et `Command::ShowChoice`
fonctionnent identiquement. `CommonEvent::event_name() -> &str` disponible
pour le nommage des clés.

### Q3 — SJIS

Géré entièrement par la crate — `byte_utils::parse_string` appelle
`SHIFT_JIS.decode()`. Toutes les strings sortent en UTF-8. Rien à décoder
manuellement.

### Q4 — Panics potentiels

Quatre points de panic dans `parse_bytes` : magic header, event signature,
command count mismatch, end signature. Plus les panics hérités de
`Command::parse_multiple`. **`catch_unwind` obligatoire.**

**→ CAS A : API directement utilisable.**

---

## Implémentation

### Fichiers modifiés

| Fichier | Action |
|---------|--------|
| `src-tauri/src/engines/wolf/extractor.rs` | Implémentation complète |
| `src-tauri/Cargo.toml` | `[patch.crates-io]` supprimé |
| `src-tauri/Cargo.lock` | Mis à jour automatiquement |
| `CHANGELOG.md` | Mis à jour |

### Changements dans extractor.rs

- **Import** : `use wolfrpg_map_parser::{command::Command, common_events_parser, Map}`
- **Nouveau variant** : `WolfSegmentKind::CommonEventMessage { event_name, event_idx, cmd_idx }`
- **Nouveau helper** : `load_common_event_bytes(game_dir)` — cherche
  `Data/BasicData/CommonEvent.dat` puis `.wolf` archives
- **Stub remplacé** : `extract_common_events(bytes, version)` — `catch_unwind`
  + itération events → commands → filtre ShowMessage/ShowChoice
- **Clé format** : `CommonEvents/{event_name}/{event_idx}/{cmd_idx}`
  (choix : `choices/{choice_idx}` en suffixe)
- **Câblage** : ajouté dans `extract_all_wolf` (type `"wolf_common_events"`)
  et `extract_wolf_project` (remplace le bloc manuel par `load_common_event_bytes`)

---

## Tests

- `test_extract_common_events_empty` — magic valide, 0 events → `Ok(vec![])`
- `test_extract_common_events_invalid_magic_no_panic` — garbage bytes →
  `Err(...)`, pas de panic processus

**249/249 verts** (247 base + 2 nouveaux)

---

## Résultat sur jeux réels

Non testé — pas de fichier `CommonEvent.dat` déchiffré disponible localement.
Tests E2E requis avec Honoka ou Inko pré-déchiffrés.

---

## Prochaine session

- Test E2E sur 月咲流ホノカver1.03 ou Densyanai Inko — vérifier que des
  segments apparaissent dans la grille après ouverture
- Compter le nombre de segments extraits (attendu : centaines)
- Si 0 segments : vérifier que `CommonEvent.dat` est présent dans les archives
  (peut être dans une archive `.wolf` avec un nom différent)
