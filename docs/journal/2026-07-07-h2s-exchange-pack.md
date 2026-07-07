# Journal — 2026-07-07 — Pack d'échange `.h2s` (partage/reprise de traduction)

**Phase** : F4/F5 (format `.h2s` prévu de longue date dans la couche Export)
**Durée estimée** : 4h (spec + backend session précédente, UI + vérif cette session)
**Statut** : ✅ Complété (gate vert + vérifié live MCP)

---

## Ce qui a été fait

- Feature complète « pack d'échange `.h2s` » : un traducteur exporte l'état
  complet de sa traduction (segments + statuts + glossaire projet ± TM) dans un
  ZIP `<Jeu>.h2s` ; un autre utilisateur l'importe sur SA copie du jeu et
  reprend là où le premier s'est arrêté. Aucun fichier du jeu ne circule.
- Backend (session précédente) :
  - `core/h2s_pack.rs` — types serde + writer/reader ZIP. Anti-zip-bomb :
    seules les 3 entrées connues sont lues, en mémoire, plafond 256 Mo
    décompressés, jamais d'extraction disque. Champs JSON inconnus ignorés
    (compat versions futures). Segments à cible vide rejetés à la lecture.
  - `commands/pack.rs` — module dédié (plutôt que gonfler export.rs/project.rs) :
    `export_h2s_pack`, `preview_h2s_import` (dry-run obligatoire, `ImportBlocker`
    typé : notAPack / unsupportedVersion / engineMismatch / langPairMismatch /
    invalidPack), `apply_h2s_import` (backup auto `backup-avant-import-*.h2s`
    dans `<app_data>/backups/` PUIS une seule transaction SQLite, événement
    `h2s://project/import-done`).
  - Identité inter-machines : `(fileName, jsonKey)` + appariement ordinal pour
    les clés dupliquées (n-ième ↔ n-ième, ordre `ORDER BY file_name, rowid`
    identique des deux côtés) — jamais les UUID locaux.
  - Politiques : `fill` (défaut), `overwrite_except_reviewed`, `overwrite_all` ;
    source modifiée → jamais appliqué par défaut, opt-in forcé `needs_review` ;
    glossaire importé en termes PROJET (terme local gagnant sur conflit) ;
    TM en `ON CONFLICT DO NOTHING` (la TM locale gagne toujours).
- Frontend (cette session) :
  - `PackExportDialog.tsx` (checkbox TM opt-in → save dialog → toast avec
    compteurs), `PackImportWizard.tsx` (rapport dry-run + Select politique +
    confirmation forte « tout écraser » via checkbox destructive + cases
    conditionnelles source-modifiée/glossaire/TM → rapport final avec chemin
    du backup), 2 boutons icône dans la toolbar (Share2/PackageOpen, visibles
    seulement projet ouvert — couvre la validation « aucun projet ouvert »).
  - `SegmentGrid` : le listener de refresh `h2s://llm/completed` couvre
    maintenant aussi `h2s://project/import-done` (même callback).
  - Types TS miroirs (`ImportPreview`/`ImportReport`/`ImportBlocker` union
    discriminée/`ImportPolicy`), i18n `pack.*` FR/EN complet.

## Vérification

- Gate : typecheck ✅ · **55 Vitest (+3 wizard)** ✅ · lint 0 err (8 warn
  préexistants) ✅ · clippy 0 ✅ · **418 Rust (+16 pack) + 3 intégration** ✅.
- Tests Rust : round-trip, ZIP invalide/sans manifest, formatVersion futur,
  statut invalide, cibles vides filtrées, champs inconnus, appariement ordinal
  des doublons, 3 politiques, source modifiée, idempotence, glossaire
  dédup/conflits, TM jamais écrasée.
- Tests Vitest wizard : dry-run → apply (politique défaut `fill`) → rapport ;
  « tout écraser » bloqué tant que la confirmation n'est pas cochée
  (stubs jsdom `hasPointerCapture`/`scrollIntoView` requis pour Radix Select) ;
  blocker moteur → message dédié sans bouton Appliquer.
- **Vérif live MCP** (projet Wolf réel Densyanai_Inko, 1948 segments) :
  export réel 1948 seg/4 fichiers/12 TM (ZIP inspecté) → wipe d'un segment →
  preview « 1 applicable / 1947 identiques » → apply `fill` → segment restauré
  (« Start », statut conservé), backup écrit → réimport idempotent (0 appliqué,
  1948 identiques) → `fill` préserve un conflit local et la grille se
  rafraîchit bien sur l'événement import-done → `overwrite_except_reviewed`
  restaure l'original → refus typés vérifiés : pack Wolf sur projet MV/MZ
  (engineMismatch) et `package.json` (notAPack). Dialog export vérifié à
  l'écran (screenshot).

## Décisions

- Module `commands/pack.rs` dédié au lieu des emplacements esquissés dans la
  spec (`export.rs`/`project.rs`) : les 3 commandes partagent
  `build_project_pack`/`load_local_index`/`classify`, la cohésion prime.
- TM embarquée en `tm.json` (même sérialisation serde que le reste du pack)
  plutôt que le `.tmx` esquissé dans la spec : pas d'interop externe requise
  ici, l'import TMX standard reste le job du TMPanel.
- Le backup pré-import est un pack `.h2s` normal → l'annulation d'un import
  est « réimporter le backup en tout écraser », zéro mécanique d'undo en base.

## Suites possibles (non planifiées)

- Bouton « restaurer un backup » listant `<app_data>/backups/`.
- Détail par fichier dans le rapport dry-run (actuellement compteurs globaux).
