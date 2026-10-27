# Journal — 2026-10-26 — Jeux Bakin sous Linux : recherche globale

**Phase** : hors roadmap (étude locale `RAE-PLAN/`, ignorée par Git)
**Durée estimée** : 1h30
**Statut** : ✅ Complété (recherche) ; essais en jeu à faire par l'utilisateur
**Outil** : Claude
**Branche et commit de départ** : `main` @ `2331c75`
**Objectif / plan / story** : demande utilisateur (prompt `RAE-PLAN/bakin/PROMPT-recherche-linux.md`) : faire tourner un jeu RPG Developer Bakin sous Linux (CachyOS, NVIDIA 615.78.08), sans lancer de jeu ni rien installer.

---

## Ce qui a été fait

- Quatre recherches web en parallèle : bugs OpenGL de Wine/Proton ; Zink et mesa-dist-win ; rapports d'utilisateurs Bakin ; lanceurs et VM.
- Lecture du code de Wine 10.0 et 11.0 sur le miroir GitHub.
- Désassemblage de `kmyCore.dll` (objdump, lecture seule).
- Découverte d'un dépôt MIT qui répare un jeu Bakin sous Proton avec le **même `kmyCore.dll`** (md5 identique).
- Hook `LD_PRELOAD` (partie VAO seulement) réécrit et compilé.

## Fichiers créés

- `RAE-PLAN/bakin/linux-recherche-globale.md` (livrable, local)
- `RAE-PLAN/bakin/_outils/bakin_vao_hook.c` et `.so` (local)
- `docs/journal/2026-10-26-bakin-linux-recherche.md`

## Fichiers modifiés

- `RAE-PLAN/README.md` : ligne d'index
- `RAE-PLAN/bakin/linux-wine.md` : renvoi en fin de §9
- `tasks/handoff.md` : pointeur

## Fichiers supprimés

- aucun

## Dépendances ajoutées

- aucune

## Décisions prises

- La conclusion « hors de portée » de `linux-wine.md` §9 est remplacée : causes identifiées.
- Le hook ne reprend pas le patch `ntdll.so` du dépôt (décalage valable pour un seul build Proton).

## Problèmes rencontrés

- « Vertex Attribute Problem(Position) » (wine-cachyos) est un libellé trompeur : il est émis quand aucun plugin chargeur d'image n'a lu la texture. Les plugins `.dlp` ne sont pas chargés sous wine-cachyos ; ce n'est pas une erreur OpenGL.
- Le journal du moteur n'écrit chaque erreur qu'une fois : « deux erreurs seulement » sous Proton masquait un échec à chaque image.

## Corrections et erreurs à ne plus répéter

- Lire un message d'erreur de moteur par son site d'émission, pas par son libellé (vérifié au désassemblage). Pas de règle ajoutée à `tasks/lessons.md` : leçon propre à l'étude.

## Vérifications et limites

- `gcc -O2 -Wall -Wextra` sur le hook : aucun avertissement (2026-10-26).
- Test natif d'interception : `dladdr` sur les pointeurs obtenus par `dlsym(libGL)`, `eglGetProcAddress` et `glXGetProcAddressARB`. Les trois fonctions visées pointent dans le hook ; les autres passent telles quelles.
- Test natif de l'effet : contexte EGL core 4.4 sur la GTX 1660 SUPER, VAO 0. `glVertexAttribPointer` renvoie `0x502` sans le hook et `0x0` avec.
- Correction en cours de session : la première version du hook n'interceptait pas `dlsym(handle, "eglGetProcAddress")`, le chemin EGL de Wine 11. Défaut relevé par relecture, corrigé, puis prouvé par le test d'interception.
- **Non exécuté** : aucun essai du jeu (règle de l'utilisateur).
- EULA de Bakin non lue avant le désassemblage (règle 3 de `RAE-PLAN/README.md`) : signalé dans le livrable.

- Ajout demandé ensuite : mode observation du hook (`BAKIN_VAO_OBSERVE=1`) pour T4. Testé en natif : il ne corrige rien (`0x502` reste en core) et journalise le profil (`0x1` core, `0x2` compatibilité) et les appels sur le VAO 0. Section T4 réécrite (commande, lectures, tableau de conclusions).

- **Résultat de T4** (lancé par l'utilisateur, GE-Proton 11-6, préfixe `~/pfx-bakin-t4`), cause racine confirmée :
  - le moteur demande 4.4 compatibilité (`0x9126 0x2`) ;
  - en recréant les contextes partagés pour `wglShareLists`, Wine 11.0 passe des attributs corrompus (`Unhandled attribList pair: 0x89540260 0x5555`…) et perd le profil ;
  - GLX donne alors un contexte 4.4 **core** (hook : `profile mask 0x1`), et le moteur appelle plus de 90 000 fois `glVertexAttribPointer` sur le VAO 0 ;
  - le bug est dans `memdup_attribs` (`dlls/opengl32/unix_wgl.c` l.518 de Wine 11.0) : la boucle s'arrête au premier 0, même quand c'est une valeur (`WGL_CONTEXT_FLAGS_ARB 0`) ;
  - ce code est absent de Wine 11.20 ; GE-Proton 11-6 est sous GLX (`EGL support is disabled`).

- **Résultat de T1** (lancé par l'utilisateur) : le jeu fonctionne, avec écran titre, carte 3D, personnages et dialogue (3 captures). `(574)` et `(343)` ont disparu, le hook a créé un VAO. Reste une `GfxError 1282` unique à `ShaderProgramGLSL.cpp(634)` (`glBindTexture`), sans effet visible.

- Ensuite, à la demande de l'utilisateur : `~/pfx-bakin-t4` et `steam-default.log` mis à la corbeille (`gio trash`, récupérables). Script `RAE-PLAN/bakin/_outils/bakin.fish` créé pour lancer les variantes B0–B3 avec le correctif (syntaxe et logique vérifiées sans lancer le jeu). Commandes Bakin de `essais/FICHE-DE-TEST.md` mises à jour.

- Essais de traduction en jeu (lancés par l'utilisateur) : B0 et B1 OK. B2 plante au lancement (`OverflowException`, moteur non démarré).
  - Cause trouvée par analyse des octets : des tailles de blocs int32 encadrent les chaînes des `.rbr` (en-tête à l'octet 17, blocs `[taille][code]` dans `Layout.rbr`, objets `[taille][xx][07][GUID][nom]` dans `Map_000.rbr`).
  - Ajouts : `rbpack.size_fields` / `fix_sizes`, variante B4 dans `build_bakin_tests.py`.
  - Contrôles : structure ré-imbriquée après réécriture (2 339 + 24 champs aux positions attendues) ; B4 ne diffère de B2 qu'aux champs de taille. Essai B4 : plus d'overflow, le moteur démarre, puis NullReferenceException dans `LayoutManager..ctor` ; variantes B5 (seul `Layout.rbr`) et B6 (seule la carte) construites pour isoler le fichier fautif.

- B3 (dictionnaire) : japonais avec et sans `/Dic /L=EN` ; le lecteur connaît `/Dic`, le lanceur non (cause probable).
- Lutris (demande utilisateur) : vérifié dans Lutris 0.5.22 que les Proton passent par umu, que les variables d'environnement (`LD_PRELOAD` compris) sont transmises, et que `GE-Proton11-6-x86_64` est déjà visible (un lien ajouté par erreur a été retiré). Créé `~/.local/share/bakin-linux/` (correctif, `bakin-prelaunch.sh` testé sur un faux préfixe, LISEZMOI) et `RAE-PLAN/bakin/lutris-bakin.md`. Lancement par Lutris : non essayé.

- Clôture de l'étude :
  - `rapport.md` §7 et recommandation révisée (archive reconstruite plutôt que dictionnaire) ;
  - contrôle hors jeu des 73 `.rbr` des deux démos : 0 incohérence, mais des candidats de taille rejetés dans `Cast`, `NItem`, `NSkill`, `Camera` et le `Layout` 1.6 ;
  - avertissement `rbpack.rejected_detectors` ajouté au script de construction ;
  - bilan dans `RESULTATS.md`, entrée dans `tasks/todo.md` ;
  - doc partageable pour Lutris.

## Tâches ROADMAP cochées

- aucune

## Prochaine session

- B5 plante comme B4 ; B6 (carte seule) OK en jeu : dialogues traduits à longueur libre validés. Cause de B5 : objets écran `[taille][GUID][16 nuls][nom]` non détectés dans `Layout.rbr` ; détecteur `_screens` ajouté ; base de la taille d'en-tête corrigée (octet 23) ; B7 construite (2 400 champs, aller-retour conforme). B7 OK en jeu : traduction à longueur libre validée (menu, dialogues, « Annonce », variables), journal sans exception. Leçon ajoutée à `tasks/lessons.md`. Suite au choix de l'utilisateur : B3, écran Options, report du format dans le plan Rust.

---
*Journal partagé Codex / Claude — Hoshi2Star*
