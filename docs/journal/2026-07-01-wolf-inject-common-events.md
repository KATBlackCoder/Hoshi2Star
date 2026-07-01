# Journal — 2026-07-01 — Phase 1 remédiation : réinjection Common Events Wolf

**Phase** : F4
**Durée estimée** : 3h
**Statut** : ✅ Complété

---

## Ce qui a été fait

Phase 1 du plan de remédiation issu de l'audit du 2026-07-01
(`docs/audit-remediation-plan-2026-07-01.md`) — 🔴 P0 perte de données.

- **Bug confirmé sur le code réel** : les traductions Common Events Wolf étaient
  extraites, traduites, comptées par la gate d'export, mais **jamais réinjectées**
  dans le ZIP. `inject_all_to_memory`/`inject_all` ne matchaient que
  `MapData`/`Database` (`_ => {}`), et `export.rs` regroupait les clés
  `CommonEvents/{event_name}` **par nom d'événement** alors que tous partagent
  l'unique `CommonEvent.dat` (un arm naïf aurait produit du last-write-wins).
- **`injection_bucket(key)`** (injector.rs) : mappe une clé de segment vers son
  fichier physique ; tous les `CommonEvents/*` collapsent dans un bucket unique
  `CommonEvents/CommonEvent`. Utilisé par les **2** sites de bucketing d'`export.rs`.
- **`inject_common_events`** : v2.x (Honoka) → `common_events_parser` +
  `patch_common_events_strings` + splice binaire ; v3.5 (Inko, LZ4) →
  `decompress`/`parse`/mute `string_args`/`dump`/`recompress`.
- Splice séquentiel extrait en **`splice_wolf_strings`**, partagé avec le path `.mps`.
- Arm `"CommonEvents"` ajouté dans `inject_all_to_memory` **et** `inject_all`.
- **Tests** : synthétiques (v2 construit à la main, v3 via `synthetic_messages`) +
  **fichiers réels** Inko (v3.5) et Honoka (v2) fournis par l'utilisateur.

## Fichiers créés

- `docs/journal/2026-07-01-wolf-inject-common-events.md` — cette entrée

## Fichiers modifiés

- `src-tauri/src/engines/wolf/injector.rs` — `injection_bucket`,
  `inject_common_events` (+ `_v3`), `patch_common_events_strings`,
  `splice_wolf_strings` (extrait de `patch_mps_strings`), arms `"CommonEvents"`,
  7 tests (synthétiques + réels)
- `src-tauri/src/engines/wolf/v3_format/common_events.rs` — `#[cfg(test)]`
  `CommonEventsV3::synthetic_messages` ; `recompress`/`dump`/`is_utf8`/`is_v35`
  ne sont plus `#[allow(dead_code)]` (utilisés par l'injecteur)
- `src-tauri/src/commands/export.rs` — les 2 sites de bucketing Wolf routent via
  `injection_bucket`
- `docs/architecture.md` — section `wolf/injector.rs` (CommonEvent.dat +
  `injection_bucket` + `splice_wolf_strings`) + date
- `CHANGELOG.md` — entrée Fixed
- `tasks/todo.md` — suivi Phase 1

## Fichiers supprimés

- (aucun)

## Dépendances ajoutées

- (aucune)

## Décisions prises

- **Bucket unique pour les Common Events** : `CommonEvents/CommonEvent` — décision
  structurante car `CommonEvent.dat` est un fichier unique, contrairement aux
  `.mps`/`.dat` (un par stem). Non ADR-worthy (détail d'implémentation interne).
- **v2 vérifié par fixture synthétique construite à la main** en plus des fichiers
  réels : reverse-engineering du layout `CommonEvent::parse` du parser forké
  (`wolfrpg-map-parser`) pour ne pas dépendre uniquement de la présence des jeux.
- **Fallback non retenu** : exclure les CE de l'extraction (plan §Phase 1) — inutile,
  l'injection v2+v3 s'est avérée faisable et testable.

## Problèmes rencontrés

- **Path des types du parser forké** : `CommonEvent` est à
  `wolfrpg_map_parser::db_parser::common_event::CommonEvent` (re-export), pas
  `wolfrpg_map_parser::common_event`.
- **Fixtures réelles absentes puis fournies** : les 2 tests
  `test_real_inko_*_round_trip` paniquaient (fixture Inko absente). L'utilisateur a
  fourni les jeux ; renommés `Densyanai_Inko_ver2.0` / `月咲流ホノカver1.03` pour
  matcher les 13 refs de tests existantes (`test/` est gitignoré).
- **Identity v3** : LZ4 recompression n'est pas byte-stable → l'invariant testé est
  l'égalité du **payload décompressé** (comme `test_real_inko_common_events_v3_round_trip`).

## Tâches ROADMAP cochées

- [ ] (aucune tâche ROADMAP formelle — fix de correction hors backlog planifié)

## Prochaine session

- Phase 2 — 🟠 Éliminer les paniques des parsers binaires Wolf (`legacy_xor.rs`
  L731/591/1214+, `checked_add`, bornes `dat_parser.rs`). Filet : round-trips
  byte-exacts existants.

---
*Généré par Claude Code — Hoshi2Star*
