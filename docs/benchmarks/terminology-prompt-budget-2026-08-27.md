# Budget de prompt terminologique

Date: 2026-08-27

## Contrat mesuré

Le benchmark automatisé `prompt_budget_holds_across_1191_fixture_segments`
construit 1 191 segments à partir du corpus japonais versionné, résout uniquement
les termes présents et mesure le fragment exact ajouté au prompt (instructions,
types, contraintes et variantes compris).

- segments: 1 191;
- surcoût médian en caractères: 9,29 %;
- surcoût maximal en caractères: 9,89 %;
- maximum observé: 2 hints par segment isolé;
- limite absolue du résolveur: 20 hints par appel.

Le résolveur ajoute progressivement les hints ordonnés et s'arrête avant que le
fragment sérialisé dépasse 10 % de l'estimation du prompt. Un terme qui ne tient
pas dans le budget est omis; aucun fallback global ne le remplace.

## Tokens réels

Le nombre de hints est maintenant enregistré avec chaque `ProviderCallMetrics`.
Les nombres de tokens exacts restent ceux renvoyés par le fournisseur, car la
tokenisation varie selon le modèle. Le gate exact en tokens sera donc rejoué sur
le fournisseur du pilote StandGirl; cette mesure en caractères est le garde-fou
portable et déterministe exécuté en CI.
