# Benchmark de l'analyseur terminologique japonais — 2026-08-27

## Décision

Le mode Lindera/IPADIC embarqué est retenu pour le premier socle MV/MZ. Il respecte largement les gates runtime, mémoire et taille compressée. Un dictionnaire externe mmap ajouterait de la complexité de packaging et de résolution de chemin sans bénéfice mesuré à ce stade.

La dépendance déclarée `lindera = "5.1"` a été résolue et verrouillée par Cargo en `5.3.0`. Lindera 6 n'est pas adopté pendant ce jalon afin de ne pas introduire une migration majeure publiée depuis seulement deux jours.

## Environnement

- OS: Linux 7.2.0-1-cachyos x86_64 GNU/Linux
- Rust: rustc 1.98.0 (`88d9e12ae`, LLVM 22.1.8)
- Lindera: 5.3.0
- Dictionnaire: `embedded://ipadic`
- Corpus: 9 cas linguistiques versionnés, répétés jusqu'à 1 191 segments
- SHA-256 du corpus: `aa92066dd160c31673eaede049150b374194be4ffc9c83f8384ca13a9ed44fc9`
- Runs chauds: 5

Le corpus couvre noms en kanji/katakana, verbes conjugués, adjectifs `い`/`な`, particules, auxiliaires, nombres, expressions et codes RPG Maker. Il mesure le coût morphologique de manière reproductible; le pilote StandGirl mesurera ensuite l'intégration complète sur les 1 191 segments réels.

## Résultats release

| Mesure | Résultat | Gate | Statut |
|---|---:|---:|---|
| Initialisation | 0,783 ms | informatif | OK |
| Passage froid, 1 191 segments | 10,389 ms | ≤ 5 000 ms | OK |
| Passage chaud médian | 10,095 ms | ≤ 5 000 ms | OK |
| Passage chaud p95 | 10,221 ms | ≤ 5 000 ms | OK |
| Tokens produits | 12 695 | stable entre runs | OK |
| RSS avant | 2 888 KiB | informatif | OK |
| RSS après init | 3 788 KiB | informatif | OK |
| RSS après scan froid/chaud | 21 524 KiB | informatif | OK |
| Hausse RSS totale | 18 636 KiB | ≤ 184 320 KiB | OK |
| Binaire benchmark non compressé | 48 788 128 octets | informatif | OK |
| Binaire benchmark gzip complet | 11 022 059 octets | ≤ 25 MiB d'ajout paquet | OK, borne supérieure |

Le binaire benchmark compressé complet reste inférieur au budget d'ajout du dictionnaire; la contribution compressée d'IPADIC est donc nécessairement inférieure à 11,1 Mo. Les données de build IPADIC occupent 58 Mo dans `target/`, mais elles ne représentent pas la taille du paquet distribué.

## Observations de build et d'exécution

- Le premier build de `lindera-ipadic` télécharge ses sources de dictionnaire. Le binaire produit contient ensuite IPADIC et fonctionne hors ligne au lancement.
- Le dictionnaire est chargé à la demande. Aucune tâche de fond ou boucle persistante n'est créée par Lindera; le CPU idle sera validé dans l'application Tauri finale.
- La RSS réelle doit être mesurée après le premier scan, car les pages du dictionnaire embarqué sont chargées paresseusement. La valeur retenue est donc 18 636 KiB, pas les 900 KiB observés juste après initialisation.
- Les mesures ne comprennent pas encore SQLite, les événements Tauri ni la construction des occurrences. Les gates d'intégration seront rejoués sur StandGirl.

## Licences

- Le crate Lindera 5.3.0 est sous licence MIT.
- Le crate `lindera-ipadic` annonce MIT et incorpore `mecab-ipadic-2.7.0-20070801` avec un avis NAIST/ICOT qui doit accompagner toute distribution substantiellement identique.
- Une copie de cet avis est versionnée dans `src-tauri/resources/licenses/lindera-ipadic-NOTICE.txt` et devra être incluse dans les bundles finaux.

## Commande reproductible

```bash
cargo bench --manifest-path src-tauri/Cargo.toml --bench terminology_analyzer
```

Le benchmark échoue si le passage froid ou le p95 chaud dépasse cinq secondes, si les comptes de tokens divergent ou si la hausse RSS Linux dépasse 180 MiB.
