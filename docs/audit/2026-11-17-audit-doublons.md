# Audit des doublons et répétitions — 2026-11-17 (Claude)

**Périmètre :** `src/` (React/TypeScript, hors `src/components/ui/`) et `src-tauri/src/` (Rust). Lecture du code seulement : aucun fichier applicatif modifié, rien de commité.
**Base :** `main` @ `1f2b841`, arbre de travail propre au départ. Les références `fichier:ligne` valent pour ce commit.
**Objectif utilisateur :** remplacer le code répété par des fonctions communes, adaptables par chaque usage. Cet audit sert de base : chaque mise en commun fera ensuite l'objet d'un lot, avec un plan validé et une PR, sans changement de comportement. Premier lot : la mise en page du texte (`TextBox`).

## Méthode et limites

- **Analyse statique seulement.** Aucun build, aucun test, aucun `clippy` : ils font geler ce PC (leçon du 2026-10-30). La couverture de test est relevée par nom de fichier et de test, pas par exécution.
- **Copier-coller :** `jscpd` n'a pas été lancé (il demandait un téléchargement, donc un accord). À la place, un script Python local a comparé les lignes normalisées (espaces réduits ; lignes vides, commentaires, imports et accolades seules ignorés ; tests exclus) et cherché les suites identiques d'au moins 7 lignes en Rust et 6 en TypeScript. Résultat brut : 76 paires en Rust (737 lignes), 93 paires en TypeScript (717 lignes, dont l'essentiel n'est que des listes d'imports de dialogues). Ce script ne voit pas les clones dont un identifiant a été renommé : `jscpd` reste une option si tu veux un second passage.
- **Logique dupliquée :** deux sous-agents (back, front) ont lu le code. Leurs constats retenus ici ont été revérifiés un par un (`diff`, `rg`, lecture). Le cas `TextBox` (§2) a été lu directement et en entier.
- **Statuts :** **vérifié** = relu dans le code ; **supposé** = plausible, non démontré. Les nombres de lignes gagnées sont des estimations.

## 1. Résumé

1. **Mise en page du texte (§2).** Les primitives sont déjà partagées, mais elles habitent `engines/mv_mz/layout/` : Wolf, `core/` et `commands/` importent depuis le dossier d'un moteur (24 imports hors `mv_mz`). La logique de décision est écrite deux fois (MZ : 150 lignes, Wolf : 60 lignes), et la couche commande trois fois par morceaux (boucle de scan, chargement de police, drapeaux de langue). Ce qui est commun à la décision est mince (une échelle d'environ 40 lignes) ; le vrai gain est structurel : MV et un repli générique n'auront rien à recopier.
2. **Back, vrais doublons (§3).** Environ 420 lignes : deux fonctions de déchiffrement identiques (54 lignes ×2), la scission récursive du pipeline LLM (30 ×2), l'injection d'un fichier MV/MZ écrite deux fois dans l'export (environ 120 ×2), le parcours des commandes Wolf 101/102 (4 sites), un calcul de date (19 ×2), un extracteur Wolf qui ne sert qu'aux tests (45).
3. **Back, accès au projet (§3, B7).** 24 requêtes `… FROM projects WHERE id = ?` dans `commands/`, la liste des `source_files` recopiée 4 fois, et le moteur comparé comme chaîne brute à 33 endroits. Une dérive existe déjà : repli `"fr"` en Rust contre `'en'` en SQL.
4. **Front (§4).** Le lot P5 a traité D3, D6, D7, D8, D10 et D11. Restent D1 et D9 en partie, D5 hors grille, D2 et D4 volontairement laissés. Constats nouveaux : 25 `String(err)` au lieu de `errorMessage`, pas de couche d'API typée hors terminologie (43 `invoke` dans 20 fichiers), 14 tests `engine ===` dispersés, un patch par frame écrit deux fois.
5. **À ne pas réunir (§5).** `mv_mz/extractor.rs` contre `vx_ace/extractor.rs` (150 lignes identiques, mais VX Ace est une copie figée et désactivée), les deux `integrity.rs`, les libellés QA EN/FR, les stores de dialogue du front.

## 2. Cas connu : la mise en page du texte (`TextBox`)

Tout ce paragraphe est **vérifié** par lecture, sauf mention contraire.

### 2.1 État actuel

| Rôle | Fichier | Lignes (hors tests) |
|---|---|---|
| Police, mesure (`GameFont`, `Metrics`, `FontCache`) | `engines/mv_mz/layout/font.rs` | 291 |
| Jetons (`Token`, `TokenKind`, `token_width`, `apply_font_change`, `raw_of`) | `engines/mv_mz/layout/text.rs` | 304 |
| Découpe (`pieces`, `wrap`, `size_after`) | `engines/mv_mz/layout/wrap.rs` | 350 |
| Types de réglage et de résultat (`LineBreaks`, `Overflow`, `FitOutcome`) | `engines/mv_mz/layout/fit.rs:23-106` | 84 |
| Décision MZ (`fit_message`, `fit_description`) | `engines/mv_mz/layout/fit.rs:731-1003` | 273 |
| Simulation MZ en pixels (`BoxSpec`, `simulate`) | `engines/mv_mz/layout/simulate.rs` | 225 |
| Jetons Wolf (`tokenize`, `split_face_prefix`) | `engines/wolf/layout/text.rs` | 174 |
| Décision Wolf (`fit_message`, `shrink`, `fits`, `measure_budget`) | `engines/wolf/layout/fit.rs:94-331` | 238 |
| Réglages par projet | `core/text_fit.rs` | 489 avec tests |
| Commandes MZ, puis aiguillage Wolf | `commands/text_fit.rs` | 777 |
| Commandes Wolf | `commands/wolf_text_fit.rs` | 318 |

Imports de `engines::mv_mz::layout` hors du moteur MV/MZ (`rg 'mv_mz::layout' --glob '!engines/mv_mz/**'`) : 24 lignes, dont 1 commentaire et 3 dans des tests, dans `state.rs:8`, `core/text_fit.rs:10-11`, `core/report.rs:10`, `commands/qa.rs:8`, `commands/text_fit.rs:20-28`, `commands/wolf_text_fit.rs:12-13`, `engines/wolf/line_breaks.rs:12-14`, `engines/wolf/layout/text.rs:11`, `engines/wolf/layout/fit.rs:16-21`. La couche `core/` dépend donc d'un moteur, et Wolf dépend de MV/MZ.

### 2.2 Ce qui est réellement commun

| # | Élément | Preuve | Nature |
|---|---|---|---|
| C1 | Types `LineBreaks`, `Overflow`, `FitOutcome` | définis en `mv_mz/layout/fit.rs:23-106`, importés par `wolf/layout/fit.rs:16` et `core/text_fit.rs:10` | déjà partagés, mal rangés |
| C2 | Police et mesure : `Metrics`, `GameFont`, `FontCache`, `is_wide` | `mv_mz/layout/font.rs`, utilisés par Wolf (`fit.rs:17`, `line_breaks.rs:12`) et `state.rs:8` | neutre, mal rangé |
| C3 | Découpe : `pieces`, `wrap`, `size_after` | `mv_mz/layout/wrap.rs:46,76,222` ; Wolf `fit.rs:21`, `line_breaks.rs:14` | neutre, mal rangé |
| C4 | Forme des jetons : `Token`, `TokenKind`, `token_width`, `raw_of` | `mv_mz/layout/text.rs:19-60,290-303` ; Wolf réutilise le type (`wolf/layout/text.rs:11`) | partagé, **mais contient des règles MZ** (voir 2.3, P1) |
| C5 | Échelle de décision : ignorer → tient déjà → recouper à la taille de base → réduire de `taille−1` jusqu'au minimum → sinon « trop long » avec une raison | MZ `fit.rs:743-869`, Wolf `fit.rs:287-330` | même squelette, écrit deux fois |
| C6 | Boucle de réduction `for size in (min_font_size..current).rev()` + code de taille en tête + nouvel essai | MZ `fit.rs:835-844` (messages), `:977-996` (descriptions) ; Wolf `fit.rs:259-266` | 3 occurrences |
| C7 | Découpe en blocs : lignes vides gardées, étiquette de locuteur seule sur sa ligne, jointure avec ou sans espace selon la langue, `Reflow` contre `Keep` | MZ `fit.rs:218-292` ; Wolf `fit.rs:158-191` | même intention, règles différentes (voir 2.3, P3) |
| C8 | Boucle de scan : compteurs par issue, caractères absents de la police, lignes notables plafonnées, tri | `commands/text_fit.rs:493-572` (80 lignes) et `commands/wolf_text_fit.rs:199-244` (46 lignes) ; `MAX_ROWS = 500` défini deux fois (`text_fit.rs:447`, `wolf_text_fit.rs:20`) | vrai doublon, environ 35 lignes |
| C9 | Chargement de la police du jeu → `(Metrics, FontInfo)` avec repli `Estimated` | `commands/text_fit.rs:146-173` et `commands/wolf_text_fit.rs:76-113` | vrai doublon, environ 25 lignes ; seule la façon de trouver le fichier diffère |
| C10 | Drapeau « langue sans espaces » `["zh","ja","ko"]` | `commands/text_fit.rs:233-235`, `commands/wolf_text_fit.rs:163-165` | 2 copies |
| C11 | « Réglages reçus et validés, sinon réglages enregistrés » | `commands/text_fit.rs:458-466` et `:472-479` | 2 copies dans la même commande |
| C12 | Résumé d'export et compteurs (`FitCounts::add`) | `wolf_text_fit.rs:286-317`, `text_fit.rs:402-417`, `export.rs:957` | à réunir avec C8 |

### 2.3 Ce qui doit rester propre à chaque moteur

| # | Point | MV/MZ | Wolf |
|---|---|---|---|
| P1 | Codes de texte | `\C \I \PX \FS \{ \}` ; `\{` et `\}` valent ±12 px bornés (`text.rs:280-287`) ; icône de 36 px (`text.rs:15`) ; textes insérés (`Inserts`) | `\f[n]`, `\c[n]`, `\cself[n]`, préfixe de visage `@N⏎` détaché avant tout (`wolf/layout/text.rs:42`) |
| P2 | Verdict « ça tient » | simulation en pixels de la fenêtre (`simulate`, `overflows`, `is_visible`) : largeur, hauteur, pages (`fit.rs:171-176`) | budget en em mesuré sur les textes d'origine : nombre de lignes et ligne la plus large (`fit.rs:227-229`) |
| P3 | Unité traitée | un message = plusieurs commandes 401, certaines non modifiables (`MessageLine.writable`) ; le résultat est redistribué ligne par ligne (`distribute`, `fit.rs:636`) | un segment = un message entier ; le résultat est une seule chaîne |
| P4 | Code de réduction | `\FS[n]`, émis à chaque début de page (`render`, `as_is` `fit.rs:694-728`) | `\f[n]` en tête, qui remplace un code de tête existant (`fit.rs:244-261`) ; refus si un autre code de taille suit (`:251-258`) |
| P5 | Nouvelle page | oui, à une fin de phrase (`paginate`, `fit.rs:469`) ; interdite si `\^` ou `\|` (`:753-755`) | n'existe pas : `NewPage` est refusé à l'enregistrement (`core/text_fit.rs:124`) |
| P6 | Motifs d'abandon | positionnement manuel, texte de droite à gauche, sauts de page déjà présents (`fit.rs:743-751`) | codes de position ou d'alignement ; code de taille sans taille de base connue (`fit.rs:289-296`) |
| P7 | Géométrie | lue dans `System.json` et les plugins (`geometry.rs`), en pixels | mesurée sur les cartes (`measure_budget`, p99 des largeurs), en em, ou saisie à la main |
| P8 | Descriptions | boîte d'aide de 2 lignes, première ligne parfois sans code (`fit_description`, `fit.rs:879`) | aucune |
| P9 | Étiquette de locuteur | entre crochets (`RE_NAME_LABEL`, `fit.rs:132`) | terminée par `:` ou `：` (`is_label`, `fit.rs:122`) |

**Trois différences de comportement à ne pas lisser** (une abstraction naïve les effacerait) :

- **« Tient déjà ».** MZ teste le texte tel que traduit *avant* tout recoupage et le rend intact (`fit.rs:757-764`). Wolf nettoie et recoupe *toujours*, puis compare le résultat au texte d'entrée pour dire `Fits` ou `Rewrapped` (`fit.rs:300-309`).
- **`Overflow::Leave`.** MZ rend le texte tel que traduit (`fit.rs:773-775`). Wolf recoupe quand même ; `Leave` n'y interdit que la réduction (`fit.rs:311-330`).
- **Quand réduire.** MZ réduit dès que ça déborde, en largeur ou en hauteur. Wolf ne réduit que s'il y a trop de lignes, jamais pour un mot trop large (`fit.rs:311-312`).

**Ressemblance trompeuse.** Les deux `fit_message` portent le même nom et rendent le même `FitOutcome`, mais ils n'évoluent pas pour les mêmes raisons : MZ suit le code du moteur (`rmmz_windows.js`, plugins), Wolf suit des mesures en jeu (plan WR). La partie à mettre en commun est l'échelle et le rapport, pas le recoupage ni le verdict.

### 2.4 Fuites de moteur dans la base actuelle

À corriger par le lot, puisque la base commune ne doit connaître aucun moteur :

- `core/text_fit.rs:61` : `if engine == "wolf"` dans `default_for` ; `:355-356` : `match` sur `"mv_mz"` / `"wolf"` dans `enabled_kinds`.
- `core/text_fit.rs:27-35` : champs `wolf_width_em`, `wolf_max_lines`, `wolf_base_font_size` dans les réglages communs ; `:118-131` : `validate_wolf`.
- `commands/text_fit.rs:344`, `:457`, `:580` : `== "wolf"` en tête de trois commandes ; `:108` : `!= "mv_mz"`.
- `mv_mz/layout/text.rs:280-287` : règle MZ `±12` dans `apply_font_change`, que Wolf appelle aussi (`wolf/layout/fit.rs:67`). Sans effet aujourd'hui : `rg 'Bigger|Smaller' engines/wolf/` ne trouve rien, Wolf ne produit jamais ces jetons.
- Côté front, `TextFitDialog.tsx:45-46`, `:138-157`, `:196` lisent directement les champs `wolf*`.

### 2.5 Proposition

Module `core/text_fit/` (le fichier `core/text_fit.rs` devient `core/text_fit/settings.rs`) :

```
core/text_fit/
  mod.rs        types LineBreaks, Overflow, FitOutcome, FitCounts
  font.rs       Metrics, GameFont, FontCache      (déplacé tel quel)
  token.rs      Token, TokenKind neutre, raw_of   (déplacé ; règles MZ sorties)
  wrap.rs       pieces, wrap, size_after          (déplacé tel quel)
  fit.rs        trait TextBox + fn fit(...)       (l'échelle C5/C6, écrite une fois)
  measured.rs   MeasuredTextBox                   (budget en em : measure_budget généralisé)
  scan.rs       boucle de scan et résumé          (C8, C12)
  settings.rs   réglages par projet               (existant)
engines/mv_mz/layout/   MzTextBox  (blocks, simulate, paginate, \FS)
engines/wolf/layout/    WolfTextBox (préfixe de visage, blocs R3, \f)
```

Points d'adaptation du trait (esquisse, à arrêter dans le plan du lot) :

| Point | Rôle | MZ | Wolf | Défaut |
|---|---|---|---|---|
| `skip_reason(msg)` | P6 | positions, RTL, sauts de page | positions, taille inconnue | aucun |
| `as_translated(msg)` | « tient déjà » | simulation du texte intact | `None` (toujours recouper) | `None` |
| `rewrap(msg, taille)` | P3, P7, C7 | blocs + `lay_out_flow` | `blocks` + `layout` | — |
| `verdict(candidat)` | P2 | `simulate` | budget en em | — |
| `size_code(n)` | P4 | `\FS[n]` par page | `\f[n]` en tête | — |
| `shrinkable(verdict)` | quand réduire | toujours | trop de lignes seulement | toujours |
| `paginate(msg)` | P5 | `paginate` | absent | `None` |
| `leave_keeps_text()` | `Leave` | oui | non | oui |

Aucun de ces points ne prend le moteur en paramètre : chaque adaptateur est un type. L'aiguillage se fait une seule fois, là où le projet est chargé.

`MeasuredTextBox` : `measure_budget` (`wolf/layout/fit.rs:94-119`) ne dépend de Wolf que par son tokenizer. C'est le cas réel dont le repli générique doit sortir. `MvTextBox` : aujourd'hui MV est refusé (`commands/text_fit.rs:131-135`, absence de `rmmz_core.js`) ; il n'existe aucun cas réel MV dans le code, donc **ne pas l'écrire dans le lot 1** (règle : la base se construit sur les cas réels).

### 2.6 Gain, risque, tests

- **Gain :** environ 110 lignes en moins (C5, C6, C8 à C12), 24 imports croisés supprimés, plus aucun `engine ==` dans `core/text_fit`. Surtout : un moteur de plus = un adaptateur, sans recopie.
- **Risque : moyen à élevé.** Le résultat est écrit dans les fichiers du jeu à l'export. Les trois différences de 2.3 sont faciles à effacer sans le voir.
- **Tests existants :** `mv_mz/layout/fit.rs` (11), `wrap.rs` (8), `font.rs` (5), `simulate.rs` (5), `text.rs` (4), `verify.rs` (4), `plan.rs` (2), `message.rs` (3) ; `wolf/layout/fit.rs` (12), `wolf/layout/text.rs` (4), `wolf/layout/mod.rs` (3, dont un `#[ignore]` sur jeu réel) ; `wolf/line_breaks.rs` (11) ; `core/text_fit.rs` (4) ; intégration `src-tauri/tests/text_fit.rs` (11, MZ seulement).
- **Trous :** aucun test d'intégration Wolf sur le scan ni sur l'export avec ajustement (`WolfExportFit`) ; aucun corpus figé des sorties. **À faire avant de toucher au code :** un test de caractérisation qui fige, pour un jeu d'exemples MZ et Wolf, le texte rendu et le `FitOutcome` (même méthode que l'étape 0 de la Phase 8).

## 3. Back : autres constats

Classés par gain. Risque : F = faible, M = moyen, É = élevé.

| # | Constat | Emplacements | Lignes | Commun / différent | Proposition | Gain | Risque | Tests |
|---|---|---|---|---|---|---|---|---|
| B1 | Injection d'un fichier MV/MZ écrite deux fois dans l'export | `commands/export.rs:505-640` (dans `write_export`) et `:1346-1556` (`debug_inject_file`) | ~120 ×2 | Même suite d'appels, une fois chacun des deux côtés : `fitted_lines`, `recorded_sources`, `split_fitted`, `should_prefix`, `with_font_prefix`, `verify_sources`, `note_rewrites`, `inject_to_bytes`, `verify_json_edit`, `build_plugins_js`. Diffère : sortie (zip ou disque) ; le chemin « un fichier » n'a pas `locale` | `prepare_file_export(...) -> octets`, appelé par les deux. **Avant :** décider si `debug_inject_file` sert encore (voir §6) | ~110 | M | `tests/export_game_update.rs`, `tests/text_coverage.rs`, `tests/e2e_project_flow.rs`, `tests/text_fit.rs` pour `export_project` ; **aucun** pour `debug_inject_file` |
| B2 | Deux fonctions de déchiffrement Wolf identiques | `engines/wolf/decrypt/legacy_xor.rs:1380-1433` et `:1440-1493` | 54 ×2 | `diff` : seuls le nom et un paramètre (`unpacked` / `press_sz`) changent, plus un commentaire | Une fonction, deux appels | 54 | F | `legacy_xor.rs:3003-3025` |
| B3 | Scission récursive du pipeline LLM | `llm/split.rs:121-150` et `:194-223` | 30 ×2 | `diff` : identiques à l'indentation près ; seuls le compteur et le log voisins diffèrent | `async fn split_and_merge(...)` | 30 | F à M (appel récursif `async`) | `llm/pipeline.rs` (`test_response_format_triggers_split`, `test_split_partial_success`) |
| B4 | Parcours des commandes Wolf 101/102 et format des clés | `engines/wolf/extractor.rs:287-331`, `:451-494` ; `engines/wolf/injector.rs:180-220`, `:477-515` ; forme déjà factorisée en `engines/wolf/integrity.rs:33-50` | ~40 ×4 | Commun : `match` sur `CID_MESSAGE` / `CID_CHOICES`, clé `{préfixe}/{cmd}[/choices/{i}]`. Diffère : préfixe (carte, événement commun), action (lire, remplacer) | Un itérateur de « sites de texte » par commande, pris par l'extracteur et l'injecteur. **Laisser `integrity.rs` à part** : son en-tête dit qu'il ne réutilise pas l'injecteur exprès | ~80 | M | modules de test `extractor.rs` (31), `injector.rs` (22), `integrity.rs` (7) ; aller-retour Wolf de `e2e_project_flow.rs` |
| B5 | `extract_wolf_project` refait `extract_all_wolf` | `engines/wolf/extractor.rs:512-560` contre `:175-242` | ~45 | Même séquence, sortie à plat. `rg` : la production n'appelle que `extract_all_wolf` (`commands/project.rs:943`) ; `extract_wolf_project` n'est appelé que par les tests du fichier (`:785`, `:809`, `:1070`) | L'écrire comme un aplatissement de `extract_all_wolf`, ou le déplacer dans les tests | 45 | F | les 3 tests cités |
| B6 | Calcul de date civile | `core/report.rs:436-460` et `utils/time.rs:4-26` | 19 ×2 | `diff` : seul le `format!` final change | `utils::time::civil_now()` + deux formats | 19 | F | aucun test direct trouvé |
| B7 | Accès au projet et aux fichiers, recopié | `… FROM projects WHERE id = ?` : 24 dans `commands/`, 5 dans `core/` (compte du sous-agent ; 14 relus : `export.rs:444,750,1365,1570`, `coverage.rs:105,185,445`, `text_fit.rs:102,320`, `wolf_text_fit.rs:122`, `project.rs:543`, `core/text_fit.rs:348`, `core/originals.rs:99`, `core/terminology/service.rs:130`). Liste complète des `source_files` : `export.rs:454`, `:772`, `translate.rs:226`, `text_fit.rs:116` ; variantes `resync.rs:179`, `core/originals.rs:110`, `coverage.rs:196` | ~120 | Même requête, colonnes et replis variables. **Dérive :** repli `"fr"` en `translate.rs:143` et `:290` contre `COALESCE(target_language, 'en')` ailleurs | `db::projects::{context(id), source_files(id)}` rendant un type (`ProjectContext { game_path, engine, source_lang, target_lang }`). La dérive `fr`/`en` est un **défaut**, à traiter à part : la corriger change un comportement | ~80 | M | `e2e_project_flow.rs`, `export_*`, `text_fit.rs`, `text_coverage.rs` |
| B8 | Moteur comparé comme chaîne brute | 33 comparaisons hors tests (`rg 'engine (==\|!=) "\|"(wolf\|mv_mz\|vx_ace)" =>'`), dont 16 dans `commands/export.rs` (`:73,81,92,204,220,466,512,519,537,558,755,762,1386,1414,1431,1481`, liste du sous-agent). Deux enums `Engine` : `engines/detector.rs:20` et `llm/tokenizer.rs:92` | — | `Engine::db_str` existe (`detector.rs:28`), pas son inverse. `Engine::from_db_str` avait été retiré en Phase 8 faute de consommateur (`tasks/todo.md:2418`) : le consommateur existe maintenant | `Engine::from_db_str`, lu une fois avec B7 ; puis des méthodes de capacité (`supports_coverage()`, `font_prefix_code(n)`…) à la place des `if engine == "wolf"` | lisibilité, pas de lignes | M | idem B7 |
| B9 | Chaîne Wolf préfixée par sa longueur | lecture : `engines/wolf/dat_parser.rs:190-218`, `engines/wolf/v3_format/coder.rs:60-92` ; écriture : `injector.rs:231-236`, `coder.rs:121-135` ; `wolf/encoding.rs` se dit central | ~30 | `dat_parser` a un repli Shift-JIS de réparation, propre au `.dat` | Faire passer `coder.rs` par `encoding.rs` ; garder le repli de `dat_parser` | ~25 | M (octets du jeu) | modules `coder.rs` (7), `dat_parser.rs` (11), `encoding.rs` — **non relu par moi : supposé** |
| B10 | `SegmentSearchHit` = `Segment` + `file_name` | `domain/types.rs:55-74` et `:91-109` ; colonnes recopiées en `commands/project.rs:313`, `:440`, `commands/search.rs:106` | 17 + 3 listes | 16 champs identiques | `#[sqlx(flatten)]` / `#[serde(flatten)]` sur un `Segment`, ou constante `SEGMENT_COLUMNS` | ~30 | M (forme JSON vue par le front) | module `commands/search.rs` (8) |
| B11 | Reste de la Phase 8 | `commands/coverage.rs:477-487` refait l'INSERT de `insert_source_file` (`commands/project.rs:996`) | ~10 | identique (supposé : non relu ligne à ligne) | appeler le helper | 10 | F | `tests/text_coverage.rs` |
| B12 | Petits doublons à 2 occurrences | `core/tm.rs:120-146` / `:150-172` ; `core/terminology/repository.rs:389-398` / `:449-458`, `:573-594` / `:596-616` ; `llm/provider.rs:573-585` / `:689-701` ; `commands/translate.rs:125-136` / `:259-275` | ~10 chacun | — | **Laisser** : 2 occurrences, gain faible | — | — | — |

**Conversions d'erreur.** 357 `map_err`, dont 206 `.to_string()` ; `export.rs` 70, `project.rs` 30, `coverage.rs` 24, `text_fit.rs` 23 (les 4 premiers vérifiés par `rg -c`). C'est répétitif, mais c'est la convention du projet (`Result<T, String>` à la frontière IPC, `CLAUDE.md`). **Non proposé** : un type d'erreur commun changerait cette convention, ce qui est une décision d'architecture (ADR), pas une mise en commun.

**Déjà fait et confirmé en place** (Phases 6 et 8) : `manifest::refresh_stats`, `relevant_terms`, `map_id_from_name`, `QaError::penalty/label`, `Engine::db_str/data_dir/game_title`, `classify_file_type`, `extract_project`, `insert_source_file/insert_segment`. Restes : B8 et B11.

## 4. Front : état après P5 et constats nouveaux

### 4.1 État des constats D1 à D11

| # | État | Preuve |
|---|---|---|
| D1 | partiel | `errorMessage` existe (`lib/utils.ts:9`), 3 appelants seulement (`stores/llm.ts:298`, `:346`, `stores/pilot.ts:110`) |
| D2 | non fait, voulu | stores `resync`, `coverage`, `textFit` inchangés (essai sans gain, journal P5) |
| D3 | fait | `lib/events.ts`, `hooks/useTauriEvent.ts` |
| D4 | non fait, voulu | en-têtes de panneau encore à la main (`TMPanel.tsx:86`, `QAPanel.tsx:264`, `TerminologyInspector.tsx:46`, `TextFitPreviewPanel.tsx:211`) — supposé, relevé par le sous-agent |
| D5 | partiel | `SegmentColumnMeta` (`features/editor/columns.tsx:193`) pour la grille seule |
| D6, D7, D8, D10, D11 | faits | `H2S_FILTER`, `renderWithQuery`, `ensureModelConfigured`, `StatusBadge` |
| D9 | partiel | `TerminologyScope` (`lib/types.ts:335`) ; `TerminologyScope \| "both"` encore écrit 3 fois (`terminology/api.ts:85`, `TermTranslateDialog.tsx:49`, `:162`) — supposé |

### 4.2 Constats nouveaux

| # | Constat | Emplacements | Commun / différent | Proposition | Gain | Risque | Tests |
|---|---|---|---|---|---|---|---|
| F1 | `String(err)` au lieu de `errorMessage` | 25 sites hors `lib/` (vérifié par `rg`) : `stores/resync.ts`, `coverage.ts`, `textFit.ts`, `project.ts`, `hooks/useAppHandlers.ts`, `useExportToFile.ts`, `AppToolbar.tsx`, `PackImportWizard.tsx`, `TextCoverageDialog.tsx`, `TextFitDialog.tsx`, `SegmentGrid.tsx`, `TermTranslateDialog.tsx`, `TerminologyWorkspace.tsx` | `catch` puis toast ou état. **Défaut visible :** `TerminologyWorkspace.tsx:203` lance un `Error`, `:230` affiche `String(error)` → le toast commence par « Error: » | `errorMessage` partout. Ne pas toucher les sites qui analysent le texte (`useAppHandlers.ts:149`, `lib/projectError.ts`) | cohérence + 1 défaut | F (change le texte pour les vrais `Error`, donc à annoncer) | `useAppHandlers.test.ts`, `TerminologyWorkspace.test.tsx`, `ResyncDialog.test.tsx`, `TextFitDialog.test.tsx`, `TextCoverageDialog.test.tsx` |
| F2 | Pas de couche d'API typée hors terminologie | `features/terminology/api.ts` (12 commandes) est le seul module ; ailleurs 43 `invoke` dans 20 fichiers (`useAppHandlers.ts` 9, `stores/project.ts` 6, `coverage.ts` 4, `textFit.ts` 3 : vérifié). `get_project_stats` ×4 : `useAppHandlers.ts:67`, `:214`, `stores/project.ts:111`, `ProjectList.tsx:50` (vérifié) | nom de commande et arguments en chaîne à chaque appel | `lib/api/` par domaine, sur le modèle de `terminologyApi` | typage ; peu de lignes | F à M (les tests simulent `invoke` par nom : ils restent valables) | tous les tests de stores et de dialogues |
| F3 | Tests `engine ===` dispersés | 14 sites (vérifié) : `AppToolbar.tsx:306,321-322,336,355-356`, `useAppHandlers.ts:119`, `stores/project.ts:126`, `stores/pilot.ts:74`, `PilotComparisonWorkspace.tsx:85`, `TextFitPreviewPanel.tsx:177,217`, `FontSizeDialog.tsx:29,86`, `lib/constants.ts:93` | `Project.engine` est une `string` nue ; 3 capacités distinctes (couverture, ajustement, pilote) | type `Engine` + un prédicat **par capacité** dans `lib/`. Ne pas les fusionner : chaque capacité évolue seule | lisibilité ; prépare MV et Bakin | F | `AppToolbar.test.tsx`, `FontSizeDialog.test.tsx`, `pilot.test.ts` |
| F4 | Patch `segments-updated` regroupé par frame, écrit deux fois | `SegmentGrid.tsx:224-236` et `GlobalSearchResults.tsx:77-97` (vérifié) | même mécanique ; diffèrent la cible et la place du test `pending.size === 0` | hook `useCoalescedSegmentUpdates(apply)` ; déplacer `applySegmentUpdates` dans `lib/` (il est exporté d'un composant). 2 occurrences, mais même raison de changer | ~15 | M (chemin chaud de la grille) | `SegmentGrid.test.tsx`, `GlobalSearchResults.test.tsx` |
| F5 | `previewResync` / `applyResync` | `stores/resync.ts:28-43` et `:46-57` (vérifié) | identiques au booléen `apply` et à l'état initial près | `runResync(projectId, apply)` interne | ~10 | F | `ResyncDialog.test.tsx` |
| F6 | Petits doublons locaux | `TextCoverageDialog.tsx:563-578` / `:617-632` (section repliable) ; `LanguageSettings.tsx:42-63` / `:64-85` (sélecteur de langue) ; `ProjectList.tsx:198-207` / `:280-289` (bouton « ouvrir un jeu ») ; `AppDialogs.tsx:91-96` / `:132-137` ; `AppToolbar.tsx:72-73` / `:98-99` (`mm:ss`) | clones exacts relevés par le script ; contenu décrit par le sous-agent | composants ou fonctions locaux | ~50 | F | `SettingsModal.test.tsx`, `ProjectList.test.tsx`, `AppToolbar.test.tsx`, `format.test.ts` |
| F7 | Bouton « icône ou spinner » de la barre | `AppToolbar.tsx:245,260,279,297,344,392,413` (supposé, non relu) | 7 ternaires `Loader2` / icône | `ToolbarButton({ icon, busy })` local | ~25 | F | `AppToolbar.test.tsx` |
| F8 | Code mort | `lib/types.ts:443` `PipelineBatchMetrics` (jamais importé) ; `lib/types.ts:623` `CoverageCoreWarning` réécrit en ligne en `TextCoverageDialog.tsx:612-616` (supposé) | — | supprimer / importer | faible | F | typecheck |

## 5. Ressemblances trompeuses : à ne pas réunir

| Paire | Pourquoi |
|---|---|
| `engines/mv_mz/extractor.rs` ↔ `engines/vx_ace/extractor.rs` (150 lignes identiques : `261-335`↔`105-179`, `401-454`↔`362-411`, `344-375`↔`183-214`, `477-499`↔`457-479`) | VX Ace est une copie figée de l'ancien extracteur JSON, et il est désactivé (détection commentée, `engines/detector.rs:136-145`, vérifié). Le parcours d'événements a déjà divergé. Les réunir lierait un moteur vivant à un moteur éteint. À revoir le jour où VX Ace est réactivé |
| Les deux `fit_message` (MZ, Wolf) | Voir §2.3 : seuls l'échelle et le rapport sont communs |
| `engines/mv_mz/integrity.rs` ↔ `engines/wolf/integrity.rs` | API différentes ; seul `IntegrityIssue` est partagé. `wolf/integrity.rs` est indépendant de l'injecteur exprès (c'est un contrôle) |
| `core/qa.rs:838-879` ↔ `:923-963` | Libellés EN et FR : même `match`, textes différents ; le `match` exhaustif sert de garde-fou |
| `engines/wolf/v3_format/{command,map,common_events}.rs` (bloc de 7 lignes ×4) | Simple signature de `dump(...)` |
| `write_zip` (`export.rs:1256`) ↔ `write_pack` (`core/h2s_pack.rs:149`) | Formats et erreurs différents |
| Front : stores `resync` / `coverage` / `textFit` | Forme commune, cycles de vie différents ; l'essai de P5 n'a rien gagné |
| Front : `AppDialogs.tsx:45-64` ↔ `useAppHandlers.ts:243-264` | Contrat d'un hook (retour d'un côté, déstructuration de l'autre), pas une logique en double |
| Front : `terminology/api.ts:67-74` ↔ `lib/types.ts:261-268` | Entrée d'écriture contre réponse : structs Rust distinctes |
| Front : écoutes de run (`stores/llm.ts`, `stores/pilot.ts`, `TermTranslateDialog.tsx`) | Trois durées de vie différentes |

## 6. Classement et lots proposés

### 6.1 Classement par gain et par risque

| Gain \ Risque | Faible | Moyen | Élevé |
|---|---|---|---|
| **Fort** | B2, B3 | B1, B4, B7 + B8 | §2 `TextBox` |
| **Moyen** | B5, B6, F1, F6 | F2, F3, F4, B9, B10 | — |
| **Faible** | B11, F5, F7, F8 | — | — |

### 6.2 Lots, un par PR

Chaque lot : plan dans `tasks/todo.md`, aucun changement de comportement, aucun test existant modifié (ajout de tests de caractérisation autorisé), gate complet en session cloud.

| Lot | Contenu | Gain | Risque | Preuve attendue |
|---|---|---|---|---|
| **L1 — `TextBox`** (§2) | Étape 0 : corpus de caractérisation MZ et Wolf. Puis A : déplacer types et primitives vers `core/text_fit/` (déplacements purs). B : trait `TextBox`, échelle commune, `MzTextBox`, `WolfTextBox`, `MeasuredTextBox`. C : couche commande (scan, police, drapeaux de langue, aiguillage unique). Sans `MvTextBox` | fort | élevé | corpus identique avant et après ; `rg 'mv_mz::layout'` hors `mv_mz` = 0 ; `rg 'engine == \|"wolf"' core/text_fit` = 0 |
| **L2 — Clones exacts du back** | B2, B3, B5, B6, B11 | ~160 lignes | faible | diff net négatif ; tests cités verts |
| **L3 — Parcours des commandes Wolf** | B4 (extracteur et injecteur ; `integrity.rs` intact) | ~80 lignes | moyen | aller-retour Wolf de `e2e_project_flow.rs` ; sorties de l'extracteur identiques |
| **L4 — Suppression de `debug_inject_file`** | B1 : la commande est supprimée (décision du 2026-11-17), plus de factorisation à faire | ~210 lignes | faible | tests d'export verts ; `rg debug_inject_file` = 0 ; `clippy` sans code mort |
| **L5 — Contexte de projet et moteur typé** | B7, B8 (`ProjectContext`, `Engine::from_db_str`, capacités). La dérive `fr` / `en` est signalée, pas corrigée dans ce lot | ~80 lignes, 33 comparaisons de chaîne en moins | moyen | gate ; `rg` des comparaisons de chaîne en baisse |
| **L6 — Front, petits doublons** | F1, F5, F6, F7, F8, restes de D9 | ~100 lignes, 1 défaut d'affichage | faible | Vitest ; test du toast sans « Error: » |
| **L7 — Front, API typée et moteur typé** | F2, F3, F4 | typage | faible à moyen | Vitest ; aucun `invoke(` hors `lib/api/` et `features/*/api.ts` |
| **Plus tard, sur décision** | B9, B10 ; type d'erreur commun (ADR) ; VX Ace à sa réactivation | — | — | — |

L1 passe en premier, comme convenu. L2 et L6 sont indépendants de tout et peuvent se faire en parallèle. L5 gagne à passer après L1 (L1 retire déjà 6 comparaisons de moteur) et avant L4 (qui profite de `ProjectContext`).

### 6.3 Décisions à prendre avant les lots

1. **`debug_inject_file` (`commands/export.rs:1346`).** Enregistrée dans `lib.rs:100`, mais `rg` ne trouve aucun appel dans `src/` ni dans `src-tauri/tests/` (vérifié). La garder (alors L4 la factorise) ou la supprimer (alors L4 se réduit à une suppression) ? **Décidé le 2026-11-17 (utilisateur) : la supprimer.** L4 devient donc : retirer la commande, son enregistrement dans `lib.rs:16` et `:100`, et les helpers qui ne servent plus qu'à elle.
2. **Repli de langue cible `"fr"`** en `commands/translate.rs:143` et `:290`, contre `'en'` partout ailleurs. Défaut ou choix ? Hors des lots de mise en commun dans les deux cas.
3. **`TextFitSettings` et les champs `wolf_*`.** Les colonnes restent en base. Le lot L1 peut soit les garder tels quels derrière l'adaptateur Wolf (aucun changement d'IPC), soit les regrouper en un sous-objet (change la forme JSON vue par `TextFitDialog.tsx`). La première voie respecte « sans changement de comportement ».
4. **`jscpd`.** Second passage souhaité ou non (téléchargement par `pnpm dlx`).

## 7. Vérifié / supposé

- **Vérifié par moi** (lecture, `diff`, `rg`) : tout le §2 ; B1 (appels identiques des deux côtés), B2, B3, B5, B6 (par `diff`), B4 (5 sites par `rg`), B7 (14 requêtes relues, dérive `fr`/`en`), B8 (compte de 33), F1 (25 sites, défaut `:203`/`:230`), F2 (comptes d'`invoke`, `get_project_stats` ×4), F3, F4, F5 ; absence d'appel à `debug_inject_file`.
- **Supposé** (relevé par un sous-agent, non relu ligne à ligne) : B9, B10, B11, B12 ; le total de 24 + 5 requêtes `projects` ; la liste exacte des 16 lignes d'`export.rs` en B8 ; D4, D9 ; F7, F8 ; le nombre de tests par module quand il vient du sous-agent (`extractor.rs` 31, `injector.rs` 22, `integrity.rs` 7, `coder.rs` 7, `dat_parser.rs` 11).
- **Non fait :** aucun test lancé, aucune compilation ; `jscpd` non lancé ; `commands/pilot.rs`, `pack_terminology.rs`, `core/terminology/scanner.rs` et `resolver.rs`, `stores/search.ts`, `updater.ts`, `settings.ts` non lus en entier.
