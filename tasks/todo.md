# En attente (tests manuels utilisateur)

- [ ] Test manuel "Tout traduire" sur un projet multi-fichiers : vérifier que
  le % ne touche 100% qu'à la toute fin (`h2s://llm/completed`), pas après
  chaque fichier. (commit `bf48f02`)
- [ ] Test manuel "Tout traduire" sur CommonEvent.dat (2000+ segments) :
  vérifier que les lignes passent à "Traduit" batch par batch en temps réel.
  (commit `bf48f02`)

# Tâches dormantes (ne pas démarrer sans déclencheur)

- [ ] **Restructuration wolf Phase 2+3** : renommer `v3_format/` → `format_v3/`
  et extraire la glue `wolfrpg_map_parser` (branches v2 inline
  d'`extractor.rs`/`injector.rs`) vers `format_v2/`.
  **Déclencheur** : un bug v2, une feature v2, ou l'extension du support v1.
  Décision 2026-06-12 : différé — code v2 vert (tests réels Honoka), refacto
  purement cosmétique sans travail v2 planifié. Phase 1 (`decrypt/`) faite.
- [ ] **ESLint 10 sans `eslint.config.js`** : `pnpm lint` cassé (préexistant,
  migration flat config à faire).

# Hors scope notés

- Edge case % > 100% si des segments changent de statut entre le COUNT initial
  et les SELECT par-fichier — risque jugé négligeable (desktop mono-utilisateur).
