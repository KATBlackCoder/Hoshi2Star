# Architecture de la terminologie

## Principe directeur

Le contexte terminologique n'est pas universel. Lindera décrit la morphologie japonaise; chaque adaptateur moteur décide quels segments sont des acteurs, objets, locuteurs, lieux ou textes généraux. Le cœur terminologique ne contient donc aucune règle MV/MZ implicite.

```text
extracteur moteur -> segments SQLite
                         |
scan explicite ----------+
  adaptateur moteur + Lindera/IPADIC
                         |
entries globales <- occurrences projet -> traductions globales/projet
                         |
resolver unique ----------+----------+
                         |          |
                   requête LLM      QA
```

## Modules et responsabilités

| Couche | Modules | Responsabilité |
|---|---|---|
| Moteur | `engines/terminology.rs`, `engines/mv_mz/terminology.rs` | Transformer le contexte propre au moteur en graines sémantiques stables. |
| Analyse | `core/terminology/analyzer/` | Protéger les codes, appeler Lindera, mapper IPADIC, filtrer les candidats. |
| Scan | `scanner.rs`, `service.rs` | Scan incrémental par pages de 250, worker bloquant borné, annulation, événements compacts et un scan concurrent maximum. |
| Données | `types.rs`, `normalize.rs`, `repository.rs` | Contrats, normalisation centralisée, requêtes SQLite paginées et transactions. |
| Traduction | `translator.rs` | Protocole JSON strict par ID, validation de la réponse et persistance en proposition. |
| Résolution | `resolver.rs` | Portée projet > globale, pertinence par occurrence, ordre des contraintes et budget de hints. |
| API | `commands/terminology.rs` | Validation IPC et délégation; aucune logique SQL dupliquée. |
| UI | `features/terminology/` | TanStack Query pour l'état serveur, petit store Zustand pour les filtres, table et dialogues spécialisés. |
| QA | `core/qa.rs`, `core/report.rs`, `TerminologyInspector.tsx` | Règles de cohérence à partir du resolver commun; inspecteur en lecture. |
| Packs | `commands/pack_terminology.rs` | Adaptation isolée vers/depuis le champ glossaire du format `.h2s` v1. |

Cette séparation garde les fichiers focalisés et évite trois duplications dangereuses : un second resolver dans le QA, des requêtes SQLite dans les commandes et une logique moteur générique dans le cœur.

## Modèle SQLite

La migration `0008_terminology_library.sql` ajoute :

- `terminology_entries` : lemme source global, nature, type sémantique, sens, origine et statut;
- `terminology_translations` : cible, langue, portée projet nullable, révision, enforcement, provenance modèle et variantes;
- `terminology_occurrences` : relation vers projet/segment, forme de surface, type moteur et fréquence;
- `terminology_scans` : audit des scans, version analyseur, progression et erreur;
- `terminology_segment_state` : empreinte incrémentale par segment/version d'analyseur.

Les clés étrangères suppriment en cascade les occurrences et traductions de projet. Une entrée globale n'est pas supprimée avec le projet. L'ancienne table `glossary_terms` reste uniquement comme donnée historique de rollback; aucun code applicatif actif ne la lit.

Les listes sont filtrées, triées et paginées côté serveur, avec une page maximale bornée. Le test 10 000 entrées mesure 40 requêtes et impose un p95 inférieur à 100 ms sans charger la base entière dans React.

## Incrémentalité et ressources

Le hash d'un segment inclut son texte, son contexte moteur et les versions de l'analyseur/adaptateur. Un re-scan inchangé lit les pages mais n'appelle pas Lindera et ne réécrit pas les occurrences. L'analyse CPU est exécutée dans `spawn_blocking`; SQLite reçoit des transactions par chunk; les scans et traductions terminologiques ont chacun un sémaphore borné.

IPADIC est embarqué et initialisé une fois dans `AppState`. Il n'existe ni thread d'analyse permanent ni polling en arrière-plan. Le scan n'est lancé qu'après une action utilisateur. Les événements Tauri contiennent seulement progression, compte découvert, statut et erreur; ils ne transportent pas la liste complète des termes.

## Intégration au pipeline LLM

`TranslationContext` porte des `TerminologyHint` riches et non un tuple de glossaire. Pour chaque sous-lot créé, le pipeline demande au resolver les termes ayant une occurrence dans les IDs concernés. Le resolver :

1. applique la traduction de projet avant la globale;
2. exclut les cibles vides et non pertinentes;
3. ordonne `required`, `preferred`, `contextual`, puis la fréquence;
4. déduplique;
5. ajoute au plus 20 hints et s'arrête avant 10 % du budget de prompt estimé.

Le prompt explique au modèle qu'un hint ne doit jamais être inséré si le terme source n'apparaît pas. Le fournisseur reste configurable; le cœur n'a pas de prompt différent par marque, mais applique une politique adaptée aux capacités exposées par le modèle.

## QA, compatibilité et suppression

Le QA reçoit les mêmes IDs de segments et résout les mêmes termes. Les règles tiennent compte de la révision, de l'enforcement, du type sémantique et des variantes. La preview est pure; seul l'audit explicite persiste les résultats.

Le pack v1 garde `glossary.json` pour la compatibilité externe. `pack_terminology.rs` est un anti-corruption layer : il n'exporte que les valeurs validées et protège les valeurs verrouillées à l'import. La compatibilité IPC Wolf `extract_wolf_speakers` alimente désormais les mêmes tables.

## Extension à un nouveau moteur ou analyseur

Un nouveau moteur doit implémenter le contrat de `engines/terminology.rs`, fournir une table de vérité sur ses types de segments et des fixtures. Il ne doit pas copier l'adaptateur MV/MZ dans le core. Un nouvel analyseur de langue doit implémenter `MorphologicalAnalyzer`, ajouter sa normalisation, ses filtres, un corpus linguistique, un benchmark de RAM/CPU/paquet et rester désactivé tant que ces gates n'ont pas passé.

## Vérification

Les niveaux sont : tests unitaires des contrats/migrations/repository/analyseur/resolver/QA, intégrations Tauri mockées, E2E synthétique ja→en et ja→fr, parcours visible via MCP Tauri, puis pilote privé sur copie temporaire. Les résultats du jalon sont consignés dans [la validation du 27 août 2026](../validation/terminology-2026-08-27.md).
