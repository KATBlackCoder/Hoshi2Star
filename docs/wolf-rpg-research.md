# Support du format Wolf RPG dans Hoshi2Star (Tauri v2 + Rust) — Rapport de recherche

## TL;DR
- **Approche recommandée : bundler WolfTL (C++, Sinflower) comme sidecar Tauri v2 pour l'extraction/injection du texte (.dat/.mps → JSON), et UberWolf/UberWolfCli comme sidecar pour le déchiffrement des archives .wolf.** Une implémentation 100 % Rust native est possible pour le déchiffrement DXA (XOR 12 octets) en réutilisant la logique de GARbro et de la crate `wolfrpg-map-parser`, mais le parsing complet des bases de données et des Common Events, ainsi que le chiffrement v3.5 (ChaCha20 + WolfX), nécessitent un effort de rétro-ingénierie important.
- **Le déchiffrement legacy (Wolf v1/v2/v3 jusqu'à v3.31) est un simple XOR à clé 12 octets** ; le parsing du texte est documenté par Wolf Trans/WolfTL. Les versions **v3.5+/WolfX** introduisent un chiffrement ChaCha20 et un stockage par hash de la clé Pro — non réductible en Rust natif sans réutiliser UberWolf.
- **Les placeholders Wolf RPG sont proches mais PAS identiques à RPG Maker MV/MZ** : `\c[n]`, `\v[n]` existent dans les deux, mais Wolf ajoute des codes spécifiques (`\cself[]`, `\udb[]`, `\cdb[]`, `\sdb[]`, `\self[]`, `\r[base,ruby]`, `<L>/<C>/<R>`) qu'il faut protéger.

## Key Findings

1. **Deux modes de packaging** : soit `Data.wolf` unique (tout le dossier Data chiffré), soit dossier `Data/` contenant plusieurs `.wolf` (`BasicData.wolf`, `MapData.wolf`, `System.wolf`, `Picture.wolf`, `Icon.wolf`, etc.).
2. **Le chiffrement DXA est un XOR octet-par-octet à clé 12 octets** (`DXA_KEYSTR_LENGTH = 12`), symétrique (chiffrement = déchiffrement).
3. **Bug spécifique Wolf RPG** : le décalage de clé par fichier se base sur la **taille du fichier modulo 12**, pas sur sa position dans l'archive.
4. **Outils clés, tous de l'auteur Sinflower** : WolfDec (déchiffrement legacy, 311 étoiles / 51 forks), UberWolf (GUI + CLI, support Pro/v3.5/WolfX, 163 étoiles / 17 forks au 5 juin 2026), WolfTL (extraction/injection texte vers JSON, 22 étoiles / 4 forks). Tous en C++.
5. **Une crate Rust existe** : `wolfrpg-map-parser` (v0.6.0, MIT, 10K SLoC, propriétaire G1org1owo, ~38 téléchargements/mois — parse les .mps en structs Rust + sortie JSON), mais limitée aux maps.
6. **Tauri v2 supporte les sidecars** via `externalBin` + plugin-shell + permissions `shell:allow-execute`/`shell:allow-spawn`.

## Details

### 1. Structure des fichiers d'un projet Wolf RPG

Un projet/jeu Wolf RPG (WOLF RPG Editor, créé par Smoking WOLF / SmokingWOLF) est organisé autour de :

- **`Game.exe`** (ou `GamePro.exe` pour les jeux Pro) : exécutable racine, utilisé par UberWolf pour l'auto-détection (drag & drop de Game.exe → détecte le dossier Data et déchiffre). La version moteur se lit dans Propriétés → Détails → Version du fichier.
- **`Config.exe`**, **`Game.ini`**, **`Editor.exe`/`Editor.ini`** : configuration et éditeur.
- **`Data/`** : contient les données du jeu, organisées en sous-dossiers :
  - **`BasicData/`** : contient `Game.dat` (paramètres de base du jeu), les bases de données (System DB, User DB, Variable DB), les définitions d'items/compétences/personnages, les icônes (`iconNNN.png`).
  - **`MapData/`** : fichiers de cartes `.mps` (un par carte), contenant les événements, pages, et commandes (dialogues).
  - **`CommonEvent.dat`** (ou équivalent) : les événements communs (Common Events).
  - **Dossiers d'assets** : `Picture/`, `Sound/`, `CharaChip/`, `MapChip/`, etc.
- **Fichiers de projet** (côté éditeur, non distribués) : `.project`, et les `.dat`/`.mps` non chiffrés.

**Fichiers contenant du texte traduisible** : `MapData/*.mps` (dialogues, choix, noms d'événements), les bases de données dans `BasicData` (`.dat` : noms d'items, compétences, personnages, descriptions), et `CommonEvents` (`.dat`). WolfTL catégorise la sortie en trois types : **CommonEvents, Databases, et Maps**.

**Extensions** :
- `.wolf` : archive DXA chiffrée (peut être `Data.wolf` global ou fichiers individuels comme `BasicData.wolf`).
- `.mps` : fichier de carte (MapData), format binaire.
- `.dat` : bases de données et common events, format binaire.
- `.project` : fichier projet de l'éditeur (strings n'apparaissant que dans l'UI de l'éditeur, pas dans le jeu — généralement inutile à traduire).

### 2. Versions Wolf RPG et différences

| Version moteur | Format DXA | Chiffrement | Notes |
|---|---|---|---|
| Wolf v1.x | DXA v3/v4 | XOR clé 12 octets (32-bit) | Format historique |
| Wolf v2.0–2.28 | DXA v5/v6 | XOR clé 12 octets | Passage 64-bit en v6 (archives > 2 Go), encodage Shift-JIS |
| Wolf v3.0–3.31 | DXA v8 | XOR clé 12 octets (support v3.31+ ajouté à UberWolf en juin 2024) | Support UTF-8 (multi-langue), détection clé Pro possible |
| Wolf v3.5+ / WolfX / Pro | DXA + ChaCha20 | **ChaCha20 + protection WolfX** | Clé Pro stockée en **hash** (extraction impossible), nécessite "unprotect" |

Détail crucial pour le seuil de support : à partir de **Wolf RPG v3.595**, les outils comme LinguaGacha indiquent que ce n'est pas encore supporté. UberWolf gère jusqu'à v3.5/WolfX via un "WolfX cracker".

**Encodage** : Wolf RPG v2 et antérieur ne supporte qu'**une seule langue à la fois** (Shift-JIS par défaut, code page 932). Depuis **Wolf RPG v3, l'UTF-8 est supporté** (texte multilingue). rewolf-trans lit en Shift-JIS et écrit en GBK par défaut, avec options `--renc`/`--wenc`. Translator++ auto-détecte la version et bascule l'encodage automatiquement pour Wolf v2 et antérieur.

### 3. Algorithme de déchiffrement DXA (documenté précisément)

Le format d'archive est **DXA (DX Archive)** de la bibliothèque **DxLib** (japonaise). C'est le même format pour tous les jeux DxLib ; seule la clé 12 octets varie.

**Algorithme : XOR répété à clé 12 octets.** Constante `DXA_KEYSTR_LENGTH = 12`. La fonction canonique de DxLib `DXArchive::KeyConv` :

```c
void DXArchive::KeyConv(void *Data, int Size, int Position, unsigned char *Key) {
    Position %= DXA_KEYSTR_LENGTH;   // 12
    int j = Position;
    for (int i = 0; i < Size; i++) {
        ((u8*)Data)[i] ^= Key[j];
        if (++j == DXA_KEYSTR_LENGTH) j = 0;
    }
}
```

Équivalent C# dans GARbro (`ArcFormats/DxLib/ArcDX.cs`, morkt/GARbro — implémentation de référence à porter en Rust) :

```csharp
internal static void Decrypt (byte[] data, int index, int count, long offset, byte[] key) {
    int key_pos = (int)(offset % key.Length);
    for (int i = 0; i < count; ++i) {
        data[index+i] ^= key[key_pos++];
        if (key.Length == key_pos) key_pos = 0;
    }
}
```

XOR étant symétrique, **déchiffrement = chiffrement** (point essentiel : un même code Rust sert dans les deux sens).

**Bug spécifique Wolf RPG (confirmé)** : pour les données par fichier, le `Position` de départ passé n'est pas la position dans l'archive mais la **taille décompressée du fichier**. Donc le décalage de clé = `(UnpackedSize % 12)`. La TOC est déchiffrée avec un décalage `IndexOffset % 12` (v≤5) ou `0` (v6+). Citation de qazmlpok (himeworks) : pour un fichier de taille `0x197DD`, on utilise la clé décalée car `0x197DD % 0xC == 1`. La position dans l'archive n'a aucune importance. GARbro implémente ce comportement : pour version > 5, `dec_offset = dx_ent.UnpackedSize`.

**Clés connues (depuis la table `DECRYPT_MODES` de WolfDec `main.cpp`)** :
- Wolf v2.20 : `38 50 40 28 72 4F 21 70 3B 73 35 38` = ASCII **`8P@(rO!p;s5`** (clé la plus citée, utilisée avec `DxaDecode.exe -K:8P@(rO!p;s5`). La chaîne complète 12 caractères est en réalité `8P@(rO!p;s58`.
- Wolf v2.01 : `0F 53 E1 3E 04 37 12 17 60 0F 53 E1`.
- Wolf v2.10 : `4C D9 2A B7 28 9B AC 07 3E 77 EC 4C`.
- Wolf v3.10 et v3.173 : clés plus longues (40–46 octets) listées dans WolfDec.
- One Way Heroics : ASCII `nGui9('&1=@3#a` ; One Way Heroics Plus : `Ph=X3^]o2A(,1=@3#a`.
- Clé hex base DXv6 : **`C705CA7D8DE3DEF1D90C85F4`** (sortie de la fonction `keyCreate` de DxLib, sert à déchiffrer l'en-tête de 48 octets — attribuée à « Wolf RPG Editor v2.20 beta »).
- **Pas de clé (NULL)** : DxLib fait `memset(Key, 0xAA, 12)`, ce qui après `keyCreate` donne la clé constante **`55AA2055550655AA55D57C66`** (ex. Magic Castle RePure Aria / 楽園魔城リピュアリア). Un flag `DXA_FLAG_NO_KEY` dans l'en-tête indique cet état.

**Structure de l'en-tête DXA** :
- Signature ASCII **"DX"** (`0x5844` little-endian), suivie du champ version (5, 6, 8). Wolf RPG a utilisé les versions 5, 6 et 8 du DXLib Archiver.
- En-tête v6 (0x2C octets, champs 64-bit) : `IndexSize` (uint32 @0), `BaseOffset` (int64 @4), `IndexOffset` (int64 @0x0C), `FileTable` (int64 @0x14), `DirTable` (int64 @0x1C), `CodePage` (int32 @0x24).
- Tout l'en-tête est XORé via `Decrypt(header, 0, len, 4, key)` (offset 4 car la signature "DX.." précède).
- TOC : table de répertoires (`DirTable`) + table de fichiers (`FileTable`). Entrée fichier = {name offset, attributs (bit 0x10 = répertoire), data offset, unpacked size, packed size (-1 si non compressé)}. Data offset = `BaseOffset + offset`. Entrée répertoire = 0x40 octets en v6.
- Compression : LZSS custom DxLib et/ou Huffman dans les versions récentes (flag `DXA_FLAG_NO_HEAD_PRESS` pour l'en-tête).

**Récupération automatique de la clé ("guess key")** : possible par attaque à texte clair connu, car les champs 64-bit de l'en-tête contiennent beaucoup d'octets nuls. XORer le chiffré avec le clair (0x00) révèle directement les octets de clé. GARbro implémente `GuessKey`/`GuessKeyV6` (vérifie sig `'D','X',version`, valide l'index_offset, log `Restored key '{0}'`) ; UberWolf annonce la "détection automatique de clé". Code Python (DXv6) du fil himeworks :
```python
if (header[0xC:0x10] == header[0x24:0x28]) and (header[0x14:0x18] == header[0x2C:0x30]) and ...:
    key = header[0xC:0x10] + header[0x1C:0x20] + header[0x14:0x18]
```
Les 12 derniers octets du fichier sont aussi tous à 0 sauf un octet `0x40`, fournissant du texte clair supplémentaire.

**Changements v3.31+/v3.5/WolfX (depuis les release notes UberWolf)** :
- v0.3.0 (15 juin 2024) : « Added decryption support for the latest Wolf RPG Editor encryption (v3.31+); Added decryption and protection key detection support for the latest Wolf RPG Editor Pro encryption (v3.31+) ».
- v0.4.1 (déc. 2024) : passage 32→64-bit, packing v3.31, remplacement de la recherche de clé par injection par l'option "unprotect".
- v0.5.0 (4 mai 2025) : « Fully implemented Wolf RPG v3.5, including handling of WolfX files. NOTE: Starting with Wolf RPG v3.5, extracting the protection key for Pro games will no longer be possible, as a hash is stored instead of the actual key. Please use the unprotect option to remove the Pro protection altogether. » + ajout du chiffrement basé **ChaCha20** et déchiffrement des fichiers WolfX.
- v0.6.0 (août 2025) : contournement des mesures anti-unpacking de v3.5.
- v0.6.2 (1er mars 2026, dernière) : « Applied several fixes to the WolfRPG parser · Fixed problems with the WolfX cracker · Rearranged the position of WolfX cracking in the processing chain ».

### 4. Placeholders / codes d'échappement Wolf RPG (référence complète)

Tous les codes sont **sensibles à la casse**. Source : manuel officiel Wolf RPG Editor (miroir Dreamsavior).

**Insertion de nombre/chaîne** :
- `\v[XXX]` : valeur de la XXXe variable normale (équivalent RPG Maker `\v[n]`).
- `\v?[XXX]` : XXXe variable du jeu de variables de réserve ? (ex. `\v1[30]`).
- `\s[XXX]` : XXXe variable chaîne.
- `\self[X]` (0–9) : variable SelfX de l'événement de carte.
- `\cself[XX]` (0–99) : SelfX de l'événement commun (0–4 et 10–99 = nombres, 5–9 = chaînes).
- `\udb[A:B:C]` : User DB Type A, Data B, Field C.
- `\cdb[A:B:C]` : Variable DB.
- `\sdb[A:B:C]` : System DB.
- `\sys[XXX]` : variable système. `\sysS[XXX]` : chaîne système.
- Ordre de priorité (pour imbrication) : `\v < \v? < \self < \cself < \sys < \udb < \cdb < \sdb < \s < \sysS`.

**Propriétés de texte** :
- `\c[XX]` : couleur de police (depuis System DB Type 12) — équivalent RPG Maker `\c[n]`.
- `\f[XX]` : taille de police. `\m[XX]` : taille max de ligne.
- `\E` : contours ("edge"). `\N` : retire les contours (défaut).
- `\-[XX]` : resserre l'espacement de XX pixels.
- `\font[X]` : change vers la sous-police X (`\font[0]` = défaut).
- `\A+` / `\A-` : active/désactive l'anti-aliasing.

**Contrôle d'affichage** :
- `\\` : un `\` littéral.
- `\!` : pause jusqu'à appui touche. `\.` : attend 0,25 s. `\^` : fin forcée sans input.
- `\>` / `\<` : affichage instantané / stop.
- `\mx[??]`, `\my[??]` : décalage X/Y. `\ax[??]`, `\ay[??]` : ancrage forcé.
- `\sp[??]` : vitesse de frappe. `\space[??]` : hauteur de saut de ligne.
- `\r[Base,Ruby]` : texte ruby au-dessus de la base (ex. furigana) — **important : contient du texte traduisible à l'intérieur**.
- `\i[num]` (0–999) : affiche `iconNNN.png` depuis BasicData. `\img[chemin]` : affiche une image.
- `<L>` / `<C>` / `<R>` : alignement gauche/centre/droite.

**Codes spécifiques aux noms de champs de DB** : `\d[X]` (numéro de champ + X), variantes `\udb[A:B]`/`\cdb[A:B]`/`\sdb[A:B]`.

**Comparaison RPG Maker MV/MZ** : `\v[n]` et `\c[n]` sont communs. Mais Wolf utilise des codes DB (`\udb`, `\cdb`, `\sdb`), des variables self (`\self`, `\cself`), le ruby `\r[,]`, et les balises d'alignement `<L>/<C>/<R>` qui n'existent pas tels quels dans RPG Maker. **Hoshi2Star ne peut PAS réutiliser tel quel son moteur de regex MV/MZ** ; il faut un jeu de patterns Wolf dédié. LinguaGacha avertit que ces codes (`\c[1]`, etc.) doivent être préservés exactement, sous peine de crash du jeu.

### 5. Outils existants — analyse

**WolfDec (Sinflower, C++, 311 étoiles / 51 forks)** : déchiffreur simple, drag & drop de `.wolf`. Utilise une version modifiée de DxLib. Table de clés intégrée. **Obsolète pour les nouveaux jeux** → renvoie explicitement vers UberWolf.

**UberWolf / UberWolfCli (Sinflower, C++, 163 étoiles / 17 forks au 5 juin 2026, dernière release v0.6.2 du 1er mars 2026)** : déchiffreur GUI + CLI. Auto-détection (drag Game.exe/GamePro.exe → détecte Data, déchiffre tout). Support Wolf Pro, détection de clé de protection, "unprotect", v3.5/WolfX. `UberWolfCli.exe "chemin\Game.exe"` ou un dossier ou un `.wolf` unique. **C'est l'outil de déchiffrement de référence en 2025-2026.**

**WolfTL (Sinflower, C++, MIT, 22 étoiles / 4 forks)** : extrait les données traduisibles des `.dat`/`.mps` vers JSON. Code de parsing basé sur Wolf Trans. Modes : `create` (créer le patch/dump), `patch`, `patch_ip` (in-place). Sortie : dossier `dump` avec CommonEvents/Databases/Maps. **Supporte les jeux WolfPro** (après patch, fichiers déchiffrés ouvrables dans l'éditeur). Inclut une table complète des commandes Wolf (codes 0–1000 : Message=101, Choices=102, SetString=122, etc.). **C'est le backend utilisé par Translator++ pour Wolf RPG.**

**Wolf Trans (elizagamedev, Ruby)** : pionnier, inspiré de RPG Maker Trans. Extrait vers fichiers texte (`> BEGIN STRING / > CONTEXT / > END STRING`). Format de contexte : `MPS:TitleMap/events/0/pages/1/65/Message`. **Nécessite des données déjà déchiffrées** (.project/.dat/.mps, pas .wolf). Projet ancien, peu maintenu. Fork TokcDK ajoute le support chinois.

**rewolf-trans (KCFindstr, TypeScript/npm, version 0.1.8 publiée il y a ~4 ans, 11 étoiles GitHub, aucun autre projet npm ne l'utilise)** : réécriture améliorée de Wolf Trans. Format de patch amélioré (localisation unique des strings), catégorisation par risque, encodage custom (`--renc`/`--wenc`, défaut lecture Shift-JIS / écriture GBK). Utilisable en CLI (`npx rewolf-trans`) ou comme dépendance Node. Commandes `generate` et `apply`. **Atout pour Tauri : c'est du Node.js, packageable en sidecar.** Limite : en développement, breaking changes, dernière publication ancienne.

**wolfrpg-map-parser (crate Rust, v0.6.0, MIT, 365 Ko, 10K SLoC, #308 in Game dev, ~38 téléchargements/mois, propriétaire G1org1owo)** : parse les `.mps` en arbre de structs Rust + sortie JSON. `Map::parse(&bytes)`. **Seule brique Rust native existante**, mais limitée aux maps (pas les DB ni common events).

**Translator++ (Dreamsavior, NW.js/Node)** : éditeur CAT GUI. Utilise WolfTL comme parser Wolf RPG (sélection "WolfTL" à la création du projet). Détecte la version Wolf et auto-bascule l'encodage. Workflow : Create Project → Wolf RPG → WolfTL → traduire → Export to folder.

**LinguaGacha (neavo)** : traducteur LLM. Pour Wolf : déchiffrer d'abord avec WolfDec/UberWolf, puis créer un projet WolfTL dans Translator++, exporter le `.trans`, traduire dans LinguaGacha, réimporter. **Limite documentée : jeux avec version moteur ≥ 3.595 non supportés.**

**RuneTranslate** : app desktop Windows, "engine-aware". Supporte Wolf RPG end-to-end (extraction → traduction → build jouable). Comparable conceptuellement à Hoshi2Star (export d'un build redistribuable plutôt qu'injection runtime).

**MTool (AdventCirno)** : outil joueur, injection runtime/traduction one-click. Supporte Wolf RPG (1.00 à dernière version). Mais : modifie le jeu en place (les copies injectées par MTool ne peuvent pas être réutilisées par d'autres outils).

### 6. Intégration Tauri v2 + Rust

**Tauri v2 supporte les sidecars** (binaires externes) :
- Configuration : `bundle > externalBin` dans `tauri.conf.json` (ex. `"binaries/uberwolf-cli"`).
- Le binaire doit être nommé avec le suffixe target triple (ex. `uberwolf-cli-x86_64-pc-windows-msvc.exe`).
- Permissions dans `src-tauri/capabilities/default.json` : `shell:allow-execute` ou `shell:allow-spawn`, avec la définition des arguments autorisés (validators regex) pour la sécurité.
- Appel depuis Rust : `app.shell().sidecar("uberwolf-cli").arg(...).output()`. Depuis JS : `Command.sidecar('binaries/uberwolf-cli', [...])`.
- Pour la lecture/écriture des fichiers `.wolf`/`.dat`/`.mps`, utiliser le plugin `fs` de Tauri v2 avec les permissions `fs:allow-read`/`fs:allow-write` ou, plus simple, gérer les I/O directement côté Rust (accès filesystem natif) plutôt que via la couche permissions du webview.

**Options pour Hoshi2Star** :

1. **Sidecar C++ (UberWolfCli + WolfTL)** — *recommandé pour démarrer rapidement* : bundler les deux exécutables Windows. UberWolfCli déchiffre les `.wolf`, WolfTL extrait/réinjecte le texte en JSON. Round-trip : `WolfTL.exe Data OutputDir create` → traduire le JSON dans Hoshi2Star → `WolfTL.exe Data OutputDir patch`. Avantages : robuste, gère v3.5/WolfX/Pro, maintenu activement (WolfTL et UberWolf v0.6.2 mars 2026). Inconvénient : Windows-only (binaires C++), dépendance externe, licence (UberWolf à vérifier ; WolfTL est MIT).

2. **Sidecar Node.js (rewolf-trans)** : packager rewolf-trans avec `pkg` en binaire autonome. Avantage : pur JS, multiplateforme, format de patch propre. Inconvénients : ne déchiffre PAS les `.wolf` (il faut quand même UberWolf), peu maintenu, ne gère pas v3.5.

3. **Implémentation Rust native** : 
   - Déchiffrement DXA legacy (XOR 12 octets + guess key) : **faisable en Rust natif** (~quelques centaines de lignes), en portant la logique de GARbro `ArcDX.cs`/DxLib. Pas de crate clé en main existante pour le déchiffrement, mais simple.
   - Parsing maps : réutiliser `wolfrpg-map-parser`.
   - Parsing DB/Common Events : à porter depuis WolfTL (C++) ou Wolf Trans (Ruby) — effort significatif.
   - v3.5/WolfX (ChaCha20 + hash) : **non réalisable raisonnablement en Rust natif** sans rétro-ingénierie lourde → garder UberWolf en sidecar pour ces cas.

4. **Bindings C FFI vers UberWolfLib/WolfTL** : compiler les libs C++ en bibliothèques statiques liées à Rust via FFI. Avantage : intégration plus fine que des process. Inconvénient : complexité de build cross-platform, ces libs ne sont pas conçues comme API stables.

**Gestion du round-trip par les outils existants** : tous suivent extract → translate → inject. WolfTL et rewolf-trans réécrivent les fichiers binaires `.dat`/`.mps` directement (pas besoin de re-chiffrer si on supprime les `.wolf` originaux — le jeu lit les fichiers déchiffrés en priorité). Pour une distribution propre, on peut re-packager via l'éditeur Wolf (ゲームデータの作成) ou laisser les fichiers déchiffrés.

### 7. Cas pratiques

**Jeux de test populaires** :
- **Mad Father** (Sen / Miscreant's Room) : le jeu Wolf RPG le plus célèbre, version freeware + Steam/Switch. Bon cas de test (traduit par vgperson). Attention : la version Steam remake utilise un chiffrement plus récent.
- **One Way Heroics / Plus** (SmokingWOLF lui-même, l'auteur du moteur) : sur Steam, clé connue (`nGui9('&1=@3#a`).
- **Misao, The Crooked Man, LiEat, Paranoiac, Alicemare** : titres horreur Wolf populaires (Ib et The Witch's House sont RPG Maker, pas Wolf).
- Jeux DLsite (ex. Goddess of Memorier RJ171667) : cas typiques avec dossier Data multi-`.wolf`.

**Cas limites** :
- Jeux v3.5+/WolfX : nécessitent UberWolf récent (le "WolfX cracker"), pas de solution Rust native.
- Clé Pro stockée en hash (v3.5+) : impossible d'extraire la clé, utiliser "unprotect".
- Jeux avec clé custom non dans la table : utiliser la détection automatique (guess key) ou OllyDbg sur les handles de fichier.
- Encodage : Shift-JIS (v2) vs UTF-8 (v3+) — détecter via la version moteur.
- Noms de fichiers de cartes en japonais : peuvent nécessiter un renommage si l'éditeur tourne en locale non-japonaise.

**Ressources communautaires** :
- GitHub : Sinflower (WolfDec, UberWolf, WolfTL, WolfSave, SRPG-ToolBox), elizagamedev/wolftrans, KCFindstr/rewolf-trans, TokcDK/wolftrans, morkt/GARbro (ArcDX.cs — implémentation C# de référence du déchiffrement DXA), Daviid-P/Wolf_RPG_Decompyler (port Python, versions 5/6/8, `DXA_FLAG_NO_KEY`).
- vgperson.com : guide de traduction Wolf RPG.
- gametrans.bitbucket.io/wolf.html : tutoriel de déchiffrement.
- dreamsavior.net : manuel Wolf RPG (référence des codes spéciaux), docs Translator++.
- himeworks.com/tools/dxextract (commentaires) : détails du format DXA.
- crates.io : wolfrpg-map-parser.

## Recommendations

**Étape 1 (MVP) — Sidecars C++ :** Bundler **UberWolfCli** (déchiffrement) + **WolfTL** (extract/inject JSON) comme sidecars Tauri v2. C'est la voie la plus rapide et la plus robuste vers un support fonctionnel couvrant Wolf v1→v3.5/WolfX. Configurer `externalBin` + permissions `shell:allow-execute` avec arguments validés. Pipeline : détecter Game.exe → UberWolfCli déchiffre → WolfTL `create` → parser le JSON dans le modèle Hoshi2Star → traduction → WolfTL `patch`.
- *Bénéfice* : support immédiat des jeux modernes, réutilisation du parser de référence (le même que Translator++).
- *Seuil de changement* : si la dépendance à des binaires C++ Windows-only ou la licence d'UberWolf pose problème, passer à l'étape 2/3.

**Étape 2 — Déchiffrement Rust natif legacy :** Implémenter en Rust le déchiffrement DXA XOR 12 octets + détection automatique de clé (guess key) pour les versions ≤ v3.31, en portant GARbro `ArcDX.cs`. Garder UberWolf en fallback sidecar uniquement pour v3.5+/WolfX.
- *Bénéfice* : moins de dépendances externes, contrôle total sur le pipeline legacy.
- *Seuil* : ne franchir cette étape que si une part significative des jeux cibles est en ≤ v3.31 (mesurer sur un échantillon réel avant d'investir).

**Étape 3 — Parsing Rust natif :** Intégrer la crate `wolfrpg-map-parser` pour les maps, et porter le parsing DB/CommonEvents depuis WolfTL. Réutiliser le format de contexte de Wolf Trans (`MPS:Map/events/N/pages/N/N/Message`) pour un round-trip fiable et une réconciliation stable des traductions.

**Pour les placeholders :** créer un module de protection regex Wolf-spécifique (distinct de MV/MZ), couvrant tous les codes `\...` et `<L>/<C>/<R>`, avec traitement spécial de `\r[base,ruby]` (le ruby est traduisible). Avertir l'utilisateur/le LLM de préserver ces codes exactement.

**Pour l'encodage :** détecter la version moteur via Game.exe (Propriétés → Version) ; Shift-JIS (cp932) pour v2 et antérieur, UTF-8 pour v3+. Refuser ou avertir pour v3.595+ (non supporté par l'écosystème actuel).

## Caveats
- **Licences** : vérifier la licence d'UberWolf avant bundling commercial (WolfTL est MIT ; UberWolf n'a pas de licence explicite confirmée dans mes sources). rewolf-trans et wolfrpg-map-parser sont MIT.
- **Binaires Windows-only** : UberWolf/WolfTL sont C++/Windows. Pour une app Tauri multiplateforme, le support Wolf serait limité à Windows (ce qui est cohérent avec le fait que les jeux Wolf RPG sont Windows-only).
- **v3.5/WolfX évolue vite** : UberWolf reçoit encore des correctifs en 2026 (WolfX cracker, v0.6.2 du 1er mars 2026). Une implémentation native risquerait l'obsolescence ; le sidecar permet de suivre les mises à jour amont.
- **Détails ChaCha20 v3.5 non documentés au niveau du code** : seules les release notes UberWolf confirment l'usage de ChaCha20 ; la dérivation de clé/nonce n'est pas publiquement documentée.
- **Maintenance des outils** : Wolf Trans (Ruby) et rewolf-trans (npm, dernière publication il y a ~4 ans, 11 étoiles, aucun dépendant npm) sont peu maintenus ; WolfTL/UberWolf sont les plus actifs.
- **Aspect éthique/légal** : la plupart des auteurs Wolf RPG ne souhaitent pas que leurs jeux soient édités/extraits sans permission (avertissement explicite de vgperson) — à intégrer dans les CGU de Hoshi2Star.
- **Source du code `KeyConv`/`Decrypt`** : les extraits proviennent de DxLib upstream et de GARbro (`ArcDX.cs`), fonctionnellement identiques au code embarqué dans UberWolfLib ; les blobs `.cpp` exacts d'UberWolfLib (`3rdParty/DXArchive*.cpp`, `src/WolfDec.cpp`/`WolfPro.cpp`) n'ont pas pu être lus directement (robots GitHub).