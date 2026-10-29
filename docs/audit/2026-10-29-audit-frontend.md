# Audit du front-end — 2026-10-29 (Claude)

**Périmètre :** `src/`, hors `src/components/ui/` (shadcn généré). Lecture du code seulement : aucune modification applicative.
**Base :** `main` @ `b999c9b`. Aucun fichier de `src/` n'a changé depuis `8e32c99` (`git diff --stat 8e32c99 HEAD -- src/` vide). Les références `fichier:ligne` valent pour ces deux commits.
**Objectif utilisateur :** comprendre le fonctionnement du front-end et relever ses défauts majeurs (doublons, lenteurs), en particulier la lenteur du défilement des textes d'un jeu extrait. Ce rapport doit servir de base à un plan d'optimisation.

## Méthode et limites

- Analyse statique. La grille de segments a été lue directement. L'architecture, les doublons et les lenteurs hors grille ont été relevés par trois sous-agents. Leurs constats principaux ont été revérifiés dans le code (marqués « vérifié »).
- Le comportement de `@tanstack/react-virtual` 3.13.25 / `virtual-core` 3.15.0 a été vérifié dans `node_modules`.
- Volumétrie relevée en lecture seule dans la base de l'app (`~/.local/share/com.blackat.hoshi2star/hoshi2star.db`).
- **Rien n'a été mesuré à l'exécution.** Aucun profil React, aucune timeline WebKit. Le gate lourd et `tauri dev` n'ont pas été lancés (ils font geler ce PC). Chaque mécanisme ci-dessous est vérifié dans le code, mais **son poids réel reste à mesurer** (protocole en §3.4).
- Statuts : **vérifié** = lu dans le code ; **hypothèse** = plausible, non démontré.

## 1. Résumé

1. **Défilement lent de la grille.** Il vient du rendu, pas du chargement. Le fichier le plus gros (12 423 segments, moins de 1 Mo de texte) est entièrement chargé en mémoire avant affichage. Ensuite, à chaque événement de défilement :
   - la grille entière est re-rendue de façon synchrone (`flushSync`, comportement par défaut de `useVirtualizer`) ;
   - chaque ligne qui apparaît monte un `<textarea>` redimensionné après l'affichage, puis est remesurée ;
   - la hauteur estimée des lignes (56 px) est inférieure à la hauteur réelle (environ 2,4 lignes de texte japonais). La hauteur totale grossit donc pendant la descente et l'ascenseur « recule ». C'est l'impression d'un chargement continu vers le bas.
2. **Re-rendus de toute l'application.** Ils sont dus à des abonnements à des stores Zustand entiers :
   - chaque événement LLM re-rend `App` et tout l'arbre, dialogues fermés compris ;
   - chaque clic sur une ligne re-rend `App`.
3. **IPC en double ou inutile** :
   - stats de projet chargées sans aucun consommateur ;
   - rafraîchissements doublés après resync et import ;
   - rechargement complet de la grille après une traduction déjà patchée en mémoire.
4. **Coûts CSS propres à WebKitGTK :** `backdrop-blur` sur des cartes qui défilent, `text-wrap: pretty` sur toutes les cellules source, starfield peint mais invisible.
5. **Doublons :** environ 330 lignes factorisables. Les principales : traduction LLM ×2, abonnement aux événements Tauri, stores de dialogue ×3, panneaux de l'inspecteur. S'y ajoutent des incohérences (seuils QA à 4 endroits) et du code mort.

## 2. Fonctionnement du front-end

**Arbre de montage**

- `main.tsx` monte `StrictMode` > `QueryClientProvider` > `App`.
- `App` appelle `useAppHandlers()`, qui porte l'état des dialogues et de l'export. Il rend `AppShell` (`ModeNavigation` + `ContextBar` + `<main>`), `AppDialogs`, `UpdateDialog` et `Toaster`.
- `ui.mode` choisit l'espace de travail :
  - `library` : `ProjectList` ;
  - `terminology` : `TerminologyWorkspace`, seul écran en `React.lazy` (`App.tsx:32`) ;
  - `patch` : `PatchWorkflow` + `AppToolbar`, puis, au choix :
    - `PilotComparisonWorkspace` ;
    - un `ResizablePanelGroup` : `FileTree` | `SegmentGrid` (ou `GlobalSearchResults`) | `InspectorRail` (`TMPanel`, `QAPanel`, `TextFitPreviewPanel`, `TerminologyInspector`).
- **Toujours montés, même fermés :** `SettingsModal`, `AboutModal`, `TranslateAllDialog`, `ExportIntegrityDialog`, `ResyncDialog`, `TextCoverageDialog` (643 lignes), `TextFitDialog` (549 lignes), `UpdateDialog`, plus 3 `AlertDialog`.

**Données**

- **Stores Zustand :** project, editor, llm, ui, search, settings, coverage, pilot, resync, textFit, updater.
- **Données principales** (projets, fichiers, stats, segments, recherche) : `invoke` brut puis `setState`, sans cache ni dédoublonnage.
- **React Query** sert seulement à quelques panneaux locaux : QA, TM, text-fit, terminologie, police du jeu.

**Grille de segments** (`SegmentGrid.tsx`, `features/editor/columns.tsx`)

- `get_segments` est appelé page par page (2 000 lignes) jusqu'au total. Le tableau complet est ensuite stocké dans un `useState` local (`SegmentGrid.tsx:91-123`).
- Filtre QA et recherche : `useMemo` sur tout le tableau.
- TanStack Table (`getCoreRowModel`) assure la sélection, `useVirtualizer` la virtualisation (hauteur estimée 56/40 px, `overscan: 5`, mesure dynamique via `measureElement`).
- Cellules : `SourceCell` (surlignage des codes, `memo`), `EditableCell` (`<textarea>` auto-dimensionné, `memo`), `StatusBadge` (`memo`).
- Mises à jour LLM : l'événement `h2s://llm/segments-updated` est regroupé par `requestAnimationFrame` (bon point). L'événement `h2s://llm/completed` recharge tout.

**Volumétrie réelle** (base de l'app, 2026-10-29)

| Fichier | Segments | Lignes / segment (moy. / max) |
|---|---|---|
| トロイメライ `CommonEvent.dat` | 12 423 | 2,4 / 5 |
| ポンコツ `CommonEvent.dat` | 7 482 | 2,4 / 4 |
| とのかじ `DataBase.dat` | 6 411 | 2,9 / 141 |
| ポンコツ `DataBase.dat` | 5 194 | 4,6 / 67 |

Le texte du plus gros fichier pèse 0,77 Mo. Le chargement initial (7 appels IPC successifs) n'explique donc pas une lenteur **pendant** le défilement.

## 3. Diagnostic : défilement lent de la grille

### 3.1 Causes, par niveau de confiance

**G1. Re-rendu synchrone de toute la grille à chaque événement de défilement (vérifié, impact à mesurer)**
- `useVirtualizer` a `useFlushSync = true` par défaut (`@tanstack/react-virtual/dist/esm/index.js`).
- Pendant le défilement, `virtual-core` appelle `notify(isScrolling = true)`, ce qui exécute `flushSync(rerender)`.
- Tout `SegmentGrid` est donc re-rendu de façon synchrone dans le gestionnaire de scroll : en-tête, barre de filtres, pied de page et environ 30 lignes visibles.
- Les lignes ne sont pas mémoïsées (`SegmentGrid.tsx:534-581`). Chacune recrée son `onClick`, appelle `flexRender` sur 7 cellules et calcule `cn()`/twMerge par cellule (environ 240 appels par frame).
- **Correctif :** `useFlushSync: false` ; extraire une `SegmentRow` mémoïsée ; remplacer `cn()` par des classes statiques par colonne.

**G2. Hauteur estimée trop basse : la liste « s'allonge » pendant la descente (vérifié pour le mécanisme, chiffre réel à mesurer)**
- `estimateSize` vaut 56 px (40 px en compact), quel que soit le texte (`SegmentGrid.tsx:393`).
- Une ligne réelle compte en moyenne 2,4 lignes de texte `text-xs leading-relaxed`, plus le padding et le `<textarea>`. Elle dépasse donc souvent l'estimation (calcul théorique : environ 65 à 70 px, non mesuré).
- Chaque ligne mesurée après montage augmente `getTotalSize()`. Sur 12 423 lignes, l'ascenseur rapetisse et recule pendant la descente, et la fin de liste semble « se charger » au fur et à mesure.
- Une ligne au-dessus de la fenêtre qui change de taille provoque aussi une correction de `scrollTop` pendant la descente (`virtual-core`, `resizeItem`).
- **Correctif :** estimer la hauteur depuis le nombre de lignes du texte (`max(source, target)`) ; garder la mesure dynamique pour les cas extrêmes (141 lignes).

**G3. Un `<textarea>` redimensionné après affichage dans chaque ligne (vérifié, impact à mesurer)**
- `EditableCell` (`columns.tsx:84-89`) fixe `height = auto`, puis lit `scrollHeight` dans un `useEffect`, donc **après** l'affichage.
- Chaque ligne qui apparaît force un layout synchrone, change de hauteur, déclenche le `ResizeObserver`, puis une remesure et un nouveau rendu.
- **Correctif :** afficher le texte cible en lecture seule et ne monter le `<textarea>` qu'au focus ou au clic. À défaut, passer en `useLayoutEffect` ou en CSS `field-sizing: content` (support WebKitGTK à vérifier).
- ⚠ Le défaut connu **S-U3** (saisie perdue au démontage, `docs/audit/2026-10-analyse-complete.md`) touche la même cellule. Toute refonte doit sauvegarder au démontage.

**G4. `text-wrap: pretty` sur chaque cellule source (vérifié pour la règle, coût hypothétique)**
- `index.css` applique `p, li, figcaption { text-wrap: pretty; }`. Or `SourceCell` est un `<p>` (`columns.tsx:155`).
- Si WebKitGTK implémente `pretty`, chaque cellule visible paie un calcul de césure à chaque layout.
- `text-rendering: optimizeLegibility` sur `body` va dans le même sens.
- **Correctif :** limiter `pretty` aux conteneurs de prose ; `text-wrap: wrap` dans la grille.

**G5. Clé de ligne = index (vérifié)**
- `getItemKey` n'est pas fourni, donc le cache de tailles est indexé par position.
- Après un filtre ou une recherche, les tailles en cache ne correspondent plus aux bonnes lignes : sauts et remesures.
- **Correctif :** `getItemKey: (i) => rows[i].id`.

**G6. Rechargement complet qui démonte la liste (vérifié)**
- `isLoading` retourne un écran « Chargement » (`SegmentGrid.tsx:407-415`) et démonte le conteneur de défilement.
- Cela arrive à chaque `llm/completed`, `import-done` ou `resynced`. Les 12 423 lignes sont rechargées, la position de défilement est perdue, et la saisie en cours aussi (S-U3).
- **Correctif :** garder la liste affichée pendant un rechargement. Après `llm/completed`, ne recharger que si nécessaire, puisque `segments-updated` a déjà tout patché.

**G7. Re-rendus venus de l'extérieur (vérifié)**
- `SegmentGrid.tsx:65` lit tout le store LLM (`useLlmStore()`). Pendant une traduction, la grille re-rend à chaque événement de progression, de métriques ou de cooldown (environ 1/s).
- Elle re-rend aussi avec `App` (voir §4).
- `overscan: 5` est faible : en défilement rapide, des zones vides peuvent apparaître (hypothèse).

### 3.2 Points sans effet sur la lenteur

- Taille des données transférées : moins de 1 Mo de texte. `get_segments` utilise `LIMIT/OFFSET` (7 pages pour 12 423 lignes) ; c'est négligeable une fois les données chargées.
- `SourceCell` et `StatusBadge` sont mémoïsés : ils ne se recalculent pas pendant le défilement.

### 3.3 Ordre d'expérimentation conseillé (du moins cher au plus structurant)

1. `useFlushSync: false` (une ligne).
2. `getItemKey` par id de segment ; `overscan` de 5 à 10.
3. `estimateSize` calculé depuis le nombre de lignes du texte.
4. `text-wrap: wrap` dans la grille.
5. `SegmentRow` mémoïsée, classes statiques par colonne.
6. Cellule cible en texte simple, `<textarea>` au focus (avec sauvegarde au démontage, cf. S-U3).
7. Pas de démontage pendant les rechargements ; pas de rechargement après `llm/completed`.

### 3.4 Protocole de mesure (à lancer par l'utilisateur ou en session cloud)

- **Build release** (`pnpm tauri build`) : en dev, `StrictMode` double les rendus et le profil n'est pas représentatif.
- **Fichier test :** トロイメライ `CommonEvent.dat` (12 423 segments). Faire défiler du haut en bas : à la molette sur 10 s, puis en tirant l'ascenseur.
- **Mesures :**
  - nombre de rendus de `SegmentGrid` par seconde de défilement (React Profiler, ou un compteur temporaire) ;
  - images par seconde et longues tâches (inspecteur WebKit, onglet Timelines) ;
  - variation de `virtualizer.getTotalSize()` entre le haut et le bas.
- Mesurer avant et après chaque étape du §3.3, et ne garder que les étapes qui gagnent.

## 4. Re-rendus globaux

| # | Constat | Où | Impact | Statut |
|---|---|---|---|---|
| R1 | `App` lit tout le store LLM via `useAppHandlers` : chaque `set` du store LLM (progression, métriques, cooldown environ 1/s) re-rend tout l'arbre, dialogues fermés compris. Seules les cellules de la grille sont en `memo`. | `useAppHandlers.ts:59`, `App.tsx:42` | élevé | vérifié |
| R2 | Même abonnement au store entier dans la grille et dans les résultats de recherche. | `SegmentGrid.tsx:65`, `GlobalSearchResults.tsx:42` | élevé | vérifié |
| R3 | `App` lit le texte du segment actif pour le passer à `QAPanel` : chaque clic sur une ligne et chaque sauvegarde re-rendent tout l'arbre. `TMPanel` lit déjà le store directement. | `App.tsx:47-52`, `SegmentGrid.tsx:555` | élevé | vérifié |
| R4 | `requestMetrics` et `pipelineMetrics` sont recopiés à chaque ajout (O(n²) sur un run) et n'ont aucun lecteur. Leur seul effet est de déclencher R1/R2. | `llm.ts:258-263`, `:443-444` | moyen | vérifié |
| R5 | `App` lit tout le store des réglages pour `loadSettings` ; `TextFitDialog`, `TextCoverageDialog` et `ResyncDialog` lisent aussi leur store entier (`TextFitDialog` re-rend même fermé). | `App.tsx:44`, `TextFitDialog.tsx:65-66`, `TextCoverageDialog.tsx:65`, `ResyncDialog.tsx:31` | faible | vérifié |
| R6 | `columns` et `handleTabNext` sont recréés à chaque changement de `filteredSegments.length`, donc à chaque frappe dans la recherche. Changer de fichier enchaîne deux effets, donc deux rendus. | `SegmentGrid.tsx:154-173`, `:345-376` | faible | vérifié |
| R7 | `GlobalSearchResults` refait `results.map` à chaque `segments-updated`, sans regroupement par frame et même sans résultat touché. | `GlobalSearchResults.tsx:71-82` | moyen pendant un run | vérifié |
| R8 | `PilotComparisonWorkspace` : 50 cartes non mémoïsées avec callback en ligne ; un clic les re-rend toutes. | `PilotComparisonWorkspace.tsx:221` | faible | vérifié |

**Correctif commun :** des sélecteurs fins (`useLlmStore((s) => s.startTranslation)`), `getState()` dans les handlers pour la configuration, et `QAPanel` qui lit le segment actif lui-même.

## 5. IPC et rechargements

| # | Constat | Où | Statut |
|---|---|---|---|
| I1 | `activeProjectStats` n'a aucun consommateur. Les appels `get_project_stats` qui servent à le remplir sont inutiles. | `SegmentGrid.tsx:136`, `project.ts:164` | vérifié |
| I2 | Resync : `resynced` déclenche le rafraîchissement de la grille, **et** `resync.ts:55` appelle `refreshProjectData` (deux fois `get_source_files` et `get_project_stats`). Même schéma pour l'import de pack. | `SegmentGrid.tsx:270-286`, `resync.ts:55`, `PackImportWizard.tsx:114` | vérifié |
| I3 | Après `llm/completed`, toutes les pages de segments sont rechargées, alors que `segments-updated` les a déjà patchés. | `SegmentGrid.tsx:274` | vérifié |
| I4 | `ProjectList` relance N `get_project_stats` à chaque montage (passage library ↔ patch), avec un `setState` par projet, donc N re-rendus de cartes floutées. | `ProjectList.tsx:51-66` | vérifié |
| I5 | `get_source_files` et `get_project_stats` sont appelés depuis 4 endroits : `project.ts`, `SegmentGrid`, `useAppHandlers:64,212`, `ProjectList`. C'est le bon cas pour `useQuery` + invalidation sur événement. | — | vérifié |
| I6 | Les requêtes React Query `qa-report` et `tm-suggestions` ne sont jamais invalidées après une écriture. `qa-check` utilise `activeSegmentId` sans le mettre dans sa clé (déjà relevé dans S-U6). | `QAPanel.tsx:203-221`, `TMPanel.tsx:61` | vérifié |
| I7 | `loadSettings` enchaîne 11 à 13 `await store.get` successifs au démarrage. | `settings.ts:73-107` | vérifié |

## 6. Rendu WebKitGTK, bundle et polices

| # | Constat | Où | Impact | Statut |
|---|---|---|---|---|
| C1 | Cartes de projet `backdrop-blur-sm` + ombre + transition, posées sur une grille en dégradé portée par le conteneur qui défile : un nouveau flou par carte à chaque frame de défilement. | `ProjectList.tsx:153`, `:175`, `:203`, `index.css` `.observatory-grid` | élevé (écran Bibliothèque) | vérifié, coût à mesurer |
| C2 | `backdrop-blur` sur des zones fixes sans rien dessous : un calque et un flou pour un effet visuel quasi nul. | `ModeNavigation.tsx:40`, `InspectorRail.tsx:82`, `PilotComparisonWorkspace.tsx:100` | moyen | vérifié |
| C3 | Starfield du `body` (8 `radial-gradient`, `background-attachment: fixed`) entièrement masqué par `AppShell` (`bg-background` opaque) : peint pour rien. | `index.css` (`.dark body`, `body:not(.dark *)`), `AppShell.tsx:13` | faible | vérifié |
| C4 | `text-wrap: pretty` global (voir G4). | `index.css` | à mesurer | hypothèse |
| C5 | Un seul `lazy()`. Pilot, Patch et tous les gros dialogues sont importés et montés au démarrage. Aucun `manualChunks`. | `App.tsx:27-32`, `AppDialogs.tsx:12-22`, `vite.config.ts` | moyen (démarrage, mémoire) | vérifié |
| C6 | `@fontsource-variable/noto-sans-jp` complet : 124 `@font-face` (environ 105 Ko de CSS), 5,3 Mo de woff2 dans le binaire. Il n'y a pas de repli sur la police CJK du système. | `index.css:5` | moyen | vérifié |
| C7 | La barre de progression anime `left` (un layout par frame) ; il faudrait animer `transform`. | `AppToolbar.tsx:131` | faible | vérifié |
| C8 | Regex des codes clonée deux fois par cellule ; regex des termes reconstruite à chaque appel. | `constants.ts:89`, `highlight-utils.tsx:22`, `:47-49` | faible | vérifié |

## 7. Doublons (classés par gain)

| # | Doublon | Occurrences | Abstraction proposée | Gain |
|---|---|---|---|---|
| D1 | `startTranslation` et `startTranslateAll` sont identiques sauf la commande et `onCooling`. `err instanceof Error ? … : String(err)` apparaît 3 fois, alors que `errorMessage()` existe déjà dans `pilot.ts:41`. | `llm.ts:264-326`, `:328-398` | `runTranslation(command, args, opts)` + `errorMessage` dans `lib/` | ~55 lignes |
| D2 | Stores de dialogue « ouvrir → charger → erreur → appliquer » quasi identiques ; mêmes blocs spinner et erreur ; même pied de dialogue avec `Loader2`. | `resync.ts`, `coverage.ts`, `textFit.ts` ; `ResyncDialog`, `TextFitDialog`, `TextCoverageDialog`, `PackExportDialog`, `PackImportWizard` | `createDialogStore<T>()`, `<DialogStatus>`, `<BusyButton>` | ~60 lignes |
| D3 | Abonnement aux événements Tauri : `listen` + `unlisten.then` répétés ; noms d'événements en dur ; `applySegmentUpdates` recopié dans la recherche ; repli rAF/setTimeout écrit deux fois. | `SegmentGrid.tsx:244-285`, `GlobalSearchResults.tsx:72-99`, `TerminologyWorkspace.tsx:105-127`, `TermTranslateDialog.tsx:68-103`, `pilot.ts:91-116` | hook `useTauriEvent`, objet `EVENTS`, `scheduleFrame/cancelFrame` | ~40 lignes |
| D4 | Panneaux de l'inspecteur : même en-tête et mêmes états vides ; états plein écran de la grille et de la recherche qui pourraient utiliser `WorkspaceState`, qui existe déjà. | `TMPanel`, `QAPanel`, `TerminologyInspector`, `TextFitPreviewPanel` ; `SegmentGrid.tsx:397-453`, `GlobalSearchResults.tsx:248-272` | `<InspectorPanel>`, `<PanelNotice>` | ~40 lignes |
| D5 | Classes CSS par colonne écrites deux fois (en-tête, cellules) ; largeurs recopiées dans la recherche ; `size` des colonnes ignoré. | `SegmentGrid.tsx:466-474`, `:564-578` ; `TerminologyTable.tsx:252-286` ; `GlobalSearchResults.tsx:207`, `:403-433` | `columnDef.meta.className` + constante de largeurs | ~30 lignes |
| D6 | `PackExportDialog` recopie `useExportToFile` ; le filtre « Hoshi2Star pack » est dupliqué. | `PackExportDialog.tsx:45-78`, `hooks/useExportToFile.ts:33-57`, `AppToolbar.tsx:213` | paramètre `successKey` + constante `H2S_FILTER` | ~25 lignes |
| D7 | Tests : `QueryClient` créé à la main 5 fois. | `QAPanel.test.tsx`, `TerminologyInspector.test.tsx`, `TerminologyWorkspace.test.tsx`, `TextFitPreviewPanel.test.tsx` | `renderWithQuery()` | ~25 lignes |
| D8 | Contrôle « aucun modèle configuré » répété 6 fois, avec des niveaux de toast différents. | `SegmentGrid.tsx:313,335,504`, `GlobalSearchResults.tsx:167`, `useAppHandlers.ts:206,232` | `ensureModelConfigured()` | ~20 lignes |
| D9 | Types de terminologie écrits en ligne ; union de portée redéfinie 6 fois. | `features/terminology/api.ts:83,113-138`, `TermTranslateDialog.tsx`, `TerminologyToolbar.tsx`, `terminologyUiStore.ts:4` | types dans `lib/types.ts` | ~20 lignes |
| D10 | `reloadSourceFiles` + `reloadProjectStats` de la grille refont `refreshProjectData`. | `SegmentGrid.tsx:125-142`, `project.ts:160-171` | réutiliser le store | ~18 lignes |
| D11 | Pastille de statut du pied de grille recopiée de `StatusBadge`. | `SegmentGrid.tsx:617-635`, `columns.tsx:37-55` | réutiliser `StatusBadge` | faible |

Total estimé : environ 330 lignes.

## 8. Code mort

- Prop `highlightPlaceholders` passée par `App.tsx:150`, puis neutralisée par `void` (`SegmentGrid.tsx:34`, `:55-57`).
- Sélecteurs et helpers jamais importés :
  - `useUpdaterStatus` (`updater.ts:143`) ;
  - `useSearchResults` et `useSearchStatus` (`search.ts:103-104`) ;
  - `useActiveFileId` (`editor.ts:44`) ;
  - `useCoverageOpen` (`coverage.ts:31`) ;
  - `useActiveProjectStats` (`project.ts:76`) ;
  - `getProjectLangPair` (`project.ts:89`) ;
  - `useRequestMetrics` et `usePipelineMetrics` (`llm.ts:443-444`).
- Dans l'état : `requestMetrics`, `pipelineMetrics` et `activeProjectStats` (voir R4 et I1).
- i18n : `llmModal.*` (9 clés) n'est référencé nulle part (vérifié). Une douzaine d'autres clés sont probablement inutilisées (hypothèse : la construction dynamique de clés n'est pas exclue).
- Dépendances : toutes sont importées. `next-themes` n'est utilisé que par `ui/sonner.tsx`, sans aucun `ThemeProvider` (voir F3).

## 9. Défauts fonctionnels relevés au passage (hors performance)

- **F1.** Seuils de score QA différents à 4 endroits (vérifié) :
  - anneau du panneau QA : 100/75 (`QAPanel.tsx:123-127`) ;
  - colonne de la grille : 80/50 (`columns.tsx:258-262`) ;
  - recherche : < 100 (`GlobalSearchResults.tsx:437-441`) ;
  - filtres de la grille : < 100 et < 70 (`SegmentGrid.tsx:182,188`).
  Un même segment peut donc s'afficher en vert dans la grille et en jaune dans l'inspecteur.
- **F2.** `handleTranslateAll` affiche `toasts.exportError` en cas d'échec (`useAppHandlers.ts:218`) : le message parle d'export (vérifié).
- **F3.** Sans `ThemeProvider`, les toasts Sonner suivent probablement le thème système au lieu du thème choisi dans l'app (hypothèse).
- **F4.** Paire de langues reconstruite à la main, avec des replis différents : `"ja-fr"` dans `AppToolbar.tsx:222`, `:442`, `:453` ; `"ja-en"` dans `PackImportWizard.tsx:85`. `getProjectLangPair` n'est jamais appelée (vérifié).
- **F5.** `formatDuration` défini deux fois avec des unités différentes : secondes dans `lib/format.ts:2`, millisecondes dans `PilotComparisonWorkspace.tsx:602` (vérifié).
- **F6.** Textes en dur hors i18n : `relativeDate` en anglais (`lib/format.ts:26-35`) ; `aria-label` en français dans `ScanProgress.tsx:43`.
- **F7.** `TextFitDialog.tsx:143` appelle `setActiveSegment(id)` sans texte : QA et TM reçoivent `null`.

## 10. Déjà connu

Ces points figurent dans `docs/audit/2026-10-analyse-complete.md` :

- **S-U3** : saisie perdue au démontage de la cellule (lié à G3 et G6) ;
- **S-U6** : clé QA sans `segmentId`, neuf sélecteurs morts, composants de plus de 600 lignes ;
- **S-U5** : erreurs avalées.

Ils restent présents dans le code actuel. Le présent audit ajoute le diagnostic de performance, les re-rendus globaux, l'IPC redondant et la liste des doublons.

## 11. Plan proposé (à valider avant toute implémentation)

Chaque lot fait l'objet d'une branche et d'une PR, avec le gate complet en session cloud.

| Lot | Contenu | Critère de sortie |
|---|---|---|
| **P1 — Défilement de la grille** | Étapes 1 à 7 du §3.3, chacune mesurée. Sauvegarde au démontage (S-U3). | Sur le fichier de 12 423 segments, en build release : défilement fluide du haut en bas, hauteur totale stable (variation < 5 %), rendus de la grille réduits (cible chiffrée après la mesure initiale). Tests Vitest de la grille verts, plus un test de sauvegarde au démontage. |
| **P2 — Re-rendus globaux** | R1 à R5, R7 : sélecteurs fins, `QAPanel` autonome, suppression des métriques mortes. | Profil : un clic de ligne ou un événement LLM ne re-rend plus `App`. Tests verts. |
| **P3 — IPC** | I1 à I5 : fichiers et stats sous React Query avec invalidation sur événement, plus de rafraîchissement en double, pas de rechargement après `completed`. | Un seul `get_source_files` / `get_project_stats` par événement (compteur IPC). Tests verts. |
| **P4 — CSS et démarrage** | C1 à C3, C5 à C7. | Rendu visuel validé par l'utilisateur ; bundle principal réduit (taille avant/après). |
| **P5 — Doublons et code mort** | D1 à D11, §8. | Diff net négatif, aucun changement de comportement, tests verts. |
| **P6 — Défauts fonctionnels** | F1 à F7, chacun avec un test. | Tests de régression verts. |

P1 répond directement à la plainte de l'utilisateur. P2 et P3 réduisent le travail de fond pendant une traduction. P5 est le plus long mais le moins risqué : à faire après P1 à P3, pour ne pas mêler refonte et optimisation dans une même PR.
