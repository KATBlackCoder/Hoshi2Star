# Journal — 2026-10-28 — Plan UW : Wolf lu depuis le Data/ déchiffré par UberWolf

**Phase** : F5
**Statut** : 🔄 En cours (plan écrit, à valider)
**Outil** : Claude
**Branche et commit de départ** : `docs/wolf-uberwolf-plan` depuis `origin/main` @ `e95bb38` (worktree `../Hoshi2Star-uw`)
**Objectif / plan / story** : `tasks/todo.md`, « Plan — UW » ; aucun code applicatif

---

## Ce qui a été fait

- Analyse, en lecture seule de la base de l'app, des 24 956 textes extraits du jeu Wolf 3.658
  (手折らる花へのトロイメライ 体験版). Rapport et classement texte par texte dans `RAE-PLAN/essais/`
  (hors Git). Constat : 19 chemins d'image `bg/*.png⏎0⏎0⏎0` (`SysDatabase`, `遠景ﾌｧｲﾙ名`) passent le
  filtre `is_resource_path`, qui ne teste que la fin du texte entier.
- Deux jeux Wolf Pro ajoutés par l'utilisateur (とのかじ 3.723, ポンコツ 3.714, `GamePro.exe`) : en-têtes
  lus. Chiffrement 1020 (Pro) et 0 (clé propre au jeu), refusés par l'app comme prévu. Déchiffrés par
  l'utilisateur avec UberWolf ; l'app les refuse ensuite (« moteur non détecté ») car elle exige
  `Game.exe` ou `Game.ini`. ポンコツ n'a pas de `SysDatabase.dat` : l'app retomberait sur Shift-JIS.
- ポンコツ : 118 fichiers `Data/Evtext/*.txt` (Shift-JIS, ~9 740 lignes de texte, commandes `/…`) que
  l'app n'extrait pas.
- Essai en jeu (lancé par l'utilisateur) : les deux jeux Pro se lancent sans `.wolf`. Copies btrfs
  marquées par le prototype `RAE-PLAN/wolf/_outils/wolf_v3.py` (« ★ » devant chaque message et choix) :
  « ★ » affiché dans les deux jeux. Aller-retour du prototype identique octet par octet sur 27 + 306
  fichiers.
- Décision de l'utilisateur : baser l'app sur le `Data/` déchiffré par UberWolf. Plan UW écrit.

## Fichiers modifiés

- `tasks/todo.md` — « Plan — UW » en tête.
- `tasks/handoff.md` — point de reprise UW.

## Fichiers créés

- `docs/journal/2026-10-28-plan-wolf-uberwolf.md` — ce journal.

## Décisions prises

- Pas de moteurs séparés par variante de Wolf : même format de texte partout ; seul l'encodage varie.
- Décisions UW-D1 à UW-D8 proposées avec recommandation, à valider par l'utilisateur.

## Vérifications

- Instructions seules : références de lignes relues dans le code de `main` @ `e95bb38`. Aucun test
  applicatif lancé (rien à compiler).

## Prochaine action

- Validation du plan UW par l'utilisateur, puis UW-1.
