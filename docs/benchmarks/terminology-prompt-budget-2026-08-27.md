# Budget de prompt terminologique

Date: 2026-08-27

## Contrat mesuré

Le benchmark automatisé `prompt_budget_holds_across_1191_fixture_segments`
construit 1 191 segments à partir du corpus japonais versionné, résout uniquement
les termes présents et mesure le fragment exact ajouté au prompt (instructions,
types, contraintes et variantes compris).

- segments: 1 191;
- surcoût médian en caractères: 4,22 %;
- surcoût maximal en caractères: 5,41 %;
- maximum observé: 2 hints par segment isolé;
- limite absolue du résolveur: 20 hints par appel.

Le résolveur ajoute progressivement les hints ordonnés et s'arrête avant que le
fragment sérialisé dépasse 10 % de l'estimation du prompt. Un terme qui ne tient
pas dans le budget est omis; aucun fallback global ne le remplace.

## Tokens réels — Ollama `gemma4:e4b`

Le test opt-in StandGirl a comparé le même terme et le même segment avec et sans
hint via l'endpoint OpenAI-compatible d'Ollama. Le modèle a renvoyé ses compteurs
exacts:

| Cible | Sans hint | Avec 1 hint | Surcoût |
|---|---:|---:|---:|
| anglais | 275 tokens prompt | 299 | 8,73 % |
| français | 275 tokens prompt | 299 | 8,73 % |

Le premier passage avec le format explicatif long atteignait 14,55 % en anglais
et 14,91 % en français. Le test a donc révélé que la borne en caractères ne
prédisait pas assez bien le tokenizer Gemma sur un segment très court. Le format
a été compacté en conservant source, cible, type sémantique, nature grammaticale,
enforcement et variantes. Le test réel impose désormais `≤ 10 %`.

Le nombre de hints reste enregistré avec chaque `ProviderCallMetrics`. Les
compteurs exacts dépendent toujours du modèle et ne sont disponibles que si le
fournisseur les renvoie; la mesure en caractères reste le garde-fou portable et
déterministe exécuté en CI.
