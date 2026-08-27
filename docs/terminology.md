# Bibliothèque terminologique

La bibliothèque terminologique construit un vocabulaire réutilisable à partir des textes extraits d'un jeu. Elle est distincte de la mémoire de traduction : la TM mémorise des segments complets, tandis que la bibliothèque décrit des termes, leur nature, leur contexte, leurs traductions et leur niveau de contrainte.

## Parcours recommandé

1. Ouvrir le projet et choisir la langue source et la langue cible. Le premier socle analyse uniquement une source japonaise (`ja`), avec une cible notamment anglaise (`en`) ou française (`fr`).
2. Laisser l'extracteur du moteur créer les segments. Pour MV/MZ, l'adaptateur identifie les acteurs, objets, armes, armures, compétences, états, classes, ennemis, lieux, titres, locuteurs, textes système et textes généraux.
3. Ouvrir **Terminologie**, conserver la portée **Projet**, puis choisir **Analyser le vocabulaire**. Le scan est explicite : ouvrir un jeu ne déclenche ni analyse morphologique ni appel réseau.
4. Filtrer par recherche, nature grammaticale ou type moteur. Vérifier les faux positifs avant de traduire.
5. Sélectionner des termes, choisir la langue cible et lancer leur traduction avec le fournisseur configuré. Les propositions du modèle restent `proposed`; elles ne deviennent jamais obligatoires automatiquement.
6. Corriger les cibles puis choisir leur état de révision et leur contrainte.
7. Traduire les segments du jeu. Seuls les termes présents dans le sous-lot courant sont ajoutés au prompt.
8. Examiner le QA, corriger les incohérences, puis exporter le patch ou le ZIP.

## Analyse locale et catégories

Lindera/IPADIC découpe localement le japonais et fournit le lemme, la lecture, la nature grammaticale et la conjugaison. Les codes RPG Maker sont protégés avant l'analyse. Aucun fournisseur LLM n'est contacté pendant le scan.

Les natures stables sont : nom, nom propre, verbe, adjectif, adverbe et expression. Les particules, auxiliaires isolés, ponctuations, nombres sans valeur terminologique et tokens trop faibles sont filtrés. Les verbes et adjectifs sont stockés sous une forme canonique mais leurs formes fléchies restent visibles via les occurrences.

La nature grammaticale vient de Lindera; le type sémantique vient de l'adaptateur moteur. Il ne faut pas confondre les deux. Par exemple, `勇者アオイ` peut être un `proper_noun` et un `character`.

## Portée, révision et contrainte

Les entrées sources forment une base personnelle globale. Leurs occurrences relient chaque terme aux segments des projets. Une traduction peut être globale ou surchargée pour un projet précis.

| État ou contrainte | Effet |
|---|---|
| `proposed` | Proposition du modèle à réviser; jamais bloquante. |
| `approved` | Traduction validée par l'utilisateur. |
| `locked` | Traduction protégée contre un remplacement silencieux. |
| `contextual` | Aide de sens, sans obligation de reprise littérale. |
| `preferred` | Formulation recommandée pour la cohérence. |
| `required` | Forme exigée lorsque le sens et l'occurrence correspondent. À réserver aux entités stables. |

Pour les verbes et adjectifs, une cible `required` ne produit pas d'erreur exacte critique sans variante acceptée explicite. Cela évite d'imposer une forme non conjuguée dans tous les contextes.

## Utilisation par les modèles

Le même protocole fonctionne avec les fournisseurs OpenAI-compatibles configurés dans Hoshi2Star, dont Ollama, LM Studio et les endpoints cloud compatibles. La sélection du fournisseur, de l'URL, du modèle et de la clé reste explicite.

Le résolveur s'exécute après le regroupement réel des segments. Il ne charge que les termes ayant une occurrence dans le sous-lot, applique la surcharge projet avant la valeur globale, trie les contraintes fortes en premier, déduplique et limite chaque requête à 20 hints et à 10 % du budget de prompt estimé. Il n'existe aucun fallback qui enverrait des termes sans rapport avec le texte.

Les compteurs exacts de tokens sont enregistrés lorsqu'un fournisseur les renvoie. Comme chaque modèle utilise son propre tokenizer, l'application applique aussi une borne portable en caractères. Un fournisseur qui ne renvoie pas d'usage ne permet pas de certifier après coup un nombre exact de tokens.

## QA et export

Le QA utilise le même résolveur que la traduction : il n'a pas un deuxième glossaire caché. Un terme `locked + required` absent de la cible peut produire une erreur critique; un terme `approved + preferred` incohérent produit un avertissement; une proposition ou une aide contextuelle reste informative. L'inspecteur du segment permet de voir les règles réellement applicables.

La prévisualisation QA est en lecture seule. L'audit explicite avant export peut persister les résultats. Les codes, placeholders et IDs de segments sont validés indépendamment de la terminologie.

Les packs `.h2s` restent au format v1. À l'export, seules les traductions `approved` ou `locked` sont converties vers le champ historique `glossary.json`. À l'import, elles deviennent des termes de projet `approved/preferred`. Une traduction verrouillée n'est jamais écrasée silencieusement. Les sauvegardes automatiques existantes de la base SQLite restent la protection de restauration; la bibliothèque n'introduit pas une deuxième copie de jeu.

## Suppression d'un jeu

Supprimer un projet de Hoshi2Star retire ses fichiers sources, segments, occurrences et traductions propres au projet. Les entrées sources globales et leurs traductions globales restent disponibles pour les futurs jeux. Les termes orphelins ne sont pas supprimés automatiquement : une future commande d'entretien devra être explicite et réversible.

## Confidentialité et limites

- Le scan morphologique est local et hors ligne après installation.
- Seuls les termes sélectionnés sont envoyés lors de leur traduction; seuls les hints pertinents sont envoyés avec un lot de segments.
- Les clés API restent dans les réglages locaux et ne doivent jamais être placées dans un pack ou un rapport.
- IPADIC peut mal découper l'argot, les néologismes, les noms inventés, les lectures ambiguës et le texte volontairement fragmenté. L'adaptateur moteur améliore les entités structurées, mais ne remplace pas une révision humaine.
- La source japonaise est la seule analyse morphologique activée dans cette livraison. L'architecture accepte d'autres analyseurs, mais ils doivent être validés séparément.
- Une bibliothèque améliore la cohérence; elle ne garantit pas seule la fidélité sémantique, le ton, le genre du locuteur ou la qualité littéraire.

Lindera est distribué sous licence MIT. L'avis de `mecab-ipadic-2.7.0-20070801` est inclus dans les bundles sous `resources/licenses/lindera-ipadic-NOTICE.txt`.

## Validation reproductible

```bash
cargo test --manifest-path src-tauri/Cargo.toml --test terminology_e2e
H2S_STANDGIRL_PATH=/chemin/vers/le/jeu \
  cargo test --manifest-path src-tauri/Cargo.toml \
  --test terminology_real_pilot -- --ignored --nocapture
```

Le second test copie d'abord le jeu dans un dossier temporaire, utilise une base SQLite jetable et laisse l'original intact.
