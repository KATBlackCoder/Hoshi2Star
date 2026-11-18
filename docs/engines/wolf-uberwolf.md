# Wolf RPG : ouvrir un jeu, déchiffré par l'app ou par UberWolf

Hoshi2Star lit et exporte seulement les fichiers libres de `Data/` d'un jeu Wolf RPG
(ADR-008), jamais une archive `.wolf`. Un jeu encore empaqueté est déchiffré **par
l'app**, avec la bibliothèque [unwolf](https://crates.io/crates/unwolf) (ADR-010).
[UberWolf](https://github.com/Sinflower/UberWolf), ou la commande `unwolf`, reste le
recours pour les cas que l'app refuse.

## Procédure

1. Travailler sur une **copie** du jeu : le déchiffrement écrit dans son dossier, et
   l'export s'applique par-dessus.
2. Dans Hoshi2Star, « Ouvrir un jeu » et choisir le dossier du jeu.
3. Si le jeu est encore empaqueté (ou garde des archives de texte à côté de fichiers
   déjà déchiffrés), l'app propose de le déchiffrer. Le dialogue dit ce qui va changer :
   - les archives de texte sont extraites en fichiers libres dans `Data/` :
     `Data/BasicData.wolf` et `Data/MapData.wolf`, ou `Data.wolf` en entier quand le
     jeu n'a que cette archive. Un fichier déjà présent n'est jamais remplacé ;
   - sur un jeu Wolf RPG Pro 3.5 protégé, la protection est levée ; les fichiers
     d'origine sont gardés dans `Data/BasicData/backup/` ;
   - les archives de texte sont ensuite **déplacées, jamais supprimées**, dans
     `Hoshi2Star-archives/` à la racine du jeu (`Data/MapData.wolf` →
     `Hoshi2Star-archives/Data/MapData.wolf`). Les archives d'images et de sons
     (`Picture.wolf`, `BGM.wolf`…) restent en place.
4. « Déchiffrer » : la progression s'affiche, « Arrêter » interrompt au fichier
   suivant (aucune archive n'est alors déplacée). À la fin, le rapport s'affiche et le
   projet s'ouvre.
5. Vérifier que le jeu se lance encore (`Game.exe` ou `GamePro.exe`).

Pour remettre le jeu dans son état empaqueté, remettre les archives de
`Hoshi2Star-archives/` à leur place ; l'app ne le fait pas.

## Quand l'app refuse

| Message | Cause | Que faire |
|---|---|---|
| Protection Wolf RPG Pro non levée | Jeu Pro V1 ou V2 | Déchiffrer avec UberWolf (levée de protection) |
| `Data.wolf` et un `Data/` non vide | `Data/` est peut-être une extraction partielle | Retirer l'un des deux, ou passer par UberWolf |
| Archive déjà dans `Hoshi2Star-archives/` | Un déchiffrement précédent | Déplacer l'ancienne copie hors du dossier du jeu |
| Jeu illisible après extraction | Pas de base lisible ou pas de carte | Vérifier le jeu ; UberWolf en recours |
| Archive non déplacée | Fichier verrouillé (jeu lancé sous Windows) | Fermer le jeu, recommencer (les archives ont été remises en place) |

## Recours : UberWolf ou `unwolf` en ligne de commande

1. Déchiffrer la copie : les fichiers en clair arrivent dans `Data/`
   (`Data/MapData/*.mps`, `Data/BasicData/*.project`, `*.dat`, `CommonEvent.dat`…).
   Pour un jeu Pro, lever aussi la protection.
2. Retirer du dossier du jeu les archives qui contiennent des textes : `Data.wolf`
   (racine), `Data/MapData.wolf`, `Data/BasicData.wolf`. Le jeu les lit avant `Data/` :
   tant qu'elles sont là, il ignorerait la traduction, et l'app refuse d'exporter.
3. Ouvrir le dossier du jeu dans Hoshi2Star.

## Ce que l'app attend pour ouvrir le jeu

- Un lanceur à la racine : `Game.exe`, `GamePro.exe` (Wolf Pro) ou `Game.ini`.
- `Data/BasicData/` contenant au moins un `.project` ou `CommonEvent.dat`.
  (Un `BasicData/` à la racine du jeu est reconnu comme Wolf, mais seuls
  `Data/MapData/` et `Data/BasicData/` sont lus : un tel jeu est refusé faute de base
  lisible dans `Data/BasicData/`.)
- Une base lisible parmi `SysDatabase.dat`, `DataBase.dat`, `CDataBase.dat` :
  son en-tête donne l'encodage (Shift-JIS pour les 2.x, UTF-8 pour les 3.x).
  Sinon le projet est refusé (`wolf_no_database`).
- Aucune archive de texte dans le dossier du jeu (`wolf_archive_present`).

## Export

Le zip `hoshi2star.zip` contient seulement les fichiers libres modifiés
(`Data/MapData/Map001.mps`, `Data/BasicData/CommonEvent.dat`…), à décompresser
par-dessus le jeu déchiffré. Chaque fichier passe la garde d'intégrité Wolf.

## Limites connues

- Hypothèse à vérifier en jeu (UWF-V) : le jeu ignore `Hoshi2Star-archives/` et lit
  les fichiers libres de `Data/`.
- Les polices chiffrées WolfX (`*.wolfx`) ne sont pas déchiffrées par l'app.
- Après application d'un export, `Data/` contient des traductions : ne pas
  réextraire le projet depuis ce dossier (la source serait la traduction), et
  un segment remis à vide garderait l'ancienne traduction. Garder une copie
  déchiffrée intacte du jeu (instantané d'origine Wolf prévu, `BACKLOG.md`).
- Les fichiers texte propres à un jeu (ex. `Evtext/*.txt`) ne sont pas
  extraits.
- Un nom de fichier non ASCII sans dossier (`決定.ogg`) ou un chemin contenant
  une ponctuation de phrase (`Picture/立ち絵（笑顔）.png`) est extrait comme
  texte : ne pas le traduire (la règle garde `ファイル名はicon.png` comme texte).
