# Restructuration du module `engines/wolf/` par responsabilité — PLAN

## Objectif

Découper `engines/wolf/` selon les **vrais axes de variation** (déchiffrement vs
format texte), PAS par version (`legacy`/`moderne`/`pro` rejeté : `pro` n'a aucun
code propre, partage 100 % du parsing avec `moderne` → dossier fantôme). Coutures
cibles :

- `decrypt/` — variantes de déchiffrement (l'axe qui va grossir avec WolfX)
  - `legacy_xor.rs` = ex-`decryptor.rs` (XOR DXA v5/v6/v8, natif, déjà testé)
  - `wolfx.rs` = **stub** établissant la couture (erreur de guidage « passe par
    UberWolf »), SANS crypto réelle — le câblage effectif reste la tâche F5 WolfX.
- `format_v2/` — glue fine au-dessus de `wolfrpg_map_parser` (extrait des
  branches v2 inline d'`extractor.rs`/`injector.rs`)
- `format_v3/` = ex-`v3_format/` (parser maison, renommé pour la symétrie)
- Partagés inchangés : `extractor.rs`, `injector.rs` (orchestrateurs,
  agnostiques à la version), `dat_parser.rs`, `encoding.rs`, `placeholders.rs`

Invariant : `extractor`/`injector` continuent de dispatcher par **détection sur
octets** (`is_lz4_v3`), jamais par mode choisi par l'utilisateur. Aucun
changement de comportement, aucun changement d'API côté commands TS.

## Contraintes / points de contact (relevés)

- Réfs externes au module renommé : **`detector.rs:153`** importe
  `wolf::decryptor::{...}` → à mettre à jour. `commands/project.rs` (→`extractor`)
  et `commands/export.rs` (→`injector`) ne touchent QUE les orchestrateurs →
  inchangés.
- API publique de `decryptor.rs` à préserver : `extract_all`, `DecryptorError`,
  `WolfArchive`, `WolfFile`, `known_key`, `read_signature`, `read_header`,
  `guess_key_v6`.
- `V3FormatError` : nom de type conservé (seul le **chemin de module** change),
  pour limiter le churn.
- Stratégie d'imports : chemins explicites (`decrypt::legacy_xor::…`) plutôt que
  re-exports larges, pour la lisibilité — peu de sites concernés.

## Phase 1 — `decrypt/` namespace (risque faible, valeur haute)

- [ ] `git mv` `wolf/decryptor.rs` → `wolf/decrypt/legacy_xor.rs`
- [ ] Créer `wolf/decrypt/mod.rs` : `pub mod legacy_xor; pub mod wolfx;`
- [ ] Créer `wolf/decrypt/wolfx.rs` : stub documenté + `WolfXError` de guidage +
      `fn decrypt(_: &[u8]) -> Result<…, WolfXError>` renvoyant l'instruction
      UberWolf. NON câblé dans le flux (couture seulement).
- [ ] `wolf/mod.rs` : retirer `pub mod decryptor;`, ajouter `pub mod decrypt;`
- [ ] MAJ refs internes : `extractor.rs` (`super::decryptor::extract_all` →
      `super::decrypt::legacy_xor::extract_all` ; `super::decryptor::DecryptorError`
      → `…::legacy_xor::DecryptorError`)
- [ ] MAJ ref externe : `detector.rs:153` (`wolf::decryptor::{…}` →
      `wolf::decrypt::legacy_xor::{…}`)
- [ ] `cargo fmt && cargo clippy -D warnings && cargo test engines::wolf` → vert

## Phase 2 — renommer `v3_format/` → `format_v3/` (rename pur)

- [ ] `git mv` `wolf/v3_format/` → `wolf/format_v3/`
- [ ] `wolf/mod.rs` : `pub(crate) mod v3_format;` → `pub(crate) mod format_v3;`
- [ ] Remplacer `v3_format` → `format_v3` dans `extractor.rs` et `injector.rs`
      (imports + tous les `v3_format::…`). `V3FormatError` inchangé.
- [ ] `cargo fmt && cargo clippy -D warnings && cargo test engines::wolf` → vert

## Phase 3 — extraire `format_v2/` (étape la plus invasive)

- [ ] Créer `wolf/format_v2/mod.rs` exposant une surface parallèle à `format_v3` :
      parse/inject map + common_events (+ database) au-dessus de
      `wolfrpg_map_parser`.
- [ ] Déplacer les branches v2 inline (`else` non-LZ4) d'`extractor.rs`
      (`extract_map_segments`, `extract_common_events`, `extract_database_segments`)
      vers des appels `format_v2::…`.
- [ ] Idem pour les branches d'injection v2 d'`injector.rs`.
- [ ] Relocaliser/ajuster les tests v2 concernés (garder les tests d'intégration
      fichiers réels Honoka là où ils prouvent le round-trip).
- [ ] `cargo fmt && cargo clippy -D warnings && cargo test engines::wolf` → vert
- Note : Phase 3 est séparable (commit distinct). Si le risque/temps l'exige,
  livrer Phases 1–2 d'abord ; v2 reste fonctionnel inline en attendant.

## Phase 4 — Vérification & docs

- [ ] Gate complet : `pnpm typecheck` (aucun changement TS attendu) +
      `cargo clippy -D warnings` + `cargo test` → 303 attendus
- [ ] MAJ `docs/architecture.md` (tableau : `decryptor.rs` → `decrypt/legacy_xor.rs`
      + `decrypt/wolfx.rs` ; `v3_format` → `format_v3` ; nouveau `format_v2`)
- [ ] MAJ mémoire `project_wolf_v3_status.md` (nouvelle arborescence)
- [ ] `git status` : vérifier que seuls les fichiers Wolf attendus bougent
      (pas de débordement de scope vers les changements pipeline en cours)

## Hors scope (explicitement)

- Implémentation réelle du déchiffrement WolfX (ChaCha20) — reste la tâche F5.
- Sélecteur de version côté UI — rejeté (auto-détection conservée).
- Renommage de `V3FormatError`/autres types publics — non nécessaire.

---

# Progression globale "Tout traduire" (corriger le 100% prématuré) — DONE

## Problème

`pipeline::run_inner` calculait `total = segments.len()` **par appel** (donc par
fichier dans `translate_all_segments`), et émettait `h2s://llm/progress { done, total }`
avec ces valeurs locales. Le frontend (`llm.ts`) calcule
`pct = round(done/total*100)`, donc la barre atteignait 100% à la fin de **chaque
fichier**, pas seulement à la fin de "Tout traduire".

## Réalisé

- [x] `llm/pipeline.rs` :
  - `run_inner` : nouveau paramètre `global_progress: Option<(usize, usize)>`
    (`(done_offset, global_total)`) entre `cooldown` et `on_progress`.
  - `(emit_done, emit_total)` calculés une fois par batch et utilisés à la
    fois pour `on_progress(...)` et pour `ProgressPayload` émis.
  - `run` transmet `global_progress` à `run_inner`.
  - `#[allow(clippy::too_many_arguments)]` ajouté (8 params).
  - 10 sites de tests existants mis à jour (`None,` ajouté).
  - Nouveau test `test_global_progress_offset_accumulates_across_files` :
    vérifie que `done` continue depuis l'offset du fichier précédent sans
    redescendre (3/10 → 10/10).

- [x] `commands/translate.rs` :
  - `translate_segments` : `pipeline::run(pairs, ..., None, None)` (inchangé
    fonctionnellement — total local correct pour un fichier/sélection unique).
  - `translate_all_segments` :
    - `global_total = total_untranslated as usize` (déjà calculé au début).
    - `done_offset` initialisé à 0 avant la boucle fichiers.
    - `pair_count = pairs.len()` capturé avant le move dans `pipeline::run`.
    - `pipeline::run(pairs, ..., Some(&mut cooldown), Some((done_offset, global_total)))`.
    - `done_offset += pair_count` après un `Ok(_)`.

## Vérification

- [x] `cargo fmt --manifest-path src-tauri/Cargo.toml`
- [x] `cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings` → clean
- [x] `cargo test --manifest-path src-tauri/Cargo.toml` → 303 passed, 0 failed
- [ ] Test manuel "Tout traduire" sur un projet multi-fichiers : vérifier que
  le % ne touche 100% qu'à la toute fin (`h2s://llm/completed`), pas après
  chaque fichier — à faire par l'utilisateur.

## Hors scope (non traité ici)

- Si `done_offset + done` dépasse `total_untranslated` (edge case si des
  segments ont changé de statut entre le COUNT initial et les SELECT
  par-fichier), le % pourrait dépasser 100% — pas géré, risque jugé négligeable
  pour un usage desktop mono-utilisateur.

---

# Rafraîchissement progressif de SegmentGrid pendant la traduction — DONE

## Problème
`SegmentGrid` ne rechargeait les segments que sur `h2s://llm/completed`
(fin de tout le run). Pour un fichier de 2000+ segments, rien ne s'affichait
avant la fin complète de "Tout traduire".

## Réalisé

- [x] `llm/progress.rs` : nouveau `SegmentUpdatePayload { id, target_text, status }`
  (camelCase), pour `h2s://llm/segments-updated`.
- [x] `llm/pipeline.rs` :
  - `result_status(r: &TranslationResult) -> &'static str` factorisé
    (utilisé par `persist_batch_results` et la nouvelle émission).
  - Après `persist_batch_results`, émission de `h2s://llm/segments-updated`
    avec `Vec<SegmentUpdatePayload>` (un par segment du batch).
  - Doc du module mis à jour (étape 7 du flow).
- [x] `lib/types.ts` : nouveau type `SegmentUpdate { id, targetText, status }`.
- [x] `SegmentGrid.tsx` : nouveau listener sur `h2s://llm/segments-updated`,
  fusionne `targetText`/`status` dans `segments` via `setSegments(prev => prev.map(...))`
  — pas de requête DB, pas de perte de tri/sélection/scroll. No-op pour les
  segments d'un autre fichier (id ne matche pas).

## Vérification

- [x] `cargo fmt` / `cargo clippy -D warnings` / `cargo test` → 303 passed
- [x] `pnpm typecheck` → OK
- [ ] `pnpm lint` → préexistant : ESLint 10 sans `eslint.config.js` (pas lié à
  ce changement, à traiter séparément)
- [ ] Test manuel "Tout traduire" sur CommonEvent.dat (2000+ segments) :
  vérifier que les lignes passent à "Traduit" batch par batch en temps réel —
  à faire par l'utilisateur.
