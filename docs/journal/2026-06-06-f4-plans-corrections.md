# Analyse corrections plans F4 — 2026-06-06

Session de relecture des plans F4 Wolf RPG avant implémentation.
Mode : ANALYSE + CORRECTION DE PLANS UNIQUEMENT — aucun code applicatif écrit.

Sources lues dans l'ordre prescrit :
1. `CONTEXT.md`
2. `docs/plans/f4-01-wolf-foundations.md` (Step 6 — regex RE_WOLF)
3. `docs/plans/f4-02-wolf-decryptor.md` (Step 3 — table de clés)
4. `docs/plans/f4-04-wolf-injector.md` (Step 2 — offsets injector)
5. `src-tauri/src/llm/tokenizer.rs` (regex MV/MZ de référence)
6. `docs/wolf-rpg-research.md` (section §4 placeholders + §3 clés Wolf)
7. `docs/plans/wolf-rpg-approach.md` (section placeholders)

---

## Correction 1 — Regex RE_WOLF (f4-01 Step 6)

### Analyse

**Problème signalé (duplication `\f[n]`) :** réel mais non-bloquant.
- `\\[vcsfiVCSFI]\[\d+\]` couvre `v,c,s,f,i,V,C,S,F,I` — inclut `f`
- `\\[fm]\[\d+\]` couvre aussi `f` → duplication
- **`\m[n]` est bien capturé** par `\\[fm]\[\d+\]` — la fausse alarme du signalement
  sur `\m[n]` "manquant" est infondée. L'alternation regex prend le premier match :
  `\f[n]` est pris par la première branche, `\m[n]` tombe dans la seconde. Correct.

**Vrai problème découvert en lisant `wolf-rpg-research.md §4` :**
Les codes de contrôle d'affichage Wolf suivants sont documentés dans le rapport mais
**absents de RE_WOLF** :

| Code | Signification | Présent dans RE_WOLF ? |
|------|--------------|------------------------|
| `\!` | pause jusqu'à appui touche | ❌ MANQUANT |
| `\.` | attend 0,25 s | ❌ MANQUANT |
| `\^` | fin forcée sans input | ❌ MANQUANT |
| `\>` | affichage instantané | ❌ MANQUANT |
| `\<` | stop affichage instantané | ❌ MANQUANT |
| `\E` | activer contours | ✅ dans `\\[EN\\]` |
| `\N` | retirer contours | ✅ dans `\\[EN\\]` |
| `\\` | backslash littéral | ✅ dans `\\[EN\\]` |

Ces codes existent dans MV/MZ Groupe B (`\\[G\\$.|!><^{}]`) mais RE_WOLF est une
regex **standalone** qui ne réutilise pas RE_MVMZ. L'omission vient du document
`wolf-rpg-approach.md` qui liste `\E \N \A+ \A-` comme "effets texte" sans inclure
les codes de contrôle d'affichage — tous présents dans le rapport mais oubliés lors
de la rédaction de la liste "nouveaux patterns".

**Impact si non corrigé :** le LLM voit `\!` comme un `!` littéral et risque de le
supprimer ou déplacer → corruption du timing des dialogues Wolf.

**Comparaison avec RE_MVMZ existant** (tokenizer.rs ligne 37) :
```rust
\\[G\\$.|!><^{}]  # Groupe B — codes sans argument
```
RE_MVMZ couvre ces codes. RE_WOLF doit les couvrir aussi en tant que regex standalone.

### Décision : **Corrigé**

### Justification

Problème réel confirmé par `wolf-rpg-research.md §4`. L'absence de ces 5 codes
produirait une corruption silencieuse dans les exports Wolf v2/v3 — exactement le
type de bug indétectable avant les tests e2e.

### Changements apportés au plan

**f4-01 Step 6 — Regex RE_WOLF :**
1. `\\[fm]\[\d+\]` → `\\m\[\d+\]` (supprime le `f` redondant, la branche couvre maintenant uniquement `\m[n]`)
2. `\\[EN\\]` → `\\[EN\\!.^><]` (ajoute `\!`, `\.`, `\^`, `\>`, `\<`)
3. Règle de priorité #10 mise à jour pour lister les 8 codes no-arg

---

## Correction 2 — Clés v3.10/v3.173 longueur (f4-02 Step 3)

### Analyse

Le rapport `docs/wolf-rpg-research.md §3` mentionne explicitement :
> "Wolf v3.10 et v3.173 : clés plus longues (40–46 octets) listées dans WolfDec."

Cette information est disponible — le ⚠️ "À INVESTIGUER" n'est donc pas un inconnu
total, mais une **ambiguïté documentée** :

**Deux interprétations possibles de "40-46 octets" :**
1. Longueur du mot de passe **brut** avant `keyCreate()` → la clé XOR résultante
   reste 12 octets (`DXA_KEYSTR_LENGTH = 12`) → `[u8; 12]` correct
2. Longueur de la **clé XOR elle-même** → `DXA_KEYSTR_LENGTH` aurait changé pour
   v3.x → `[u8; 12]` insuffisant, nécessiterait `Vec<u8>` ou `enum WolfKey`

**Ce que le rapport ne dit pas :** il ne précise pas laquelle des deux interprétations
est correcte. La constante `DXA_KEYSTR_LENGTH = 12` est cependant documentée comme
fondamentale dans le code DxLib (§3 : "XOR répété à clé 12 octets. Constante
`DXA_KEYSTR_LENGTH = 12`"). Si cette constante avait changé pour v3.x, ce serait
une modification majeure du format DXA — non mentionnée dans le rapport.

**Portée F4 :** les versions v3.10/v3.173 sont **hors scope F4** (F4 cible
v1/v2/v3.0–3.31 via DXA XOR). Le ⚠️ est donc une précaution pour F5, pas un
blocant pour F4.

**Impact si non corrigé en F4 :** aucun — les jeux de test disponibles (月咲流ホノカ
v2.255, Densyanai Inko v2.x) n'utilisent pas les clés v3.10. Le risque se matérialise
seulement si F5 implémente v3.10 sans avoir résolu l'ambiguïté.

### Décision : **⚠️ enrichi mais non résolu — correct**

Le `⚠️ À INVESTIGUER` est la bonne approche puisque l'ambiguïté est réelle. La
correction consiste à :
1. Enrichir le commentaire inline avec ce que le rapport dit (pas laisser croire
   que c'est un inconnu total)
2. Ajouter une tâche explicite obligatoire "vérifier WolfDec AVANT de coder v3.x"
3. Clarifier que la portée F4 n'est pas bloquée

### Justification

Spéculer sur l'interprétation (1) ou (2) sans lire le code WolfDec serait une erreur.
Le rapport contient l'information que "40-46 octets" existent, mais pas leur sémantique
exacte. La tâche d'investigation est la seule décision correcte.

### Changements apportés au plan

**f4-02 Step 3 — WOLF_KEYS :**
- Commentaire v3.10 enrichi : mentionne l'ambiguïté raw password vs clé XOR,
  clarifie que DXA_KEYSTR_LENGTH=12 est constant dans le rapport, et que c'est hors scope F4
- Tâche explicite ajoutée : "Vérifier WolfDec main.cpp AVANT de coder v3.x"

---

## Correction 3 — Offsets injector (f4-04 Step 1 + Step 2)

### Analyse

La question est : `wolfrpg-map-parser` expose-t-elle des offsets binaires dans son API ?

**Ce que les sources disponibles disent :**
- `docs/wolf-rpg-research.md §5` : `Map::parse(&bytes)` → "arbre de structs Rust
  + sortie JSON". Aucune mention d'offsets.
- `docs/plans/wolf-rpg-approach.md §Option C` : "La crate `wolfrpg-map-parser` v0.6.0 offre :
  `Map::parse(&bytes)` → structs Rust + sortie JSON pour les fichiers `.mps`".
  Aucune mention d'offsets.

**Information non disponible dans les sources actuelles :**
Les deux sources décrivent l'API par son contenu sémantique (structs des données du
jeu) et non par ses offsets binaires. Ce style de description est caractéristique
d'un parser qui retourne les données parsées, **pas** les positions brutes.

**Évaluation de la probabilité :**
- Parsers de type "structs Rust + JSON" ne retournent généralement pas les offsets
  binaires — les offsets sont des détails d'implémentation internes
- La crate n'est pas spécifiquement conçue pour l'injection (elle parse, elle
  n'écrit pas)
- À 38 dl/mois, la crate est peu mature — les offsets sont rarement exposés
  dans un parser de jeu à ce stade de maturité

**Impact si l'Approche A est planifiée comme défaut et que les offsets n'existent pas :**
F4-04 Step 2 tel que planifié commence par modifier `ExtractedSegment` pour ajouter
`byte_offset: u64`. Si les offsets ne sont pas exposés par la crate, cette tâche
est bloquée et réoriente vers l'Approche B — **perte de temps sans investigation préalable**.

### Décision : **Corrigé**

L'investigation sur l'API doit être une tâche obligatoire **avant** Step 2 (pas
enterrée dans les tâches de Step 2). L'Approche B doit être le défaut documenté
(plus robuste, indépendante de la crate), l'Approche A devenant conditionnelle.

### Justification

Les deux sources disponibles indiquent fortement que les offsets ne sont pas exposés.
Planifier Approche A comme défaut sans avoir vérifié risque de bloquer l'implémentation
dès Step 2. L'investigation à 15 min (lire crates.io + src) protège contre un
re-planning en cours d'exécution.

### Changements apportés au plan

**f4-04 Step 1 :**
- Tâche préalable obligatoire ajoutée en tête : "Investiguer l'API wolfrpg-map-parser
  AVANT Step 2 — confirmer si les structs retournées exposent des offsets binaires"

**f4-04 Step 2 :**
- Recommandation inversée : Approche B par défaut, Approche A seulement si
  investigation confirme les offsets
- Tâches pour `byte_offset` marquées conditionnelles "(Approche A seulement)"
- Tâche Approche B ajoutée : `locate_string_in_mps(bytes, key) -> Option<(u64, usize)>`

---

## Résumé

- **Corrections appliquées : 3/3**
- **Plans modifiés :**
  - `docs/plans/f4-01-wolf-foundations.md` (Step 6 — regex corrigée)
  - `docs/plans/f4-02-wolf-decryptor.md` (Step 3 — note v3.x enrichie + tâche investigation)
  - `docs/plans/f4-04-wolf-injector.md` (Step 1 — investigation préalable + Step 2 — Approche B défaut)

- **Points restants à investiguer pendant l'exécution :**
  1. **Début F4-04 Step 1** : lire crates.io/wolfrpg-map-parser + structs publiques
     → décide entre Approche A (offsets exposés) et Approche B (re-parsing)
  2. **Avant F5 support v3.10** : lire WolfDec `main.cpp` DECRYPT_MODES
     → vérifie si les "40-46 octets" sont raw password ou clé XOR
     → décide si `[u8; 12]` ou `Vec<u8>` pour la table de clés v3.x
