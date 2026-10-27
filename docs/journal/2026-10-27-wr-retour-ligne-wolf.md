# Journal — 2026-10-27 — WR : retour à la ligne des textes Wolf à l'export (conception)

**Phase** : F4
**Durée estimée** : 1h
**Statut** : 🔄 En cours (plan à valider)
**Outil** : Claude
**Branche et commit de départ** : `main` @ `2331c75` (lecture du code sur `feat/rw-wolf` @ `9d28ae2`, worktree `../Hoshi2Star-rw`)
**Objectif / plan / story** : concevoir le retour à la ligne automatique des textes Wolf à l'export ; plan dans `tasks/todo.md`, « Plan — WR »

---

## Ce qui a été fait

- Reprise faite (handoff, leçons, état Git). RW n'est **pas fusionné** et n'a pas de PR (`gh pr list --state all` s'arrête à #12).
- Lecture sur `feat/rw-wolf` :
  - text fitting MZ (`core/text_fit.rs`, `engines/mv_mz/layout/*`) ;
  - `apply_font_prefix` et `collect_wolf_zip_entries` (`commands/export.rs`) ;
  - `engines/wolf/placeholders.rs` et l'extracteur Wolf ;
  - les verrous `mv_mz` (`qa.rs:133`, `report.rs:116`, `set_text_fit`).
- Format de `Game.dat` lu dans WolfTL (`WolfTL/WolfRPG/GameDat.hpp`, MIT). `Game.dat` 2.23 est en clair :
  police principale `M+ 2c medium`, réglages numériques dans `m_unknown1` (22 octets), non décodés.
- Mesures en lecture seule (Python `-I`, scripts dans le scratchpad, outils `RAE-PLAN/wolf/_outils/wolf_v2.py`)
  sur les 18 cartes et le `CommonEvent.dat` extraits du 2.23 :
  - messages avec visage : 3 lignes au plus, largeur p99 45 colonnes demi-chasse, max 52 ;
  - messages sans visage : largeur p99 48 ;
  - choix : p99 20.
- Mesure avec la vraie police (`mplus-2c-medium.ttf`, fontTools) :
  - « ※Thank you for downloading th » vaut 276 px à 18 px ;
  - les lignes d'origine de la même fenêtre valent 320 à 336 px à 16 px ;
  - les deux ne collent pas ⇒ calibrage en jeu obligatoire.
- Cibles du projet de test `APP`, lues dans la base de l'app en lecture seule : les `\n` parasites sont rejetés en fin de phrase par le LLM.
- Plan WR écrit dans `tasks/todo.md` ; relu par un conseiller avant écriture.

## Fichiers créés

- `docs/journal/2026-10-27-wr-retour-ligne-wolf.md`

## Fichiers modifiés

- `tasks/todo.md` — section « Plan — WR » (avant « Études REA »).
- `tasks/handoff.md` — section WR.

## Décisions prises

- Aucune décision produit : WR-E0 à WR-E6 attendent l'utilisateur.

## Problèmes rencontrés

- `tasks/todo.md` et `tasks/handoff.md` sont modifiés en parallèle par d'autres sessions (EX, Bakin), sans commit.
  Ajouts faits par insertion ciblée ; aucune de leurs lignes n'a été touchée.

## Corrections et erreurs à ne plus répéter

- Aucune.

## Vérifications et limites

- Aucun build ni test : plan seulement. Le contrôle a porté sur la cohérence des références `fichier:ligne`
  (relues sur `feat/rw-wolf`).
- Hypothèse non vérifiée : la police du jeu est bien chargée sous Wine depuis le dossier du jeu.

## Suite — plan validé, WR-1 préparé (2026-10-27)

- Plan validé par l'utilisateur avec les recommandations WR-E0 à WR-E6.
- Outil `RAE-PLAN/wolf/_outils/build_wolf_wcal.py` (hors Git), qui réutilise `build_wolf_tests.py`, `wolf_v2.py`
  et `dxa6.py`. Variante `RAE-PLAN/essais/wolf-v223/WCAL` construite depuis `W0` :
  - 16 textes de `SYS＿タイトル.mps` remplacés par des règles graduées (écrans A à M) ;
  - relecture conforme : seules les chaînes prévues changent, structure identique octet pour octet ;
  - archive reconstruite (204 388 312 octets), 1 373 fichiers relus conformes, DXA 6, clé et page de codes inchangées.
- Fiche d'essai `RAE-PLAN/essais/wolf-v223/WCAL.FICHE.md` ; script de capture `capture.fish` (spectacle `-a`).
- Pas encore lancé en jeu.

## Essai WCAL partiel et report (2026-10-27)

- L'utilisateur a lancé WCAL sous Wine.
  - Le terminal ne montre que du bruit sans effet sur le jeu : `dmsynth` `Underrun` (son), puis un blocage de
    `loader_section` à la fermeture.
- Capture A (menu titre), copiée dans `RAE-PLAN/essais/wolf-v223/WCAL-captures/A.png` et mesurée en Python (PIL) :
  - écran de 640 × 480, sans mise à l'échelle ;
  - choix `<R>` : le texte part de x ≈ 233 et sort de l'écran à 639 (26 chiffres visibles sur 40) ;
  - pas de 16,0 px par chiffre, identique avec `\f[12]` ⇒ `\f` n'agit pas sur un choix ;
  - le 3e choix (`ゲーム終了`, d'origine) n'est pas visible, d'après l'utilisateur. Cause non établie ;
    à comparer avec `W0`.
- Le lot est **reporté** par décision de l'utilisateur. Écrans B à M non capturés.

## Reprise et WR-1 terminé (2026-10-27)

- Reprise demandée par l'utilisateur après la fusion de RW (PR #13) et de DX (PR #15, `main` @ `80b4fc5`).
  Constat K9 apporté par la session parente (budget en em) ; K10 ajouté (largeurs d'origine en em sur tout le jeu).
- WR-1 réduit à 6 écrans ; l'utilisateur a tout capturé (A à L). Résultats détaillés dans `tasks/todo.md`,
  « Résultats WR-1 » :
  - taille par défaut `\f[12]` ;
  - `\f[18]` = 1,5 fois, ce qui explique K5 ;
  - fenêtre de ~22 em (22,3 em jusqu'au cadre), même largeur avec visage ;
  - 4 lignes par fenêtre ;
  - texte non coupé au cadre ;
  - ligne d'origine de 25 em qui déborde (défaut du jeu) ;
  - icône de 20 px ;
  - choix qui sortent de l'écran.
- `SysDatabase.project` déclare `基本フォントサイズ` : piste pour la taille par défaut (WR-2 ajusté).

## WR-2 à WR-7 (2026-10-27, worktree `../Hoshi2Star-wr`, branche `feat/wr-wolf-wrap` depuis `main` @ `80b4fc5`)

- WR-2 : la taille par défaut du jeu n'est stockée nulle part de façon lisible.
  - `SysDatabase` n'a que le *nom* `基本フォントサイズ`, sans valeur.
  - `CDataBase` « システム基本フォントサイズ » vaut 0, posée par l'initialisation à l'exécution.
  - `Game.dat` contient 320 et 240, mais pas 12.
  - ⇒ champ manuel.
- WR-3 : sur les cartes 2.23, p99 = 22,0 em et 4 lignes au plus. C'est exactement la mesure en jeu.
- WR-4 à WR-7 : code et tests écrits. Détail et écarts : `tasks/todo.md`, « Plan — WR ».
- Formatage seulement : `rustfmt --check` OK sur les fichiers Rust touchés, `prettier --write` sur les fichiers
  TS/JSON (binaire du dossier principal ; `node_modules` absent du worktree).
- **Non compilé, aucun test exécuté** (règle : pas de build local). Relecture indépendante lancée avant commit.
- Relecture indépendante (agent, lecture seule) : aucune erreur de compilation repérée, attentes de tests vérifiées à la main ; 5 constats, dont 3 corrigés et 2 gardés (détail dans `tasks/todo.md`, « Relecture indépendante »).

## Gate cloud WR-8 (2026-10-27, sur `95e7a76`)

- Résultats :
  - fmt, typecheck, clippy `--all-targets -D warnings` (0 avertissement) : OK ;
  - Vitest 153/153 ;
  - lint : 0 erreur, 10 avertissements, tous sur `main` ;
  - `cargo test --locked --no-fail-fast` : 785 OK / 1 échec / 15 ignorés.
- Tous les tests WR sont verts, y compris l'e2e et le cas Vitest Wolf.
- Correction : symptôme = `fresh_schema_and_repeated_startup_are_idempotent` attend les migrations `1..=12`, et la
  base en a 13. Cause confirmée : la liste est figée dans le test. Solution : `1..=13`. Preuve attendue : la relance
  du gate. Leçon ajoutée.

- Relance sur `f02e174` (session cloud) : fmt OK, `cargo test` **786 OK / 0 échec / 15 ignorés**. **Gate vert.**

## WR-9 : essai en jeu (2026-10-27)

- La base a été sauvegardée (`backups/hoshi2star-avant-essai-wr9-20261027-181808.db`). L'app a été lancée depuis
  le worktree, avec `CARGO_TARGET_DIR` sur la cible du worktree RW (compilation en 40 s) ; migration 13 appliquée.
- Écarts relevés par l'utilisateur dans la boîte de dialogue, corrigés :
  - listes déroulantes illisibles en thème sombre (défaut ancien, partagé avec MZ) ;
  - remarques Wolf en anglais, sous un titre MZ.
- Projet `APP` : messages ajustés, taille du jeu 12, 4 lignes. Export avec « Ignorer » à la question de taille de
  police, puis décompression sur `APP`.
- Relecture hors jeu de `Data.wolf` : 27 messages recomposés, 3 lignes au plus, ligne la plus large 21,77 em.
- En jeu : **OK**. Captures dans `RAE-PLAN/essais/wolf-v223/WR9-captures/`.

## Prochaine session

- Pousser les corrections de l'essai, repasser le gate, puis PR (accord requis).
