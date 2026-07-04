# Journal — 2026-07-04 — Tokenizer : name box `\n<Name>` scindée (nom traduisible)

**Phase** : F4/F5 (maintenance LLM layer)
**Durée estimée** : 2h
**Statut** : ✅ Complété

---

## Ce qui a été fait

- Diagnostic : le Groupe G (`\\n<[^>]+>`) de `RE_MVMZ`/`RE_MZONLY` tokenisait le
  code name box ENTIER → le nom (ex. ハルカ) était opaque pour le LLM et restait
  en japonais dans le jeu exporté.
- Tokenisation scindée en 2 passes dans `Tokenizer::tokenize` :
  - **Passe 1** (MvMz + MzOnly uniquement, pas Wolf) : `RE_NAMEBOX = \\n<([^>]+)>`
    émet une paire de tokens — `⟦ph_i⟧` = `\n<`, `⟦ph_i+1⟧` = `>` — le nom reste
    inline et traduisible (le glossaire s'applique).
  - **Passe 2** : regex moteur (Groupe G retiré) sur le résultat, ce qui tokenise
    aussi les codes imbriqués dans la name box (`\n<\C[6]ハルカ>`).
- Garde-fou nouveau risque (LLM inverse/perd les tokens de paire) :
  `TokenizerError::BrokenNameBox` — contrôle structurel post-restore, le nb de
  matchs `RE_NAMEBOX` dans le texte restauré doit être ≥ au nb d'ouvrants `\n<`
  de la map, sinon rejet → retry pipeline existant (validate ne vérifie que
  présence/unicité, pas l'ordre).
- Tests 15/16/17 réécrits (comportement scindé) + 6 nouveaux : codes imbriqués,
  name boxes multiples, MzOnly, tokens de paire inversés → rejet, Wolf non
  concerné, name-box pur → `needs_translation == true` (filter.rs).

## Fichiers créés

- (aucun)

## Fichiers modifiés

- `src-tauri/src/llm/tokenizer.rs` — `RE_NAMEBOX` + passe 1 ; Groupe G retiré de
  `RE_MVMZ`/`RE_MZONLY` ; `TokenizerError::BrokenNameBox` + contrôle dans
  `restore` ; tests 15/16/17 réécrits + 6 nouveaux.
- `src-tauri/src/engines/filter.rs` — test `test_needs_translation_pure_name_box_is_translatable`.
- `CHANGELOG.md` — entrée Fixed.
- `tasks/todo.md` — section cochée.

## Fichiers supprimés

- (aucun)

## Dépendances ajoutées

- (aucune)

## Décisions prises

- Scinder plutôt que dé-tokeniser : la structure `\n<`/`>` reste protégée (paire
  de tokens), seul le contenu devient visible au LLM — pas de risque de perdre
  le code lui-même.
- Wolf exclu de la passe 1 : `\n<…>` n'est pas un code Wolf.
- Garde structurel dans `restore` (pas dans `validate`) : `validate` garantit
  présence/unicité des tokens, l'inversion de paire n'est détectable qu'après
  restauration.

## Problèmes rencontrés

- Impacts vérifiés en amont (plan) : `qa.rs` compare `\n<`+`>` au lieu du nom JP
  verbatim — cohérent ; `filter.rs::needs_translation` — un segment name-box pur
  devient traduisible (comportement voulu, testé) ; signatures pipeline/split
  inchangées.

## Tâches ROADMAP cochées

- (aucune — bugfix hors roadmap)

## Prochaine session

- Vérification live optionnelle : traduire un segment `\n<ハルカ>…` réel via
  l'app dev et confirmer que le nom sort traduit (glossaire ハルカ→Haruka).
- Backlog inchangé : 8 lints clippy --all-targets code de test, refactors warn
  (set-state-in-effect, cache qa-check), push de `bd6d898` + ce fix.

---
*Généré par Claude Code — Hoshi2Star*
