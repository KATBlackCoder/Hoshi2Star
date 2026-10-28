# Wolf RPG : ouvrir un jeu déchiffré par UberWolf

Hoshi2Star ne lit plus les archives `.wolf` (ADR-008, plan UW). Tout jeu Wolf RPG,
quelle que soit sa version (2.x, 3.x, 3.31+, Pro, WolfX), s'ouvre depuis le
dossier `Data/` déchiffré par [UberWolf](https://github.com/Sinflower/UberWolf).

## Procédure

1. Travailler sur une **copie** du jeu (l'export s'applique par-dessus).
2. Déchiffrer la copie avec UberWolf : il écrit les fichiers en clair dans
   `Data/` (`Data/MapData/*.mps`, `Data/BasicData/*.project`, `*.dat`,
   `CommonEvent.dat`…).
3. Retirer du dossier du jeu les archives qui contiennent des textes :
   `Data.wolf` (racine), `Data/MapData.wolf`, `Data/BasicData.wolf`. Le jeu les lit
   avant `Data/` : tant qu'elles sont là, il ignorerait la traduction, et
   l'app refuse d'ouvrir ou d'exporter le projet. Les autres archives
   (`Picture.wolf`, `BGM.wolf`…) peuvent rester.
4. Vérifier que le jeu se lance encore (`Game.exe` ou `GamePro.exe`).
5. Ouvrir le dossier du jeu dans Hoshi2Star.

## Ce que l'app attend

- Un lanceur à la racine : `Game.exe`, `GamePro.exe` (Wolf Pro) ou `Game.ini`.
- `Data/BasicData/` (ou `BasicData/`) contenant au moins un `.project` ou
  `CommonEvent.dat`. Sinon : « moteur non détecté ».
- Une base lisible parmi `SysDatabase.dat`, `DataBase.dat`, `CDataBase.dat` :
  son en-tête donne l'encodage (Shift-JIS pour les 2.x, UTF-8 pour les 3.x).
  Sinon le projet est refusé (`wolf_no_database`).

## Export

Le zip `hoshi2star.zip` contient seulement les fichiers libres modifiés
(`Data/MapData/Map001.mps`, `Data/BasicData/CommonEvent.dat`…), à décompresser
par-dessus le jeu déchiffré. Chaque fichier passe la garde d'intégrité Wolf.

## Limites connues

- Après application d'un export, `Data/` contient des traductions : ne pas
  réextraire le projet depuis ce dossier (la source serait la traduction), et
  un segment remis à vide garderait l'ancienne traduction. Garder une copie
  déchiffrée intacte du jeu (instantané d'origine Wolf prévu, `BACKLOG.md`).
- Les fichiers texte propres à un jeu (ex. `Evtext/*.txt`) ne sont pas
  extraits.
