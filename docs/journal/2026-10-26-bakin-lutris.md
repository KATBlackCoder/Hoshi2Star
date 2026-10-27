# Journal — 2026-10-26 — Jeu Bakin lancé depuis Lutris

**Phase** : hors produit (étude locale `RAE-PLAN/`, suite de `2026-10-26-bakin-linux-recherche.md`)
**Durée estimée** : 1h
**Statut** : ✅ Complété (vrai préfixe non essayé sous Lutris)
**Outil** : Claude (session secondaire)
**Branche et commit de départ** : `main`, `2331c75`
**Objectif / plan / story** : faire valider par l'utilisateur le lancement d'un jeu RPG Developer Bakin depuis Lutris, au lieu de `bakin.fish`. Procédure : `RAE-PLAN/bakin/lutris-bakin.md`.

---

## Ce qui a été fait

- Relevé de l'état de départ :
  - ancien journal du correctif renommé en `~/bakin-vao-hook.log.avant-lutris`, pour qu'un nouveau journal prouve le lancement par Lutris ;
  - journal du jeu B7 : 2 254 lignes, 18:13:23.
- Lecture des réglages globaux de Lutris (`~/.local/share/lutris/system.yml`, `runners/wine.yml`) :
  - `gamescope: true` pour tous les jeux, donc désactivé pour celui-ci ;
  - Wine par défaut : `Proton-CachyOS Latest`, donc à changer.
- Lecture du code de Lutris 0.5.22 (`runner_interpreter.py`) : les variables du jeu passent dans l'environnement, et GameMode ajoute `gamemoderun`.
- Création du jeu par l'utilisateur dans Lutris (lancé avec `lutris -d`), exécutable B7. L'utilisateur a lancé le jeu et envoyé 3 captures : écran titre, Options, confirmation de sortie.
- Lecture des journaux et de la configuration ; mise à jour de la procédure.

## Fichiers créés

- `docs/journal/2026-10-26-bakin-lutris.md`
- `RAE-PLAN/bakin/lutris-b7-debug.log`, `lutris-b7-vao-hook.log`, `lutris-b7-bakinplayer_log.txt` : preuves, dossier ignoré par Git.

## Fichiers modifiés

- `RAE-PLAN/bakin/lutris-bakin.md` (ignoré par Git) :
  - statut « vérifié », YAML réel ;
  - mises en garde : espace en fin de chemin, Gamescope ;
  - messages sans effet, `GLCache/`.
- `RAE-PLAN/essais/RESULTATS.md` (ignoré par Git) : écran Options de B7 vu.
- `tasks/todo.md` : ligne Lutris cochée.
- `tasks/handoff.md` : point Lutris.

## Fichiers supprimés

- Après la validation, à la demande de l'utilisateur, mis à la corbeille avec `gio trash` (environ 7,7 Gio exclusifs mesurés par `btrfs filesystem du -s`) :
  - `RAE-PLAN/essais/bakin-v07/B0` à `B6`, `RAE-PLAN/essais/wolf-v3/V2`, `RAE-PLAN/essais/wolf-v223/W2` ;
  - `~/bakin-dotnet`.

  La place est libérée quand la corbeille est vidée.
- `"/home/blackat/pfx-bakin-t1 "` (préfixe parasite, 211 Mo), mis à la corbeille, hors dépôt.

## Dépendances ajoutées

- Aucune.

## Décisions prises

- Premier essai sans Gamescope, pour reproduire les conditions éprouvées.
- GameMode laissé actif : il ne gêne pas le correctif.

## Problèmes rencontrés

- **Espace en fin de chemin du préfixe**, saisie dans Lutris.
  - Symptôme : `No valid prefix detected in /home/blackat/pfx-bakin-t1 , creating one...`.
  - Conséquence : nouveau préfixe `"/home/blackat/pfx-bakin-t1 "` (211 Mo) créé et utilisé par le jeu.
  - Preuves : `STEAM_COMPAT_SHADER_PATH=/home/blackat/pfx-bakin-t1 /shadercache` ; `prefix: '/home/blackat/pfx-bakin-t1 '` dans `b7-1793041033.yml` (`cat -A`).
  - Effet : le jeu marche, mais le script de pré-lancement vise l'autre préfixe.
  - Correction : retirer l'espace dans Lutris (à faire par l'utilisateur), puis mettre le préfixe parasite à la corbeille (accord demandé). Mise en garde ajoutée à la procédure.
- `ShellExecuteEx a échoué: Fichier introuvable` (code 256) : vient probablement de `umu-run createprefix` pendant la création du préfixe [D]. Le jeu se lance ensuite.

## Corrections et erreurs à ne plus répéter

- Lutris ne retire pas les espaces en fin de champ. Un chemin de préfixe invalide crée un préfixe neuf sans prévenir. Contrôle : `cat -A` sur le `.yml` du jeu. Consigné dans `lutris-bakin.md`. Ce n'est pas une règle de travail du projet, donc rien dans `tasks/lessons.md`.

## Vérifications et limites

- [V] `~/bakin-vao-hook.log` neuf : `loaded in bakinplayer`, `intercepted dlsym glXGetProcAddressARB`, `context 0x7f7cd4001208: created VAO 2`.
- [V] `B7/data/bakinplayer_log.txt` (18:58:54, 102 lignes) : aucune erreur ni exception ; sortie `KMY MAIN LOOP BREAK BY NO TASK`.
- [V] Journal de Lutris :
  - `LD_PRELOAD=libgamemodeauto.so.0:/home/blackat/.local/share/bakin-linux/bakin_vao_hook.so` ;
  - `BAKIN_VAO_LOG=1`, `PROTONPATH=…/GE-Proton11-6-x86_64`, `LC_ALL=ja_JP.utf8` ;
  - script de pré-lancement exécuté, avec attente.
- [V] Captures de l'utilisateur :
  - écran titre `[B7] Nouveau / Suite / Options / Quitter` ;
  - écran Options (`コンフィグ`, en japonais : textes non traduits dans B7) ;
  - confirmation `本当にゲームを終了しますか`.
- Non vu depuis Lutris : la carte (« Nouveau »), déjà validée avec `bakin.fish`.
- Avec Gamescope : non essayé.
- Avec le vrai préfixe `pfx-bakin-t1` sous Lutris : non essayé, à cause de l'espace. Le jeu a marché dans le préfixe neuf créé par Lutris : un préfixe neuf convient donc [V]. Le YAML de la procédure a été corrigé à la main ; Lutris ne l'a pas écrit tel quel.
- Aucun test applicatif : rien dans le code du produit n'a changé.

## Tâches ROADMAP cochées

- Aucune (étude hors ROADMAP).

## Prochaine session

- Pas de relance, par décision de l'utilisateur (2026-10-26). Le vrai préfixe `pfx-bakin-t1` n'a donc **pas** été essayé sous Lutris, et la carte n'a pas été vue depuis Lutris.
- Préfixe parasite mis à la corbeille le 2026-10-26, avec l'accord de l'utilisateur :
  - vérifié d'abord avec `ls -d "/home/blackat/pfx-bakin-t1 "` ;
  - puis `gio trash "/home/blackat/pfx-bakin-t1 "` ;
  - le vrai `pfx-bakin-t1` est intact.
- Reste pour l'utilisateur, dans Lutris :
  - retirer l'espace du chemin du préfixe, sinon le prochain lancement recrée le préfixe parasite ;
  - retirer `BAKIN_VAO_LOG`.
- Mettre à jour la copie en doc Claude de `lutris-bakin.md`.

---
*Journal partagé Codex / Claude — Hoshi2Star*
