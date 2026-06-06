# Hoshi2Star — Approche Wolf RPG

> Analyse technique basée sur `docs/wolf-rpg-research.md` et le code existant de Hoshi2Star v0.3.2.
> Produit en mode ANALYSE UNIQUEMENT — aucun fichier code modifié.

---

## Résumé exécutif

**Approche retenue : Rust natif pour Wolf v1/v2/v3.0–3.31 + sidecar UberWolfCli Windows-only en fallback explicite pour WolfX/v3.5+.**

Raison principale : le rapport recommande les sidecars C++ comme voie rapide, mais omet la contrainte critique que le dev (CachyOS Linux) ne pourrait pas tester Wolf RPG sur sa propre machine avec des binaires Windows-only. La majorité (~75-80%) du catalogue DLsite ciblé (jeux 2010-2023) utilise DXA v5/v6/v8 XOR 12 octets — parfaitement portable en Rust. Le parsing texte nécessite un portage partiel de WolfTL, faisable en F4.

Risque principal : le parsing `.dat` (databases/common events) est l'effort technique le plus lourd. La crate `wolfrpg-map-parser` couvre uniquement les maps. L'encodage Shift-JIS nécessite d'ajouter `encoding_rs` au projet.

---

## Analyse des options

### Option A — Sidecars C++ (UberWolfCli + WolfTL)

**Avantages**
- Couverture complète Wolf v1→WolfX dès le premier jour
- WolfTL est le même backend que Translator++ — compatibilité prouvée
- Pipeline simple : UberWolfCli déchiffre → WolfTL `create` → JSON → traduction → WolfTL `patch`
- Activement maintenu (UberWolf v0.6.2 du 1er mars 2026, correctifs WolfX cracker)
- WolfTL est MIT

**Inconvénients**
- **BLOQUANT pour le dev Linux** : UberWolfCli et WolfTL sont des binaires C++/Windows. Le dev sur CachyOS ne peut pas exécuter ces binaires nativement → impossible de tester Wolf RPG pendant le développement sans VM Windows ou Wine
- **Licence UberWolf non confirmée** : pas de fichier LICENSE explicite mentionné dans le rapport. Bundler du code sans licence dans un produit commercial à 29 $ = risque légal réel
- WolfTL JSON output : le format `dump/{CommonEvents,Databases,Maps}/` n'est pas directement compatible avec `Vec<ExtractedSegment>` d'Hoshi2Star — couche d'adaptation nécessaire
- Deux processus sidecars distincts + coordination par répertoire temporaire = complexité IPC
- Si UberWolf arrête le support WolfX : bloquant pour les nouveaux jeux

**Verdict pour Hoshi2Star** : Viable uniquement comme fallback Windows pour WolfX/v3.5+. Non viable comme approche principale car non testable par le dev.

---

### Option B — Sidecar Node.js (rewolf-trans)

**Avantages**
- MIT, conceptuellement multiplateforme

**Inconvénients**
- **Abandonné** : dernière publication npm il y a ~4 ans, 11 étoiles, aucun dépendant npm — projet sans maintenance active
- **Ne déchiffre pas les `.wolf`** : nécessite quand même UberWolfCli en amont → les deux sidecars requis
- Ne gère pas Wolf v3.5/WolfX
- Packaging Node.js en sidecar autonome (via `pkg`/`ncc`) : complexité de build additionnelle
- Format de patch incompatible avec la structure Hoshi2Star (`Vec<ExtractedSegment>` + JSON Pointer)

**Verdict** : À exclure. N'élimine aucun problème d'Option A, en ajoute de nouveaux. L'abandon du projet est disqualifiant pour un produit commercial.

---

### Option C — Rust natif

**Avantages**
- Cohérent avec l'architecture existante (`engines/mv_mz/` pattern)
- Testable sur Linux par le dev
- Aucune dépendance externe à surveiller/bundler
- Licence MIT exclusive (encoding_rs + wolfrpg-map-parser sont MIT)
- Contrôle total sur le format des clés d'extraction (compatible JSON Pointer ou équivalent Wolf)
- Déchiffrement DXA XOR est documenté précisément — portage depuis GARbro `ArcDX.cs` est direct

**Inconvénients**
- WolfX/v3.5 (ChaCha20 + hash) : **non réalisable en Rust natif sans reverse engineering** — dérivation ChaCha20 non documentée
- `.dat` parsing (bases de données + CommonEvents) : effort significatif, pas de crate Rust existante
- Effort total estimé : 3-5 semaines solo dev

**Sous-section : ce que `wolfrpg-map-parser` couvre réellement**

La crate `wolfrpg-map-parser` v0.6.0 (MIT, ~38 dl/mois, G1org1owo) offre :
- `Map::parse(&bytes)` → structs Rust + sortie JSON pour les fichiers `.mps`
- **Couvre uniquement les maps** (`MapData/*.mps`) — les dialogues de carte sont la source principale de texte
- **NE couvre PAS** : `.dat` (databases User DB/System DB/Variable DB, CommonEvents, BasicData `Game.dat`) — ce sont pourtant les noms de personnages, items, compétences, descriptions
- Activité maintainer : 38 téléchargements/mois = adoption faible ; risque d'abandon non nul
- Alternative si la crate disparaît : le parsing `.mps` est documenté (Wolf Trans + WolfTL) — portable manuellement

**Couverture réelle en utilisant uniquement `wolfrpg-map-parser`** : ~60% du texte (dialogues de cartes) ; les 40% restants (items, personnages, compétences, descriptions, CommonEvents) nécessitent le portage `.dat`.

**Verdict** : Approche principale pour F4 avec une contrainte claire — WolfX est F5, pas F4.

---

### Option D — Hybride (rapport : Étape 1 sidecars → Étape 2 Rust)

**Avantages**
- MVP rapide côté utilisateur Windows
- Migration progressive

**Inconvénients**
- **Risque de ne jamais faire l'étape 2** : l'histoire d'Hoshi2Star le montre déjà (VX Ace "code prêt, désactivé"). Un sidecar qui "fonctionne" n'a aucune pression commerciale pour être remplacé
- Le dev Linux ne peut pas tester l'étape 1 (Windows-only) → développement en aveugle, pas de workflow CI vert
- Deux architectures maintenues en parallèle pendant la transition = charge cognitive double
- Licence UberWolf non résolue reste un risque permanent si l'étape 2 est différée

**Verdict** : Rejeté comme approche pour F4. L'ordre devrait être inversé : Rust natif d'abord (testable), sidecar Windows en complément pour WolfX. C'est l'Option C avec fallback ciblé.

---

## Recommandation pour F4

### Approche retenue

**Rust natif pour Wolf v1/v2/v3.0–3.31 (couvre ~75–80% du catalogue DLsite ciblé) + sidecar UberWolfCli Windows-only pour WolfX/v3.5+ (F5).**

Justification :
1. Le dev peut tester sur sa propre machine (Linux) — pas de développement en aveugle
2. Les jeux DLsite non traduits ciblés sont majoritairement des productions 2010-2022 (ère Wolf v2/v3.0) — les jeux v3.5+ récents (2024+) sont la minorité en F4
3. La licence UberWolf non confirmée est incompatible avec un produit commercial — F5 résout ce point ou trouve une alternative
4. L'architecture résultante (`engines/wolf/`) sera cohérente avec `engines/mv_mz/` — maintenable solo

---

### Ce qui est faisable en Rust natif pour F4

| Composant | Effort estimé | Dépendance | Notes |
|-----------|--------------|-----------|-------|
| Détection Wolf dans `detector.rs` | 0.5 j | Aucune | Cherche `Game.exe`/`Game.ini`/`BasicData/` ou dossier `Data/` avec `.wolf` |
| Lecture version moteur (Game.exe) | 1 j | Aucune | Windows PE version info (champ FileVersion) ou lecture `Game.ini` si disponible |
| Déchiffrement DXA XOR v5/v6/v8 | 2 j | Aucune | Porter GARbro `ArcDX.cs` — algorithme complètement documenté dans le rapport |
| GuessKey / GuessKeyV6 | 1 j | Aucune | Attaque texte clair connu (champs nuls de l'en-tête) — documenté dans le rapport |
| Décompression LZSS/Huffman DXA | 1 j | Aucune | DxLib custom — à vérifier si réellement utilisé dans les jeux Wolf v2/v3 ciblés |
| Parsing `.mps` (maps) | 1 j | `wolfrpg-map-parser` | Wrapping avec extraction `ExtractedSegment` Wolf |
| Parsing `.dat` (DB + CommonEvents) | 4-6 j | Aucune | Portage depuis WolfTL C++ — partie la plus lourde |
| Encodage Shift-JIS → UTF-8 | 0.5 j | `encoding_rs` | Déjà utilisé partout dans l'écosystème Rust — ajout trivial à Cargo.toml |
| Injector `.mps` (réécriture binaire) | 2 j | `wolfrpg-map-parser` ou custom | Réécriture du binaire avec texte traduit |
| Injector `.dat` | 2 j | Aucune | Mirror du parser |
| `engines/wolf/extractor.rs` (orchestre) | 1 j | Tous ci-dessus | Pattern identique à `engines/mv_mz/extractor.rs` |
| Tests round-trip Wolf v2 (Mad Father freeware) | 1 j | Jeux de test | Critère de sortie F4 |

**Total estimé : ~16-20 jours solo dev.**

---

### Ce qui nécessite une dépendance externe

| Composant | Dépendance | Alternative si disparaît |
|-----------|-----------|------------------------|
| Encoding Shift-JIS | `encoding_rs` (Mozilla, MIT, très stable) | `chardetng` + `codepage` ; risque quasi nul |
| Parsing `.mps` (option) | `wolfrpg-map-parser` (MIT) | Parser manuel depuis spec Wolf Trans — faisable en 3 j supplémentaires |
| WolfX/v3.5+ déchiffrement | UberWolfCli sidecar (F5) | UberWolf source C++ forké + compilé ; ou bloquer le support WolfX |

**Recommandation sur `wolfrpg-map-parser`** : l'utiliser comme accélérateur de démarrage mais implémenter en parallèle une lecture directe si la crate se révèle insuffisante ou abandonnée. À 38 dl/mois, ne pas en faire un point de défaillance unique.

---

### Impact Linux

**Hoshi2Star cible Linux + Windows. L'approche Rust native résout le problème.**

- Dev CachyOS : peut développer et tester Wolf RPG v1/v2/v3 directement
- Utilisateurs Linux (Proton/Wine pour jouer aux jeux Wolf) : peuvent utiliser Hoshi2Star pour préparer des traductions même si les jeux Wolf sont Windows-only
- WolfX/v3.5+ : afficher un message explicite `"Ce jeu nécessite Windows (WolfX/v3.5+) — support prévu dans une future version"` — acceptable pour F4

**Si le sidecar UberWolfCli est ajouté en F5 pour WolfX :**
- Linux : désactiver la détection WolfX, afficher le message ci-dessus
- Windows : le sidecar est disponible et invoqué automatiquement
- La couche d'abstraction `wolf::decryptor` doit avoir un trait `WolfDecryptor` avec deux implémentations : `DxaDecryptor` (Rust, cross-platform) et `UberWolfSidecar` (Windows-only, runtime-gated via `#[cfg(target_os = "windows")]`)

---

### Placeholders Wolf RPG

**Nouveaux patterns par rapport à MV/MZ** (à ajouter comme `Engine::Wolf` dans `llm/tokenizer.rs`) :

```
// Déjà couverts par MV/MZ (réutilisables tels quels) :
\v[n]   → \v\[\d+\]        (valeur variable — même syntaxe)
\c[n]   → \c\[\d+\]        (couleur)
\n      → newline littéral  (même)

// NOUVEAUX — à ajouter dans le mode Wolf :
\s\[\d+\]                  // variable chaîne
\self\[\d\]                // self variable événement carte
\cself\[\d{1,2}\]          // self variable événement commun
\v\?\[\d+\]                // variable réserve (ex. \v1[30])
\udb\[\d+:\d+:\d+\]        // User DB [type:data:field]
\cdb\[\d+:\d+:\d+\]        // Variable DB
\sdb\[\d+:\d+:\d+\]        // System DB
\sys\[\d+\]                // variable système
\sysS\[\d+\]               // chaîne système
\f\[\d+\]                  // taille police
\m\[\d+\]                  // taille max ligne
\-\[\d+\]                  // espacement pixels
\font\[\d\]                // sous-police
\i\[\d{1,3}\]              // icône (0-999)
\sp\[\d+\]                 // vitesse frappe
\space\[\d+\]              // hauteur saut ligne
\mx\[\d+\]  \my\[\d+\]    // décalage X/Y
\ax\[\d+\]  \ay\[\d+\]    // ancrage forcé
<L>  <C>  <R>              // alignement (balises HTML-like)
\E  \N  \A\+  \A-          // effets texte
```

**Cas spécial `\r[Base,Ruby]`** : le ruby est du furigana (guide de lecture). Le texte `Base` est potentiellement traduisible ; le texte `Ruby` peut être supprimé pour les langues sans besoin de furigana. Approche recommandée : tokeniser `\r[...]` comme un placeholder opaque unique (protection stricte), mais avertir le traducteur via QA que le ruby est présent.

**Compatibilité avec le tokenizer existant** : créer `Engine::Wolf` dans `llm/tokenizer.rs` avec une `Regex` distincte. Les `⟦ph_N⟧` tokens sont réutilisés tels quels — le mécanisme de restauration est identique.

---

### Ordre d'implémentation recommandé pour F4

```
Étape 1 — Fondations (1 semaine)
  [ ] Ajouter `encoding_rs` à Cargo.toml
  [ ] Ajouter `wolfrpg-map-parser` à Cargo.toml  
  [ ] Créer `src-tauri/src/engines/wolf/mod.rs`
  [ ] Implémenter détection Wolf dans `detector.rs`
      (Game.exe présent + BasicData/ ou Data/*.wolf)
  [ ] Ajouter `Engine::Wolf` à l'enum dans `detector.rs`
  [ ] Créer `Engine::Wolf` mode dans `llm/tokenizer.rs`

Étape 2 — Déchiffrement DXA (1 semaine)
  [ ] `engines/wolf/decryptor.rs` :
      - Lecture en-tête DXA (signature "DX", version 5/6/8)
      - Table de clés hardcodée (Wolf v2.20, v2.01, v2.10, v3.x)
      - Algorithme KeyConv XOR 12 octets (depuis GARbro ArcDX.cs)
      - GuessKeyV6 (attaque texte clair connu)
      - Parsing TOC (DirTable + FileTable)
      - Extraction fichiers (décompression LZSS si nécessaire)
  [ ] Tests : round-trip avec archive DXA synthétique + vraie archive

Étape 3 — Extraction texte (2 semaines)
  [ ] `engines/wolf/extractor.rs` :
      - Parsing `.mps` via `wolfrpg-map-parser` → `ExtractedSegment`
      - Encodage : détecter version moteur → Shift-JIS (v2) ou UTF-8 (v3)
      - Portage parsing `.dat` depuis WolfTL (databases + CommonEvents)
      - Clés de type : `MapData/TitleMap/events/0/pages/0/5` (format Wolf Trans)
  [ ] Tests round-trip : extract → inject → binaires identiques

Étape 4 — Injection (1 semaine)
  [ ] `engines/wolf/injector.rs` :
      - Réécriture `.mps` et `.dat` avec texte traduit
      - Re-encodage UTF-8 → Shift-JIS si nécessaire (Wolf v2)
      - Option : laisser les fichiers déchiffrés (le jeu lit Data/ en priorité)
  [ ] Tests sur Mad Father freeware (Wolf v2)

Étape 5 — Intégration pipeline (0.5 semaine)
  [ ] `commands/project.rs` : `dispatch_extract` gère `Engine::Wolf`
  [ ] `commands/export.rs` : `export_project` gère Wolf
  [ ] QA Wolf : checks longueur ligne (Wolf a ses propres limites — configurer)
  [ ] Message "WolfX non supporté" si version ≥ 3.5 détectée
```

---

## Risques et caveats

**Licence**
- UberWolf : **pas de licence explicite confirmée dans les sources du rapport**. Non bundlable dans un produit commercial sans clarification. À résoudre avant F5. Alternative : forker WolfDec (311 étoiles, WolfDec n'a pas non plus de licence visible — à vérifier directement sur GitHub).
- WolfTL : MIT — pas de problème si portage ou référence.
- `wolfrpg-map-parser` : MIT — OK.
- `encoding_rs` : MIT/Apache-2.0 (Mozilla) — OK.

**Maintenance des dépendances**
- `wolfrpg-map-parser` (38 dl/mois) : faible adoption = risque d'abandon. Mitiger en ayant une implémentation de fallback (Wolf Trans format est documenté).
- UberWolf (actif en 2026) : stable pour F5 si nécessaire.

**Versions Wolf RPG — couverture réaliste**
- v1/v2/v3.0–3.31 : DXA XOR — couverture Rust native. Représente la majorité du catalogue DLsite 2010-2023.
- v3.5+/WolfX : ChaCha20 — sidecar Windows F5. Les jeux post-2024 sont la minorité cible en F4.
- Jeux sans clé (DXA_FLAG_NO_KEY) : clé constante `55AA2055550655AA55D57C66` — cas simple à gérer.
- v3.595+ : LinguaGacha lui-même dit "non supporté" — acceptable de refuser avec message d'erreur.

**Encodage Shift-JIS**
- Wolf v2 : Shift-JIS obligatoire (cp932). Le jeu CRASHE si du texte UTF-8 est injecté dans un `.dat` v2.
- Détection fiable de la version : via `Game.exe` PE FileVersion OU via le champ `CodePage` de l'en-tête DXA v6. Cette détection conditionne l'encodage de sortie.
- `encoding_rs` gère cp932 → UTF-8 et UTF-8 → cp932 de manière fiable.

**Texte ruby `\r[Base,Ruby]`**
- Le furigana (Ruby) est un guide de lecture pour les kanji — non traduisible vers EN/FR.
- Solution : tokeniser `\r[Base,Ruby]` entier comme placeholder. Afficher un warning QA spécifique ("segment contient du furigana — vérifier la traduction de Base").

**Format de clé d'extraction**
- MV/MZ utilise des JSON Pointer RFC 6901 (`/events/1/pages/0/list/5/parameters/0`).
- Wolf n'a pas de format standardisé — utiliser Wolf Trans convention : `MapData/NomCarte/events/N/pages/N/N` pour les maps et `Database/NomDB/N/N` pour les bases. Ces clés seront stockées dans `source_files.source_key` (DB existante — compatible).

---

## Jeux de test recommandés

| Jeu | Version Wolf | Chiffrement | Raison |
|-----|-------------|------------|-------|
| **Mad Father (freeware 2012)** | v2.x | DXA v5/v6 XOR | Le plus célèbre, traduit par vgperson, clé documentée, Shift-JIS |
| **One Way Heroics (Steam)** | v2.x | DXA v6, clé `nGui9('&1=@3#a` | Clé connue = test déchiffrement immédiat |
| **Misao (freeware)** | v2.x | DXA XOR | Classique horror Wolf, court, bon test texte |
| **LiEat (freeware)** | v2.x | DXA XOR | Série 3 épisodes courts, bonne couverture cases |
| **Jeu v3.0+ UTF-8** | v3.0–3.31 | DXA v8 XOR | Tester le basculement d'encodage UTF-8 |
| **Jeu v3.5+ WolfX** | v3.5+ | ChaCha20 | Vérifier que le message d'erreur s'affiche correctement |

**Source** : freeware disponibles sur RPGMaker.net, vgperson.com, ou itch.io. Ne pas utiliser des jeux DLsite payants pour les tests initiaux.
