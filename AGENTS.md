# Hoshi2Star — consignes communes à Codex et Claude

Répondre en français, tutoyer, sans flatterie ni emojis. Partir de l'existant,
respecter le périmètre demandé et distinguer faits vérifiés, hypothèses et inconnues.

## Reprise obligatoire

À chaque nouvelle session, changement d'outil ou reprise après interruption :

1. Lire `tasks/handoff.md` : objectif actuel, dernier point atteint, fichiers,
   vérifications, blocages et prochaine action. Lire l'entrée de journal liée.
2. Vérifier le dossier et l'état réel avec `pwd`, `git branch --show-current`,
   `git log -1 --oneline`, `git status --short` et les diffs utiles. La branche et
   le commit consignés sont un instantané, pas une instruction de changer de branche.
   Préserver les modifications locales préexistantes, y compris non suivies.
3. Lire `tasks/lessons.md` et `CONTEXT.md`. Pour une tâche produit, lire les parties
   pertinentes de `tasks/todo.md`, `ROADMAP.md` et du plan ou de la story concernée.
4. Avant d'agir, annoncer brièvement : ce qui est acquis, ce qui reste incertain
   ou bloqué, puis la prochaine action autorisée. Ne pas refaire une correction
   sans vérifier le code et les preuves existantes.

Une conversation privée ou une mémoire propre à un outil n'est pas un état partagé.
Reporter les décisions utiles dans le dépôt. Ne jamais supposer que l'autre outil
a lu une conversation. En cas de contradiction documentaire, vérifier le code et
les tests concernés, puis corriger le suivi dans le périmètre de la tâche.

## Où conserver chaque information

| Information | Source commune |
| --- | --- |
| Point de reprise immédiat, travail interrompu, prochaine action | `tasks/handoff.md` |
| Historique daté des changements, corrections et vérifications | `docs/journal/` |
| Erreurs à éviter, approches abandonnées et raison | `tasks/lessons.md` |
| Carte du code (rôle des dossiers, parcours, où changer quoi) | `docs/architecture.md` |
| Conventions techniques et références d'architecture | `CONTEXT.md`, `docs/adr/` |
| Tâches détaillées et plans | `tasks/todo.md`, `docs/plans/`, `docs/superpowers/plans/` |
| Orientation et idées futures | `ROADMAP.md`, `BACKLOG.md` |

`tasks/handoff.md` reste court : pointer vers les détails, sans recopier tout
l'historique. Les anciens statuts et tests restent datés ; ils ne garantissent
pas que l'état actuel fonctionne. Distinguer prévu, implémenté, vérifié et livré.
Une idée de backlog ou une recommandation d'audit n'autorise pas son implémentation.

## Enregistrer le progrès pendant le travail

- Mettre à jour `tasks/handoff.md` après chaque étape significative et avant de
  rendre la main : outil, date, objectif, statut, fichiers/fonctions concernés,
  actions faites, tests avec résultat, limites, blocages et prochaine action exacte.
  Ne pas attendre exclusivement la fin de session : une interruption peut survenir.
- Après une correction, consigner symptôme, cause confirmée, solution et preuve
  dans le journal. Ajouter à `tasks/lessons.md` la règle durable qui évite la
  récidive, avec un lien vers la preuve ; compléter une règle existante sans doublon.
- Mettre à jour la tâche ou la story concernée. Ne cocher « terminé » qu'avec les
  vérifications adaptées ; écrire « non exécuté » si un test n'a pas été lancé.
- Créer une entrée de journal par session de modification à partir de
  `docs/journal/TEMPLATE.md`, identifiée Codex ou Claude. Ne pas réécrire une
  ancienne session : une rectification va dans une nouvelle entrée liée.
- Ne pas stocker de secrets, de clés API ni de transcriptions privées intégrales.

## Règles de travail communes

- Lire les conventions de `CONTEXT.md` avant toute modification applicative.
  Utiliser pnpm pour JavaScript. Respecter Tauri v2 et les identifiants de
  compatibilité existants : un changement de nom affiché n'est pas une migration.
  Avant de travailler dans une zone peu connue, lire la carte du code `docs/architecture.md`.
- Tout lot qui crée, déplace ou supprime une frontière de module (dossier, module, couche, point d'entrée)
  met `docs/architecture.md` à jour dans la même PR.
- Pour une implémentation non triviale, écrire un plan vérifiable dans
  `tasks/todo.md` ou dans le plan/story actif avant de modifier le code.
- Pour du code applicatif, vérifier TypeScript, les tests frontend et Rust et
  Clippy conformément aux règles du projet. Pour des instructions seules, vérifier
  les références, la cohérence et le diff ; ne pas présenter cela comme un test applicatif.
- Avant toute écriture dans une session concurrente, vérifier les changements
  récents. Un seul outil modifie une même tâche à la fois ; si une autre session
  est active sur les mêmes fichiers, coordonner la reprise avant de les modifier.
- Sur un autre clone ou worktree, transférer le code ET le suivi correspondant
  via Git ou une copie explicite. Des fichiers non commités ne suivent pas une
  autre machine. Ne pas publier ni inclure des fichiers hors périmètre par défaut.

## Utilisation avec BMAD

BMAD 6.12.0 (Core + BMM) est installé pour Codex (`.agents/skills/bmad-*`) et
Claude Code (`.claude/skills/bmad-*`). Configuration commune : `_bmad/` ; sorties :
`_bmad-output/`. Consulter `docs/bmad.md` pour l'utilisation et la maintenance.

Avant toute activation BMAD, appliquer la reprise obligatoire ci-dessus.
Référencer sa story/spec et son fichier de suivi actifs depuis `tasks/handoff.md`.
Le statut détaillé reste dans cette source, sans seconde liste concurrente.
Une installation BMAD ne doit ni effacer les leçons ni remettre les tâches à zéro.
