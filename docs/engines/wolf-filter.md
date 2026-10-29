# Wolf RPG : textes non traduisibles skippés à l'extraction

Plan WF (2026-10-29, `tasks/todo.md`). À l'ouverture d'un jeu Wolf RPG, l'extracteur
saute les textes qui ne sont pas destinés au joueur. Un texte sauté ne crée **aucun
segment** (décision WF-D1) :

- il n'apparaît pas dans l'éditeur et ne part pas au LLM ;
- l'export ne touche que les segments existants, donc le jeu garde sa valeur d'origine
  (la garde d'intégrité Wolf refuse toute autre modification).

Les règles ne s'appliquent qu'à l'extraction (WF-D6) : un projet ouvert avant elles
garde ses segments. Pour en profiter, recréer le projet.

## Où sont les règles

Toutes les règles propres à Wolf sont dans `src-tauri/src/engines/wolf/filter.rs`.
`extractor.rs` ne fait que les appeler. Les règles communes à tous les moteurs
(texte vide, nombre, ponctuation seule, codes seuls) restent dans
`src-tauri/src/engines/filter.rs`.

| Fonction                             | Appelée pour                                                                | Saute                                                                         |
| ------------------------------------ | --------------------------------------------------------------------------- | ----------------------------------------------------------------------------- |
| `skip_map(nom)`                      | chaque carte (`.mps`, nom du fichier sans extension)                        | la carte entière si c'est une carte de débogage                               |
| `skip_common_event(nom)`             | chaque événement commun                                                     | l'événement entier : `X[…` (interne), `zz…` (archivé), débogage               |
| `skip_message(texte)`                | chaque message (101) et chaque choix (102) des cartes et événements communs | rien à traduire (`is_translatable`), message d'erreur                         |
| `skipped_fields(type, noms)`         | chaque type de base de données                                              | des champs entiers : noms d'événements communs, doublons « système »          |
| `skip_database_value(champ, valeur)` | chaque valeur texte de base                                                 | rien à traduire, `自動ｼｽﾃﾑ初期化`, valeur sans japonais dans un champ inconnu |

## Les règles

Seules les règles qui ne sautaient **aucun texte affiché** sur les trois jeux mesurés
(3.658, とのかじ, ポンコツ) sont appliquées (WF-D2). Motifs exacts (WF-D5) :

- **Débogage** (nom de carte ou d'événement commun, sans tenir compte de la casse) :
  contient `sample`, `debug`, `test`, `ﾃﾞﾊﾞ`, `デバ`, `ﾃｽﾄ`, `テスト` ou `使用禁止`, ou
  commence par `[D]` ou `※`.
- **Message d'erreur** : `ｴﾗｰ` n'importe où, ou `エラー` / `警告` suivi d'espaces
  éventuels puis `!`, `！`, `:` ou `：`. Motif strict : `警告、彼女の魔力が…` est une
  réplique de l'histoire de とのかじ et reste traduisible.
- **Champ de noms d'événements communs** : parmi les valeurs non vides du champ (tout le
  type), au moins 50 % sont, une fois découpées sur `,`, `、`, `|` et le retour à la
  ligne, des noms d'événements communs ou des nombres ; et au moins une valeur nomme un
  événement commun. `CommonEvent.dat` est donc lu avant les bases.
- **Doublon « système »** : le nom du champ contient `ｼｽﾃﾑ`, `システム` ou `Sys` ; sur ses
  lignes non vides (au moins 3), il a la même valeur qu'un même autre champ de la ligne,
  dont le nom ne contient pas ces mots, dans au moins 80 % des cas. Seul le champ système
  est sauté.
- **Champ homonyme** : la clé d'un segment de base désigne son champ par son nom, et
  l'export prend le premier champ texte de ce nom. Un champ qui porte le nom d'un champ
  sauté placé avant lui est donc sauté aussi : sinon sa traduction serait écrite dans le
  champ sauté.
- **Règles reprises sans changement** : chemins de ressources (`bg/スラム.png⏎0⏎0⏎0`),
  événements `X[` et `zz`, `自動ｼｽﾃﾑ初期化`, champs connus (`name`, `名前`, `説明`…) ou
  valeur avec du japonais.

Les règles décident par carte, par événement et par champ : sur le 3.658, 344 groupes
sur 352 sont soit entièrement à traduire, soit entièrement à exclure.

## Ajouter une règle

1. Mesurer la règle sur de vrais jeux avant de l'écrire : elle ne doit sauter aucun texte
   affiché. Noter la mesure dans le plan ou le journal.
2. L'écrire dans `engines/wolf/filter.rs`, avec ses tests unitaires (cas sautés et cas
   proches qui doivent rester).
3. L'appeler depuis la fonction de `filter.rs` qui correspond à l'étape d'extraction
   (`skip_map`, `skip_common_event`, `skip_message`, `skipped_fields`,
   `skip_database_value`). N'ajouter un appel dans `extractor.rs` que pour une nouvelle
   étape.
4. Compléter le test de jeu complet `test_extract_all_wolf_skips_non_translatable_texts`
   (`extractor.rs`) si la règle porte sur un nouveau type de texte.

Règles mesurées et **non retenues** (fiabilité insuffisante, voir `BACKLOG.md`) : mot-clé
dans le nom du champ, table de ressources d'après le nom du type, noms repris d'un autre
champ, faible diversité, type de débogage. Les noms qui servent de clés (recettes,
compétences requises) restent extraits (WF-D3).
