# Journal — 2026-11-17 — DD, vérification locale (V1)

**Phase** : F5 (maintenance : mise en commun des doublons)
**Durée estimée** : 1h
**Statut** : 🔄 En cours (V1 fait ; V2 à V7 en attente)
**Outil** : Claude (local, sans compilation)
**Branche et commit de départ** : `main` @ `00b9269`
**Objectif / plan / story** : section V du « Plan — DD » (`tasks/todo.md`). Journaux de la session cloud :
`docs/journal/2026-11-17-dd-back.md` (sur `refactor/dd-l3-wolf-sites`) et `docs/journal/2026-11-17-dd-front.md`
(sur `refactor/dd-l7-front-api`).

---

## Ce qui a été fait

- Constat : les 7 PR sont ouvertes, dans l'ordre du plan. Back : #34 L4 → `main`, #36 L2 → L4, #37 L1 → L2, #38 L5 → L1,
  #39 L3 → L5. Front : #35 L6 → `main`, #40 L7 → L6. Aucune règle d'arrêt déclenchée (journaux cloud).
- Contrôles mécaniques (lecture Git seule, aucune compilation) :
  - tests existants : `git diff --name-status origin/main...<bout de chaîne> -- src-tauri/tests '*.test.ts'
    '*.test.tsx'` ne montre que des `A` (back : corpus L1, caractérisation L5 et leurs fixtures ; front :
    `TerminologyWorkspace.errorToast.test.tsx`, `src/lib/engine.test.ts`) ;
  - modules `mod tests` des fichiers Rust modifiés ou déplacés : identiques à `main` (script de comparaison ; les
    5 écarts apparents venaient d'un `#[cfg(test)]` placé plus haut dans le fichier, et le module de test de
    `font.rs` est déplacé octet pour octet dans `core/text_fit/font.rs`) ;
  - attendus de caractérisation (`text_fit_corpus/{mz,wolf}.json`, `project_context_errors.json`,
    `wolf_sites.json`) : chacun créé par un seul commit, premier commit de son lot, jamais re-béni ;
  - chaînes indépendantes : seul `CHANGELOG.md` est commun ; la chaîne front ne touche pas `src-tauri/`, la chaîne
    back ne touche pas `src/` ; aucune ne touche `tasks/` ;
  - auteur de tous les commits : adresse noreply ;
  - pas de CI GitHub sur ces branches : le gate est celui des journaux cloud (back : 850 tests Rust OK / 15 ignorés,
    clippy 0 ; front : 201 tests Vitest).
- Relecture du code par quatre sous-agents en lecture seule (L1 ; L5 ; L4 + L2 + L3 ; L6 + L7), en comparant
  ancien et nouveau code fonction par fonction : **aucun changement de comportement trouvé**, hors exceptions
  voulues de DD-D2.

## Constats de qualité (sans effet à l'exécution)

- L1 (CONFIRMÉ par Claude) : `core/text_fit/mod.rs:4-5` dit que le module « knows no engine », alors que
  `settings.rs:12-13` importe `MzTextBox`/`WolfTextBox` et que `engine_fit()` choisit l'adaptateur. Doc à corriger.
- L1 : doc de `FitOutcome::TooLong` (« left as translated ») inexacte pour Wolf, qui rend le texte recoupé ; code
  mort mineur (`wolf/layout/fit.rs` branche `Unfitted(None) | LeftAsTranslated` inatteignable, `WolfTokenRules` sans
  effet, `verdict` MZ redondant). Deux lectures du moteur (`engine_fit()` et `FitEngine::of`) : toutes deux passent
  par `Engine::from_db_str` après L5, sans divergence possible des valeurs.
- L5 : `Engine::from_db_str` recalculé plusieurs fois (`llm/pipeline.rs:600`, `commands/export.rs:206`/`:222` dans
  un `filter`, `core/report.rs:122` dans une boucle) ; `apply_font_prefix` garde un `&str`.
- L4 : `docs/architecture.md:72`, `:88`, `:166` décrivent encore `debug_inject_file` (réécrit en V8) ;
  `wolf::injector::inject_all` n'a plus d'appelant en production (gardé : tests existants).
- L6/L7 : commentaire doublé `SegmentGrid.tsx:122-125`, doubles lignes vides (`AppToolbar.tsx:106-107`,
  `SegmentGrid.tsx` vers `:203`), type `Engine` exporté inutilisé (`src/lib/engine.ts:7`). `pnpm lint` ne lance
  pas Prettier, d'où ces écarts non détectés.
- Remarque du relecteur L1 écartée : « `fit_description` hors échelle non signalé » est faux, le journal back le
  note dans les écarts au plan de L1.

## Bilan chiffré (mesuré par Claude, `git show` des bouts de chaîne)

- Lignes de code de production (sans lignes vides, sans commentaires, sans modules `#[cfg(test)] mod` ni fichiers
  de test) : Rust `src-tauri/src` 29 334 (`origin/main`) → 29 488 (bout L3), soit +154 ; front `src` 12 893 → 13 057,
  soit +164. Total : +318 lignes, environ +0,75 %.
- Par lot (Rust) : L4 −191, L2 −18, L1 +387, L5 +33, L3 −57. Front : L6 +8, L7 +156.
- Doublons de l'audit traités : back B1 (supprimé avec `debug_inject_file`), B2, B3, B4, B5, B6, B7 (requêtes
  `projects` dans `commands/` : 24 → 7), B8 (comparaisons de moteur en chaîne : 37 → 3), B11, et §2 `TextBox`
  (C5, C6 sauf descriptions, C8 à C12) ; front F1 (24 sites), F2, F3 (14 → 5 comparaisons), F4, F5, F6, F7, F8,
  D9. Laissés exprès : B9, B10 (« plus tard »), B12 (« laisser »), D2, D4.
- L'audit estimait environ −420 lignes de doublons côté back et −110 pour `TextBox`. L'écart vient des relais
  `pub use` aux anciens chemins (DD-D3), du trait `TextBox` documenté et de ses deux adaptateurs, et de la couche
  `src/lib/api/` (~200 lignes typées).

## Trous de la caractérisation (chemins jugés équivalents à la lecture, jamais exécutés)

- MZ : `TooLong::Unfitted` et exclusions `\PX`, RTL, saut de page ; police nommée chargée et caractères absents.
- Wolf : `Overflow::NewPage`, `\f[n]` au milieu du texte, fenêtre inconnue.
- Les deux : langues sans espaces et `space_before_marks`, `MAX_ROWS`, `min_font_size` ≥ taille de base,
  `vx_ace`/moteur inconnu, `core/report.rs`.
- Caractérisation L5 : seul `future_engine` (moteur inconnu) ; `vx_ace`/`wolf` couverts par équivalence de
  `from_db_str`. L3 : une autre commande à chaînes (103) et une traduction visant un texte filtré, non couverts.

## Vérifications et limites

- Rien compilé ni exécuté en local (leçons des 2026-10-25 et 2026-10-30). Les relectures sont des lectures.
- V2 à V5 impossibles dans ce dossier en l'état : ni `test/` (jeux d'essai), ni `src-tauri/target/` (binaire).
  Il faut les jeux MZ et Wolf, et une compilation lancée par l'utilisateur.

## Prochaine session

- Décisions utilisateur : nettoyage des constats de qualité (avant ou après fusion) ; compléter ou non le corpus ;
  V2 à V5 (jeux, compilation). Puis V7 (fusion par l'utilisateur dans l'ordre des chaînes) et V8.

---
*Journal partagé Codex / Claude — Hoshi2Star*
