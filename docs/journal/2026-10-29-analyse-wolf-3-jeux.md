# Journal — 2026-10-29 — Textes Wolf extraits : trois jeux comparés, fusion de la PR #23

**Phase** : F5 (Wolf)
**Durée estimée** : 3h
**Statut** : ✅ Complété (analyse et fusion) ; UW-9 non fait
**Outil** : Claude (session locale)
**Branche et commit de départ** : `main` @ `e95bb38` pour l'analyse ; `docs/wolf-analyse-3-jeux` depuis `origin/main` @ `92570fb` pour ce suivi
**Objectif / plan / story** : demande de l'utilisateur — refaire l'analyse des textes extraits par l'app, jeu par jeu, puis conclure la PR #23 (plan UW, `tasks/todo.md`)

---

## Ce qui a été fait

- **Vérification du rapport de la session cloud UW.**
  - Relu `is_resource_path` et `is_lone_path` (`src-tauri/src/engines/wolf/extractor.rs`) et leurs tests.
  - Cherché les noms de fichiers non ASCII sans dossier dans les bases de とのかじ et ポンコツ : il n'y en a aucun. Dans le 3.658, les 19 chemins `bg/…` contiennent tous un `/`.
- **Comparaison entre les fichiers et la base de l'app** pour les trois projets. Les cartes et `CommonEvent.dat` ont été relus avec `RAE-PLAN/wolf/_outils/wolf_v3.py` ; l'archive du 3.658 a été dépaquetée avec `dxa8.py` dans un dossier temporaire.
- **Inventaire du texte japonais hors messages et choix**, par commande Wolf. L'app n'extrait que 101 (message) et 102 (choix).
- **Classement des segments des deux jeux Pro**, par règle, sur le champ de base ou sur le nom d'événement.
- **Fusion de la PR #23** à la demande de l'utilisateur : commit de fusion `92570fb`, branches gardées.

## Fichiers créés

- `docs/journal/2026-10-29-analyse-wolf-3-jeux.md` (ce journal).
- Hors Git, dans `RAE-PLAN/essais/` :
  - `wolf-comparaison-3-jeux.md` ;
  - `wolf-tonokaji-segments-classes.tsv` ;
  - `wolf-ponkotsu-segments-classes.tsv`.

## Fichiers modifiés

- `tasks/handoff.md` — section de l'analyse ; section UW passée en « terminé », avec la PR #23 fusionnée.
- `tasks/lessons.md` — vérifier quelle branche a produit le binaire avant d'analyser un essai.

## Décisions prises

- Fusion de la PR #23 avant UW-9 : c'est la décision de l'utilisateur. UW-9 reste à faire sur `main`.
- Aucune décision produit sur les constats ci-dessous. L'utilisateur veut d'abord en discuter.

## Constats (faits vérifiés, sauf mention)

**Version testée**
- Les projets analysés viennent du binaire de `main` (`src-tauri/target/debug/hoshi2star`, 2026-10-28 05:53 UTC), lancé par `pnpm tauri:linux` dans le dossier principal. Ce n'est pas la PR.
- Les jeux Pro étaient quand même détectés grâce au `Game.ini` créé au lancement du jeu.

**Messages et choix complets**

| Jeu | Messages et choix | Extraits | Absents |
|---|---|---|---|
| とのかじ | 5 259 | 5 234 | 25 |
| ポンコツ | 7 549 | 7 502 | 47 |
| 3.658 | 22 003 | 21 698 | 305 |

- Tous les absents sont sans japonais, sauf 8 messages d'erreur des événements système `X[…]`, que l'extracteur exclut volontairement.
- Aucun segment en trop.
- Le 3.658 donne les mêmes 24 956 clés qu'au 2026-10-28.

**Texte affiché laissé hors extraction**

| Commande Wolf | とのかじ | ポンコツ | 3.658 |
|---|---|---|---|
| 122 · opération sur chaîne | 448 | 261 | 270 |
| 150 · image de texte | 139 | 53 | 95 |
| 210 · appel avec arguments texte | 0 | 37 | 4 355 |
| 300 · appel par nom, arguments | 2 074 | 138 | 7 |

- Dans le 3.658, environ 3 900 des chaînes de la commande 210 sont des noms de bulles internes et environ 450 des libellés de choix affichés.
- Dans toutes ces commandes, du texte affiché se mêle à des clés.

**Evtext de ポンコツ**
- Le dossier `Data/Evtext/` contient 268 fichiers, sous-dossiers `b/` et `d/` compris.
- On y compte 17 352 lignes de texte, en Shift-JIS, alors que les bases du jeu sont en UTF-8.
- Ces fichiers sont hors périmètre (UW-D8).

**Segments risqués dans les jeux Pro**
- Dans ces deux jeux, les bases servent de langage de script.

| Catégorie | とのかじ | ポンコツ |
|---|---|---|
| NE_PAS_TRADUIRE | 838 | 507 |
| REFERENCE | 1 986 | 132 |
| SCRIPT_MIXTE | 477 | 686 |

- NE_PAS_TRADUIRE : noms d'événements communs, chaînes de réglage d'image `立ち絵1//FaceGraphic/…png⏎立ち絵2//`, étiquettes `/胸+1`, champs `ｼｽﾃﾑ名`.
- REFERENCE : noms qui servent de clés (recettes, compétences requises).
- SCRIPT_MIXTE : dialogue et commandes `>bgs[…]|`, `@2103`, `<kakunin>`, `/cutin,0,1` dans le même segment.
- **Hypothèse non vérifiée en jeu** : le jeu recherche des données par leur nom. Indice : l'événement `[文字列]ﾃﾞｰﾀ文字列→ID検索`.

**Codes non protégés par `engines/wolf/placeholders.rs`**
- `\img[…]` ;
- `\v3[n]` (76 occurrences dans とのかじ) ;
- arguments imbriqués : `\udb[9:\cself[23]:20]`, `\sp[\cdb[6:1:22]]` ;
- `\my[-23]` ;
- `\cE` / `\wE` / `\cS` ;
- `\a` (54 occurrences dans ポンコツ).

## Problèmes rencontrés

- Une première comparaison pour le 3.658 donnait 633 messages absents. Cause : l'app stocke les noms de fichiers d'archive en minuscules, et la comparaison tenait compte de la casse. Elle a été refaite sans casse.
- Le premier comptage d'Evtext ne lisait que le dossier racine. Il a été refait en parcourant les sous-dossiers.

## Corrections et erreurs à ne plus répéter

- Les extractions censées tester la PR avaient été faites avec le binaire de `main`. La leçon du 2026-10-29 est ajoutée à `tasks/lessons.md`.

## Vérifications et limites

- Ce suivi ne touche que la documentation : aucun build ni test lancé (règle du PC qui gèle).
- Le classement des jeux Pro se fait par champ, pas segment par segment.
- Les commandes 122, 150, 210 et 300 sont échantillonnées, pas classées.
- Non tranché : les noms de lignes de base présents dans les opérations sur base (commande 250) sont-ils de simples étiquettes de l'éditeur, ou des clés que le moteur recherche pendant le jeu ? Dans le second cas, les champs de nom principaux seraient eux aussi des références.
- UW-9, l'essai en jeu d'un export de l'app, n'est pas fait.

## Tâches ROADMAP cochées

- [ ] Aucune.

## Prochaine session

- Discuter de la suite avec l'utilisateur. Deux pistes :
  - UW-9 sur `main`, qui contient maintenant UW. Il faut au préalable déchiffrer le 3.658 avec UberWolf ;
  - un plan pour élargir l'extraction (commandes 122, 150, 210, 300, Evtext) et protéger les champs de script et de référence.

---
*Journal partagé Codex / Claude — Hoshi2Star*
