# Référence binaire — Fichiers .dat Wolf RPG

Document de référence pour l'implémentation Rust de F4-03 Steps 5–7.  
**Source primaire unique :** `docs/wolf-rpg-research.md`.  
Sections marquées **⚠️** = information absente du rapport — à vérifier dans WolfTL source
(Sinflower/WolfTL sur GitHub : `WolfTL.cpp`, `Database.h`, `CommonEvents.h`) avant d'implémenter.

---

## 1. Vue d'ensemble des fichiers .dat

### 1.1 Catégorisation WolfTL

WolfTL organise sa sortie JSON en **trois catégories** de fichiers traduisibles :

| Catégorie WolfTL | Fichiers source | Contenu |
|-----------------|----------------|---------|
| `CommonEvents`  | `CommonEvent.dat` (ou équivalent) | Événements partagés (dialogues, choix) |
| `Databases`     | Fichiers `.dat` dans `BasicData/` | Types, champs, données tabulaires |
| `Maps`          | `MapData/*.mps` | Cartes (implémentées via wolfrpg-map-parser) |

### 1.2 Fichiers .dat connus

Depuis le rapport (§1 "Structure des fichiers") :

| Fichier | Emplacement | Contenu |
|---------|-------------|---------|
| `Game.dat` | `BasicData/` | Paramètres de base du jeu (System DB equivalent) |
| `UserDB.dat` (ou `BasicData.dat`) | `BasicData/` | User Database — noms d'items, personnages, descriptions |
| `VarDB.dat` (ou `CDB`) | `BasicData/` | Variable Database — variables de jeu |
| `SysDB.dat` | `BasicData/` | System Database — police, couleurs, termes système |
| `CommonEvent.dat` | `Data/` ou `BasicData/` | Common Events — événements réutilisables |

> ⚠️ Les noms exacts (`UserDB.dat` vs `BasicData.dat`, emplacement de `CommonEvent.dat`)
> varient selon la version du moteur. Vérifier sur un jeu Wolf v2 réel (Mad Father freeware)
> et dans WolfTL source (`main.cpp` ou `WolfTL.cpp`, chercher les noms de fichiers ouverts).

### 1.3 Distinction encodage par version

| Version | Encodage texte dans .dat |
|---------|--------------------------|
| Wolf v1.x, v2.x | **Shift-JIS** (code page 932) |
| Wolf v3.x+ | **UTF-8** |

Détection via `WolfVersion::is_utf8()` (déjà implémenté dans `engines/detector.rs`).

---

## 2. Structure binaire des .dat

### 2.1 Ce qui est documenté dans le rapport

Le rapport mentionne que WolfTL parse les `.dat`/`.mps` mais **ne documente pas le layout
binaire champ par champ**. Les informations suivantes sont déduites du rapport + du plan F4-03 :

**Structure approximative d'un fichier Database .dat :**

```
[Header]
  u32 : magic number  (⚠️ valeur exacte par type de fichier inconnue)
  u32 : number_of_types

  for each type:
    u32    : type_id
    string : type_name   (⚠️ format string : longueur-préfixée u32 ou null-terminated ?)
    u32    : field_count
    for each field:
      u32    : field_id
      string : field_name
      u32    : field_type  (0=int, 1=string, 2=string_array, ... — ⚠️ valeurs exactes)

[Data section]
  u32 : data_count
  for each data row:
    [field values selon field_type de chaque champ]
```

> ⚠️ **Cette structure est approximative** (source : plan F4-03, lui-même basé sur WolfTL).
> Le format exact (magic numbers, endianness, format des strings, tailles des champs) DOIT
> être vérifié dans `Database.h` et `WolfTL.cpp` avant implémentation.

### 2.2 Ce qui n'est PAS documenté dans le rapport

Les éléments suivants sont **absents du rapport** et nécessitent la lecture de WolfTL source :

- Magic numbers par type de fichier (UserDB vs SysDB vs VarDB vs Game.dat)
- Format exact des strings (longueur préfixée `u32 + bytes` ? `u32 + bytes + null` ? null-terminated ?)
- Endianness des champs numériques (probablement little-endian comme le reste de Wolf RPG)
- Champs optionnels ou conditionnels selon la version du moteur
- Présence d'un checksum ou de flags d'intégrité
- Padding entre les sections
- Différences de format entre v1, v2, et v3

> **Décision pour l'implémentation** : lire `Database.h` + `WolfTL.cpp` sur GitHub
> (Sinflower/WolfTL) AVANT d'écrire une seule ligne de parsing binaire.

### 2.3 Analogie avec le format .mps

Les fichiers `.mps` sont parsés par `wolfrpg-map-parser` qui utilise :
- Strings longueur-préfixée : `u32_le(length)` + SJIS bytes + null terminator (le null est inclus dans `length`)
- Signatures de section (magic bytes) pour valider le début de chaque bloc
- Pas de RIFF/chunk standard — format propriétaire Wolf RPG

Le format `.dat` est vraisemblablement similaire (même moteur, même auteur), mais
**non garanti** sans vérification dans WolfTL.

---

## 3. Champs traduisibles identifiés

### 3.1 Catégories de texte traduisible (depuis le rapport §1)

Le rapport confirme que les `.dat` contiennent :
- **Noms d'items, compétences, personnages** (User DB)
- **Descriptions** d'items et compétences (User DB)
- **Définitions d'icônes** (`iconNNN.png` dans BasicData — ces chemins ne sont PAS à traduire)
- **Termes système** : police, couleurs (System DB — partiellement traduisible)

### 3.2 Codes de commande Wolf RPG (depuis le rapport §5 "WolfTL")

WolfTL inclut une **table complète des commandes Wolf** (codes 0–1000). Les codes pertinents
pour l'extraction de texte :

| Code | Commande | Texte traduisible |
|------|----------|-------------------|
| `101` | Message (ShowMessage) | ✅ Oui — dialogue affiché |
| `102` | Choices (ShowChoice) | ✅ Oui — options de choix |
| `122` | SetString | ⚠️ Parfois — dépend du contexte |

> ⚠️ **Relation avec les codes .mps** : les codes `101`/`102` dans WolfTL correspondent-ils
> aux signatures `0x01650000`/`0x02660000` de `wolfrpg-map-parser` ?
>
> La réponse probable est OUI (mêmes commandes, mêmes codes logiques), mais le format
> binaire dans un `.dat` CommonEvents peut différer du format dans un `.mps` : les `.mps`
> ont leur propre framing (PAGE_SIGNATURE, EVENT_SIGNATURE), les CommonEvents sont dans
> un `.dat` avec son propre header.
>
> À confirmer dans `CommonEvents.h` de WolfTL.

### 3.3 Champs à ignorer

Depuis le plan F4-03 + rapport :

**Toujours ignorer :**
- Chemins de fichiers : valeurs finissant par `.png`, `.wav`, `.ogg`, `.bmp`
- Champs numériques purs (IDs, flags, coordonnées, valeurs entières)
- Valeurs vides (`""`)

**Heuristique de détection texte japonais traduisible :**
- Contient des caractères Unicode hiragana (U+3041–U+3096)
- Contient des caractères katakana (U+30A0–U+30FF)
- Contient des kanji CJK (U+4E00–U+9FFF)

```rust
fn contains_japanese(text: &str) -> bool {
    text.chars().any(|c| {
        matches!(c,
            '\u{3041}'..='\u{3096}'  // hiragana
            | '\u{30A0}'..='\u{30FF}' // katakana
            | '\u{4E00}'..='\u{9FFF}' // CJK unified
        )
    })
}
```

**Noms de champs connus comme traduisibles** (depuis Wolf Trans + plan F4-03) :
- `name` / `名前`
- `description` / `説明`
- `note` / `備考`
- `message` / `text`

### 3.4 Format des clés de contexte

Wolf Trans utilise le format (depuis le rapport §5) :

```
MPS:TitleMap/events/0/pages/1/65/Message
```

Pour Hoshi2Star, le format adapté (cohérent avec Step 1 `WolfSegmentKind`) :

```
MapData/{map_name}/events/{e}/pages/{p}/{cmd}          # .mps (implémenté)
Database/{db_name}/{type_id}/{data_idx}/{field_name}    # .dat Database
CommonEvents/{event_name}/commands/{cmd_idx}            # .dat CommonEvents
```

---

## 4. Structure CommonEvents.dat

### 4.1 Ce qui est documenté

Le rapport confirme :
- Les Common Events contiennent des **commandes similaires aux events de maps** (dialogues, choix)
- WolfTL les catégorise séparément des Databases
- Le fichier s'appelle `CommonEvent.dat` ou équivalent (emplacement variable)

### 4.2 Analogie avec .mps

Les Common Events sont conceptuellement identiques aux pages d'events de cartes :
- Même commandes (Message=101, Choices=102)
- Même codes de contrôle et placeholders

**Différence probable :** pas de Map header ni de positionnement XY — juste une liste
d'events avec leurs pages de commandes, dans un conteneur `.dat`.

### 4.3 Ce qui n'est PAS documenté

> ⚠️ **Inconnues critiques pour l'implémentation :**
>
> - Est-ce que le format binaire des commandes dans `CommonEvent.dat` est **identique**
>   à celui des commandes dans les `.mps` ?
>   → Probable OUI (mêmes signatures `0x01650000` etc.) mais non confirmé.
>
> - Le `.dat` des Common Events a-t-il son propre magic number distinct des Databases ?
>   → À vérifier dans `CommonEvents.h` de WolfTL.
>
> - Comment les Common Events sont-ils nommés dans le fichier (champ `name` ?) ?
>   → Utilisé pour construire la clé `CommonEvents/{name}/...`
>
> - Y a-t-il une section header + une section data comme pour les Databases ?
>   Ou est-ce une liste plate d'events avec leurs commandes ?

---

## 5. Approche recommandée

### 5.1 Ordre d'implémentation

**Recommandation : commencer par les Databases (Step 5–6), puis CommonEvents (Step 7).**

Justification :
- Les Databases ont une structure tabulaire (header → types → champs → données) plus
  régulière et plus facile à tester avec des données synthétiques.
- Les Common Events nécessitent de parser des blocs de commandes — plus proche des `.mps`
  mais dans un conteneur `.dat` de format inconnu.
- Le risque le plus élevé est CommonEvents (format hybride .dat + commandes event).

**Risques par composant :**

| Composant | Risque | Raison |
|-----------|--------|--------|
| Database header | Moyen | Format tabulaire, mais magic numbers inconnus |
| Database data fields | Moyen | Types de données à valider empiriquement |
| CommonEvents commandes | Élevé | Format hybride — framing .dat + encoding commandes |
| Strings SJIS vs UTF-8 | Faible | `encoding_rs` déjà en place |

### 5.2 Stratégie de lecture WolfTL source

Avant toute implémentation, lire dans l'ordre :

1. `WolfTL.cpp` → `main()` ou point d'entrée → comprendre comment les fichiers sont ouverts
2. `Database.h` → structures C++ qui correspondent aux types Rust à créer
3. `CommonEvents.h` → vérifier si format binaire = .mps ou nouveau format
4. Chercher les constantes magiques (magic numbers) et les assertions de format

Grep utiles sur le repo WolfTL :
```bash
grep -rn "magic\|signature\|0x[0-9a-fA-F]\{4,\}\|fread\|ReadFile" src/
grep -rn "CommonEvent\|Database\|UserDB\|SysDB" src/
```

### 5.3 Plan de fallback si WolfTL source est illisible

Si le source WolfTL est difficile à analyser statiquement :
1. Utiliser un vrai fichier `.dat` d'un jeu freeware (Mad Father, One Way Heroics)
2. Ouvrir en éditeur hex (xxd / imhex) et comparer avec la sortie JSON de WolfTL
3. Reconstruire le format binaire par rétro-ingénierie empirique
4. Documenter les offsets découverts dans ce fichier

---

## 6. Tests synthétiques

### 6.1 Faisabilité des .dat synthétiques

> ⚠️ **Impossible à construire avant lecture de WolfTL source.**
>
> Contrairement aux `.mps` (format entièrement documenté par `wolfrpg-map-parser`),
> les `.dat` ne peuvent pas être construits synthétiquement sans connaître :
> - Les magic numbers exacts
> - Le format des strings (longueur-préfixée vs null-terminated)
> - La structure exacte de chaque section

### 6.2 Options pour les tests

**Option A — Synthétique partiel** (après lecture de WolfTL) :
- Construire des `.dat` minimaux en Rust une fois le format connu
- Même approche que les `.mps` synthétiques dans Step 4

**Option B — Fixture réel** :
- Utiliser des fichiers `.dat` d'un jeu Wolf v2 freeware (Mad Father)
- Placer dans `src-tauri/tests/fixtures/wolf/` (ne pas commiter du contenu copyrightable)
- Documenter dans le journal quelle version du jeu a été utilisée

**Option C — Snapshot testing** :
- Lancer WolfTL en CLI sur un vrai jeu → capturer la sortie JSON
- Implémenter le parser Rust → vérifier que la sortie correspond au JSON WolfTL
- Utile pour la validation fonctionnelle mais nécessite un environnement Windows

**Recommandation pour Step 5 (header seulement) :**
Une fois les magic numbers connus via WolfTL source, construire un `.dat` synthétique
minimal (header + 1 type + 0 données) pour valider le parser de schéma.

---

## 7. Référence rapide — Implémentation Rust cible

Types Rust à créer (depuis plan F4-03 Step 5) :

```rust
pub enum DatFieldType {
    Int,
    String,
    StringArray,
    Unknown(u32),
}

pub enum DatValue {
    Int(i32),
    String(String),
    StringArray(Vec<String>),
    Null,
}

pub struct DatField {
    pub id: u32,
    pub name: String,           // décoder SJIS si v2
    pub field_type: DatFieldType,
}

pub struct DatType {
    pub id: u32,
    pub name: String,           // décoder SJIS si v2
    pub fields: Vec<DatField>,
    pub data: Vec<Vec<DatValue>>,
}

pub struct DatFile {
    pub types: Vec<DatType>,
}
```

Fonction `read_string` : utiliser `engines::wolf::encoding::decode_wolf_text` déjà implémenté.
Format du curseur : `std::io::Cursor<&[u8]>` pour lecture séquentielle avec gestion d'offset.

---

## 8. Ressources

| Ressource | URL / Emplacement | Usage |
|-----------|-------------------|-------|
| WolfTL source | `github.com/Sinflower/WolfTL` | **Référence principale — lire avant Step 5** |
| Wolf Trans (Ruby) | `github.com/elizagamedev/wolftrans` | Format de contexte, liste des commandes |
| GARbro (ArcDX.cs) | `github.com/morkt/GARbro` | Format DXA (implémenté en F4-02) |
| Mad Father freeware | itch.io / téléchargement direct | Jeu de test Wolf v2, bien connu |
| dreamsavior.net | Manuel Wolf RPG | Référence des codes spéciaux `\...` |
| `docs/wolf-rpg-research.md` | Ce repo | Source primaire du rapport |
| `docs/references/ArcDX-reference.md` | Ce repo | Format DXA (F4-02) |
