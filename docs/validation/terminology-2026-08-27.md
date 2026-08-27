# Validation de la bibliothèque terminologique — 27 août 2026

## Verdict

Le pipeline terminologique MV/MZ est opérationnel pour une source japonaise vers l'anglais ou le français. Le scan, la révision, les contraintes par segment, le QA, les packs v1, la suppression de projet et le packaging sont couverts. L'analyse réelle StandGirl respecte les budgets de temps, mémoire et inactivité. Le budget exact de tokens a également été validé avec Ollama `gemma4:e4b`.

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

Ollama ne répondait pas pendant cette première session MCP. Le parcours fournisseur a ensuite été rejoué localement avec `gemma4:e4b`, comme détaillé ci-dessous. Aucun texte n'a été envoyé hors de la machine.

## Validation Ollama réelle

Modèles détectés localement : `gemma4:e4b` (8B, Q4_K_M) et `gemma4:e2b` (5,1B, Q4_K_M). Le test utilise le modèle par défaut Hoshi2Star `gemma4:e4b`, une copie temporaire de StandGirl et une DB jetable.

Trois termes structurés ont été traduits en anglais puis en français. Chaque résultat a été persisté en `proposed`, puis un seul terme a été explicitement verrouillé pour tester le pipeline. Exemples du passage final : `六花 → Rikka`, `凛 → Rin`, `汎用 → General/Générique`.

Les passages successifs ont également produit des variantes comme `Rikaka` et `Rokka` pour `六花`. Cette variabilité confirme qu'une sortie modèle ne doit jamais devenir `approved` ou `locked` automatiquement.

| Mesure | ja→en | ja→fr | Gate |
|---|---:|---:|---:|
| Traduction de 3 termes, tokens prompt | 434 | 434 | informatif |
| Traduction de 3 termes, tokens réponse | 141 | 143 | informatif |
| Segment témoin sans hint | 275 | 275 | référence |
| Même segment avec 1 hint | 299 | 299 | ≤ 302,5 |
| Surcoût prompt exact | 8,73 % | 8,73 % | ≤ 10 % — OK |
| QA avec terme verrouillé | 100, 0 critique | 100, 0 critique | OK |
| Appels et retries | 1 tentative/appel | 1 tentative/appel | OK |

Le premier format de hint, trop verbeux, mesurait 14,55 % et 14,91 %. Il a été compacté sans retirer la nature grammaticale, le type sémantique, l'enforcement ni les variantes. Le test opt-in échoue désormais si le surcoût exact dépasse 10 %.

## Automatisation

- Rust : format et Clippy `-D warnings` propres; suite complète, 496 tests unitaires réussis et 4 ignorés, plus toutes les intégrations. Les deux pilotes privés ignorés par défaut passent lorsqu'ils sont activés explicitement.
- E2E terminologique : MV/MZ ja→en et ja→fr, fournisseur mock uniquement, QA sans critique, JSON du ZIP relu, puis suppression projet validée.
- Frontend : lint sans erreur (10 avertissements historiques hors de cette fonctionnalité), typecheck réussi, 29 fichiers et 93 tests réussis, build réussi.
- UI : workspace terminologique chargé à la demande, chunk 25,60 kB (7,81 kB gzip). Le chunk principal reste à 735,57 kB et conserve l'avertissement Vite > 500 kB, dette indépendante à traiter par découpage supplémentaire.
- SQLite : test 10 000 entrées paginé, 40 mesures, p95 observé 16,34 ms pour un gate à 100 ms.
- Prompt : médiane 4,22 %, maximum 5,41 % en caractères; 8,73 % en tokens réels Gemma; 2 hints maximum observés sur le corpus de 1 191 segments, borne absolue 20.

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
  --test terminology_real_pilot \
  standgirl_scan_meets_real_project_gates_on_a_disposable_copy \
  -- --ignored --nocapture

H2S_STANDGIRL_PATH=/chemin/vers/StandGirl \
H2S_OLLAMA_URL=http://127.0.0.1:11434/v1 \
H2S_OLLAMA_MODEL=gemma4:e4b \
  cargo test --manifest-path src-tauri/Cargo.toml \
  --test terminology_real_pilot \
  standgirl_ollama_translates_terms_and_reports_exact_prompt_tokens \
  -- --ignored --nocapture
```
