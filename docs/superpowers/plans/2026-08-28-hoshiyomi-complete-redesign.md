# Hoshiyomi Complete Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Renommer l’expérience visible en Hoshiyomi et livrer une refonte cohérente « Hoshi Observatory × Celestial Shrine » sur la Bibliothèque, le workflow Patch, la Terminologie, les paramètres, les dialogues et les états transverses, sans modifier les identifiants de compatibilité Hoshi2Star.

**Architecture:** Conserver React/Tauri/SQLite, les stores et les commandes IPC existantes. Ajouter une petite couche de composants de shell et de workflow réutilisables, faire porter les thèmes par des tokens CSS, et adapter les composants existants au lieu de dupliquer les flux métier. Les noms techniques (`hoshi2star` dans Cargo, identifiant Tauri, base, manifestes, packs `.h2s`, URLs d’update) restent intacts; seules les surfaces produit et le `productName` visible deviennent Hoshiyomi.

**Tech Stack:** React 19, TypeScript strict, Tailwind CSS v4, shadcn/Radix, Zustand, TanStack Query/Table/Virtual, Tauri v2, Vitest/Testing Library, MCP Tauri.

---

## Cartographie des fichiers

- Create: `src/components/brand/HoshiyomiMark.tsx` — identité accessible et compacte.
- Create: `src/components/common/WorkspaceState.tsx` — états vide/chargement/erreur cohérents.
- Create: `src/components/patch/PatchWorkflow.tsx` — cinq portes Extraction/Termes/Traduction/Révision/Export raccordées aux actions existantes.
- Modify: `src/index.css` — tokens Nuit céleste/Aube de papier, surfaces, focus, wrapping, overflow et reduced motion.
- Modify: `src/components/shell/ModeNavigation.tsx`, `ContextBar.tsx`, `AppShell.tsx` — coque responsive, marque et navigation.
- Modify: `src/App.tsx`, `src/components/AppToolbar.tsx` — composition Patch et états différés.
- Modify: `src/components/editor/ProjectList.tsx` — Bibliothèque de projets et progression.
- Modify: `src/features/terminology/TerminologyWorkspace.tsx`, `TerminologyToolbar.tsx`, `TerminologyTable.tsx` — hiérarchie et robustesse des tableaux.
- Modify: `src/components/settings/*.tsx`, `src/components/AboutModal.tsx`, `src/components/ui/dialog.tsx`, `src/components/ui/alert-dialog.tsx` — dialogues adaptatifs, thèmes nommés et focus.
- Modify: `src/locales/fr.json`, `src/locales/en.json`, `index.html`, `src-tauri/tauri.conf.json` — marque visible et nouveaux libellés.
- Test: composants concernés et nouveaux tests shell/workflow/états.
- Create: `docs/validation/hoshiyomi-redesign-2026-08-28.md` — matrice de validation et limites.

### Task 1: Verrouiller marque, thèmes et garde-fous responsive

- [ ] Ajouter `HoshiyomiMark` avec nom produit, signature `星詠み工房` et variante compacte.
- [ ] Remplacer les couleurs Tenmon par les tokens sémantiques observatoire/sanctuaire : indigo/ivoire, or d’information et vermillon exclusivement décisionnel.
- [ ] Ajouter des garde-fous globaux `min-width: 0`, wrapping, focus visible, cibles de 40 px et réduction de mouvement.
- [ ] Renommer seulement les surfaces visibles et conserver tous les identifiants/chemins/artefacts techniques documentés.
- [ ] Exécuter `pnpm test -- ModeNavigation button && pnpm typecheck` ; attendu : succès.

### Task 2: Recomposer la coque globale et la navigation

- [ ] Faire de la barre supérieure une navigation responsive à cinq espaces, avec libellés masqués proprement sous le seuil compact et `aria-current`.
- [ ] Ajouter un lien d’évitement vers `#workspace-main`, des repères de focus et un nom accessible pour chaque icône.
- [ ] Recomposer la barre de contexte pour les noms de projets/modèles longs sans chevauchement.
- [ ] Tester clic, flèches, état actif, marque et rendu compact.
- [ ] Exécuter `pnpm test -- ModeNavigation ContextBar AppShell` ; attendu : succès.

### Task 3: Redessiner la Bibliothèque

- [ ] Créer un en-tête d’observatoire, un appel d’ouverture, une grille de projets et une synthèse des capacités présentes/futures.
- [ ] Transformer chaque projet en carte clavier-native avec nom long multilingue, chemin, moteur, paire de langues, date et progression accessible.
- [ ] Conserver reprise, suppression et ouverture de jeu sans nouveau flux métier.
- [ ] Couvrir l’état vide, le chargement d’ouverture, l’erreur IPC et la confirmation de suppression.
- [ ] Exécuter `pnpm test -- ProjectList` ; attendu : succès.

### Task 4: Ajouter les cinq portes du workflow Patch

- [ ] Créer `PatchWorkflow` avec Extraction, Termes, Traduction, Révision et Export.
- [ ] Relier Termes à l’espace Terminologie, Révision au rail QA, Export à l’action existante; représenter Extraction/Traduction par l’état réel du projet.
- [ ] Donner aux portes un état texte + icône (jamais couleur seule), une navigation horizontale scrollable sans chevauchement, et des boutons natifs.
- [ ] Intégrer le workflow à l’espace Patch sans perturber TanStack Table/Virtual ni le redimensionnement des panneaux.
- [ ] Exécuter `pnpm test -- PatchWorkflow AppToolbar InspectorRail SegmentGrid` ; attendu : succès.

### Task 5: Harmoniser Terminologie et QA

- [ ] Recomposer l’en-tête et les compteurs Terminologie comme instruments de l’observatoire.
- [ ] Renforcer les états proposed/approved/locked par libellé, symbole, contraste et styles cohérents en clair/sombre.
- [ ] Garantir le scroll horizontal local du tableau, le wrapping des contenus éditoriaux et l’absence de débordement global.
- [ ] Harmoniser l’inspecteur TM/QA/Terminologie avec les mêmes surfaces et états.
- [ ] Exécuter `pnpm test -- TerminologyWorkspace TerminologyTable TerminologyInspector QAPanel` ; attendu : succès.

### Task 6: Refaire paramètres, apparence et dialogues

- [ ] Rendre Settings utilisable sous 1260×600 et aux plus petites tailles Tauri : hauteur bornée, navigation adaptable, panneaux scrollables, footer stable.
- [ ] Nommer les thèmes « Aube de papier » et « Nuit céleste » avec aperçus visuels, sans ajouter de troisième préférence.
- [ ] Corriger labels, noms accessibles, messages live/error et focus des dialogues existants.
- [ ] Adapter About, import/export, traduction globale et dialogues terme aux textes longs et à `max-height`.
- [ ] Exécuter `pnpm test -- SettingsModal TermEditorDialog TermTranslateDialog PackImportWizard` ; attendu : succès.

### Task 7: Validation de bout en bout

- [ ] Exécuter `pnpm test`, `pnpm typecheck`, `pnpm lint`, `pnpm build` et les tests Rust ciblés sans dépendance externe.
- [ ] Démarrer l’application Tauri avec le bridge MCP, vérifier Bibliothèque/Patch/Terminologie/Lecteur/Images, paramètres et dialogues.
- [ ] Capturer Nuit céleste et Aube de papier à 1260×600 puis à une taille compacte prise en charge.
- [ ] Exécuter dans le webview un audit de `scrollWidth > clientWidth`, des rects hors viewport, des contrôles sans nom et des cibles trop petites; corriger chaque résultat réel.
- [ ] Documenter captures, parcours, fichiers touchés, tests et limites dans `docs/validation/hoshiyomi-redesign-2026-08-28.md`.

## Auto-revue

- Couverture : les cinq onglets, deux thèmes, workflow, paramètres/providers, terminologie, QA, dialogues et états sont affectés à une tâche.
- Compatibilité : les identifiants Cargo/Tauri, fichiers `.hoshi2star*`, base `hoshi2star.db`, format `.h2s` et URL d’update ne sont pas migrés.
- Performance : aucune dépendance d’animation ou de données n’est ajoutée; TanStack et la virtualisation restent en place.
- Accessibilité : boutons natifs, focus, cibles, contrastes, libellés, reduced motion et audits clavier/DOM sont explicitement testés.
