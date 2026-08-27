# Validation de la bibliothèque terminologique — 27 août 2026

## Verdict

Le pipeline terminologique MV/MZ est opérationnel pour une source japonaise vers l'anglais ou le français. Le scan, la révision, les contraintes par segment, le QA, les packs v1, la suppression de projet et le packaging sont couverts. L'analyse réelle StandGirl respecte les budgets de temps, mémoire et inactivité.

La seule mesure conditionnelle non disponible sur cette machine est le nombre exact de tokens renvoyé par un modèle réel : aucun serveur Ollama ne répondait à `localhost:11434`. La borne portable de 10 % en caractères et la limite de 20 hints sont actives et testées; les métriques exactes seront renseignées automatiquement par tout fournisseur qui retourne son usage.

## Pilote StandGirl isolé

Source : `StandGirl-立ちんぼ六花の深い夜-`. Le test opt-in copie toute l'arborescence dans un `tempdir`, écrit uniquement dans la copie et une DB jetable, puis détruit cet espace.

| Mesure | Résultat | Gate | Statut |
|---|---:|---:|---|
| Segments extraits | 1 191 | 1 191 | OK |
| Scan froid debug | 1 792 ms | ≤ 5 000 ms | OK |
| Re-scan no-op | 52 ms | ≤ 1 000 ms | OK |
| Segments réanalysés au re-scan | 0 | 0 | OK |
| Entrées distinctes | 1 252 | informatif | OK |
| Occurrences | 3 898 | informatif | OK |
| DB après checkpoint | 3 301 376 octets | informatif | OK |
| RSS avant/après | 40 148 / 82 632 KiB | delta ≤ 180 MiB | OK |
| Delta RSS | 42 484 KiB | ≤ 184 320 KiB | OK |
| CPU après scan, fenêtre 500 ms | 0 tick | retour à l'inactivité | OK |
| Scan encore actif après fin | non | non | OK |
| Empreinte IDs/clés/textes avant/après | identique | identique | OK |
| Empreinte complète du jeu original avant/après | `2d60b1f3858c0e39c4a32c538b39cd8cf99830e1127e378951372c9bc04f4523` | identique | OK |

Natures : 696 noms, 67 noms propres, 255 verbes, 113 adjectifs, 88 adverbes et 33 expressions.

Types moteur : 1 087 généraux, 91 système, 20 objets, 15 lieux, 10 locuteurs, 9 compétences, 7 classes, 6 états, 4 personnages, 1 armure, 1 ennemi et 1 titre.

Le pilote de traduction acquis avant ce jalon reste la référence export : 1 191 textes traduits, 0 erreur critique finale, 205 avertissements de largeur et ZIP valide. Il n'a pas été retraduit artificiellement sans fournisseur local; le présent replay valide l'intégration de la nouvelle bibliothèque sur exactement les mêmes 1 191 sources.

## Parcours visible MCP Tauri

Application debug réelle Hoshi2Star 0.4.10, Tauri 2.11.2, Linux x86_64 :

- ouverture d'une copie MV/MZ ja→en;
- navigation Projet → Terminologie;
- scan explicite, 73 termes trouvés;
- filtres nom/verbe/adjectif vérifiés;
- recherche de `勇者アオイ`;
- cible `Hero Aoi` enregistrée en `locked + required`;
- retour au Patch, traduction du segment puis QA à 100;
- inspecteur du segment affichant le terme, `character`, `proper_noun`, `locked`, `required`;
- aucune erreur, alerte ou exception console;
- projet synthétique supprimé après le test.

La traduction de termes via modèle réel était indisponible car Ollama ne répondait pas. Le protocole fournisseur est couvert par l'E2E déterministe avec mock local; aucun texte n'est parti sur le réseau.

## Automatisation

- Rust : format et Clippy `-D warnings` propres; suite complète, 495 tests unitaires réussis et 4 ignorés, plus toutes les intégrations.
- E2E terminologique : MV/MZ ja→en et ja→fr, fournisseur mock uniquement, QA sans critique, JSON du ZIP relu, puis suppression projet validée.
- Frontend : lint sans erreur (10 avertissements historiques hors de cette fonctionnalité), typecheck réussi, 29 fichiers et 93 tests réussis, build réussi.
- UI : workspace terminologique chargé à la demande, chunk 25,60 kB (7,81 kB gzip). Le chunk principal reste à 735,57 kB et conserve l'avertissement Vite > 500 kB, dette indépendante à traiter par découpage supplémentaire.
- SQLite : test 10 000 entrées paginé, 40 mesures, p95 observé 16,34 ms pour un gate à 100 ms.
- Prompt : médiane 9,29 %, maximum 9,89 %, 2 hints maximum observés sur le corpus de 1 191 segments, borne absolue 20.

## Packaging et licence

Le paquet Debian debug a été généré et contient :

`usr/lib/hoshi2star/resources/licenses/lindera-ipadic-NOTICE.txt`

Le binaire et le paquet ont été produits. La commande Tauri retourne ensuite un code non nul parce que la clé privée de signature updater n'est pas présente dans l'environnement local; cette clé de publication ne doit pas être versionnée. La structure du bundle et la licence sont néanmoins validées.

## Commandes de reproduction

```bash
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
pnpm lint
pnpm typecheck
pnpm test
pnpm build
cargo test --manifest-path src-tauri/Cargo.toml --test terminology_e2e
H2S_STANDGIRL_PATH=/chemin/vers/StandGirl \
  cargo test --manifest-path src-tauri/Cargo.toml \
  --test terminology_real_pilot -- --ignored --nocapture
```
