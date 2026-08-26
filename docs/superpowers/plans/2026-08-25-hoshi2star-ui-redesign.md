# Hoshi2Star Personal Workspace Redesign Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transformer l'interface Tenmon actuelle en espace personnel clair et évolutif pour les modes Bibliothèque, Patch, Player et Images, sans perdre la densité utile de l'éditeur CAT.

**Architecture:** Le redesign conserve React, shadcn, Tailwind et les stores existants. Une app shell légère sépare navigation globale, contexte du projet et outils de travail; les panneaux avancés deviennent repliables et mémorisés. Le redesign commence après la fondation langues/providers, se valide par tests composants, MCP en debug, captures comparatives et tests Tauri E2E.

**Tech Stack:** React 19, TypeScript strict, Tailwind CSS v4, shadcn/Radix, Zustand, TanStack Table/Virtual, Tauri v2, Vitest/Testing Library, MCP debug bridge, tauri-driver/WebDriver sur Linux et Windows.

---

## Position dans la roadmap

Exécuter ce plan après `2026-08-25-hoshi2star-lean-multilingual-providers.md` Tasks 1–10 et avant le Player Mode. Les routes Player et Images peuvent afficher un état « bientôt disponible » jusqu'à leur implémentation, mais leur présence dans la navigation verrouille l'architecture du produit.

## Audit visuel actuel

### Hiérarchie et navigation

| Avant | Après attendu |
| --- | --- |
| Barre supérieure très fine, actions et contexte mélangés | App shell avec navigation de modes, projet actif et actions contextuelles séparés |
| Écran vide utilise trois colonnes inactives | Bibliothèque centrée avec appel principal et explication des trois modes |
| Outils TM, QA et glossaire occupent toujours 25 % | Rail d'inspection repliable, onglets TM/QA/Glossaire et état mémorisé |
| Actions avancées dispersées | Menu « Outils » pour export TMX, rapport QA, debug et packs `.h2s` |

### Typographie et surfaces

| Avant | Après attendu |
| --- | --- |
| Nombreux libellés à 10–11 px et contraste faible | Texte fonctionnel à 12–14 px, contrastes vérifiés, titres équilibrés |
| Grandes surfaces plates séparées uniquement par traits | Diviseurs pour la structure, ombres transparentes pour cartes/modales |
| Compteurs dynamiques proportionnels | `tabular-nums` sur progression, durée, compteurs et scores |
| Rayons identiques sur surfaces imbriquées | Rayons concentriques : rayon extérieur = intérieur + padding |

### Interaction et mouvement

| Avant | Après attendu |
| --- | --- |
| Plusieurs icônes ont une zone visible très petite | Zones interactives 40×40 px minimum sans chevauchement |
| Transitions définies composant par composant | Tokens 120/180/240 ms et propriétés explicites uniquement |
| Modales avec état `mounted/visible` dupliqué | Primitive Dialog/Sheet partagée avec transition interruptible |
| Feedback surtout par toast | Feedback local près de l'action plus toast pour les résultats globaux |

## Principes non négociables

- Aucun `transition: all` ni classe Tailwind `transition` générique.
- Aucun ajout de Motion/Framer Motion : utiliser CSS puisque le projet n'en dépend pas.
- `active:scale-[0.96]` uniquement sur boutons tactiles; possibilité `static` pour actions sensibles.
- Zones cliquables d'au moins 40×40 px, idéalement 44×44 px.
- `text-balance` pour les titres courts, `text-pretty` pour les descriptions.
- `antialiased` à la racine et `tabular-nums` sur les valeurs dynamiques.
- Respect de `prefers-reduced-motion`.
- États loading, empty, error, disabled, focus et keyboard définis pour chaque flux.
- Pas de redesign de la table au prix de sa virtualisation ou de ses performances.

## Structure de fichiers cible

- Create: `src/components/shell/AppShell.tsx` — structure globale.
- Create: `src/components/shell/ModeNavigation.tsx` — Bibliothèque/Patch/Player/Images.
- Create: `src/components/shell/ContextBar.tsx` — projet, moteur, langues, provider.
- Create: `src/components/shell/InspectorRail.tsx` — TM/QA/Glossaire repliables.
- Create: `src/components/library/LibraryHome.tsx` — accueil et projets récents.
- Create: `src/components/library/ProjectCard.tsx` — carte de reprise.
- Create: `src/components/library/ModeCard.tsx` — explication des modes.
- Create: `src/components/common/EmptyState.tsx` — empty state réutilisable.
- Create: `src/components/common/StatusChip.tsx` — états cohérents.
- Create: `src/stores/ui.ts` — mode, inspector, densité et préférence mémorisée.
- Modify: `src/App.tsx`, `src/index.css`, `src/App.css`.
- Modify: `src/components/AppToolbar.tsx`, `AppDialogs.tsx`, `AboutModal.tsx`.
- Modify: `src/components/editor/ProjectList.tsx`, `FileTree.tsx`, `SegmentGrid.tsx`, `TMPanel.tsx`, `QAPanel.tsx`, `GlossaryPanel.tsx`.
- Modify: `src/components/settings/SettingsModal.tsx`.
- Modify: `src/components/ui/button.tsx`, `badge.tsx`, `input.tsx`, `select.tsx`, `alert-dialog.tsx`, `resizable.tsx`.
- Create: `e2e-tests/specs/redesign.spec.ts` — parcours shell et clavier.
- Create: `docs/design/redesign-acceptance.md` — matrice visuelle.

---

### Task 1: Verrouiller les tokens visuels et d'interaction

**Files:**
- Modify: `src/index.css`
- Modify: `src/App.css`
- Modify: `src/components/ui/button.tsx`
- Modify: `src/components/ui/input.tsx`
- Test: `src/components/ui/button.test.tsx`

- [ ] **Step 1: Écrire les tests de primitive Button**

Tester que le bouton normal possède une zone minimale, une transition ciblée et l'échelle pressée; tester que `<Button static>` retire l'échelle.

```tsx
render(<Button>Ouvrir</Button>);
expect(screen.getByRole("button")).toHaveClass("min-h-10", "active:scale-[0.96]");

render(<Button static>Supprimer</Button>);
expect(screen.getByRole("button")).not.toHaveClass("active:scale-[0.96]");
```

- [ ] **Step 2: Définir les tokens CSS**

Ajouter sous `:root` et `.dark` des variables de surface, ombre, focus et mouvement :

```css
:root {
  --surface-raised: color-mix(in srgb, var(--background) 94%, black 6%);
  --shadow-surface: 0 0 0 1px rgb(0 0 0 / 0.06), 0 1px 2px -1px rgb(0 0 0 / 0.06), 0 2px 4px rgb(0 0 0 / 0.04);
  --shadow-surface-hover: 0 0 0 1px rgb(0 0 0 / 0.08), 0 1px 2px -1px rgb(0 0 0 / 0.08), 0 2px 4px rgb(0 0 0 / 0.06);
  --duration-fast: 120ms;
  --duration-normal: 180ms;
  --duration-slow: 240ms;
}

.dark {
  --surface-raised: color-mix(in srgb, var(--background) 94%, white 6%);
  --shadow-surface: 0 0 0 1px rgb(255 255 255 / 0.08);
  --shadow-surface-hover: 0 0 0 1px rgb(255 255 255 / 0.13);
}
```

- [ ] **Step 3: Ajouter typographie et réduction de mouvement**

```css
html {
  -webkit-font-smoothing: antialiased;
  -moz-osx-font-smoothing: grayscale;
}

@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    scroll-behavior: auto !important;
    animation-duration: 0.01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: 0.01ms !important;
  }
}
```

- [ ] **Step 4: Mettre à jour Button/Input sans casser shadcn**

Button utilise `min-h-10`, focus visible, transition explicite et `active:scale-[0.96]`. Input conserve une bordure accessible et une hauteur minimum de 40 px.

- [ ] **Step 5: Vérifier et commit**

Run: `pnpm test -- button && pnpm typecheck && pnpm lint`

```bash
git add src/index.css src/App.css src/components/ui/button.tsx src/components/ui/input.tsx src/components/ui/button.test.tsx
git commit -m "style: establish accessible interaction tokens"
```

---

### Task 2: Ajouter l'état UI et la navigation par modes

**Files:**
- Create: `src/stores/ui.ts`
- Create: `src/stores/ui.test.ts`
- Create: `src/components/shell/ModeNavigation.tsx`
- Test: `src/components/shell/ModeNavigation.test.tsx`

- [ ] **Step 1: Définir l'état minimal**

```ts
export type AppMode = "library" | "patch" | "player" | "images";
export type InspectorTab = "tm" | "qa" | "glossary";

interface UiState {
  mode: AppMode;
  inspectorOpen: boolean;
  inspectorTab: InspectorTab;
  setMode: (mode: AppMode) => void;
  toggleInspector: () => void;
  setInspectorTab: (tab: InspectorTab) => void;
}
```

Ne persister que `inspectorOpen` et `inspectorTab`; le mode actif repart sur Bibliothèque au lancement.

- [ ] **Step 2: Construire la navigation**

Utiliser quatre boutons avec texte et icône. `aria-current="page"` marque le mode actif. Player et Images restent cliquables mais montrent un état explicatif tant qu'ils ne sont pas implémentés.

- [ ] **Step 3: Ajouter navigation clavier**

Flèches gauche/droite changent de mode dans le groupe, Entrée active, et chaque cible mesure au moins 40 px.

- [ ] **Step 4: Vérifier et commit**

Run: `pnpm test -- ui ModeNavigation`

```bash
git add src/stores/ui.ts src/stores/ui.test.ts src/components/shell/ModeNavigation.tsx src/components/shell/ModeNavigation.test.tsx
git commit -m "feat(ui): add personal workspace mode navigation"
```

---

### Task 3: Construire l'AppShell et la barre de contexte

**Files:**
- Create: `src/components/shell/AppShell.tsx`
- Create: `src/components/shell/ContextBar.tsx`
- Modify: `src/App.tsx`
- Modify: `src/components/AppToolbar.tsx`
- Test: `src/components/shell/AppShell.test.tsx`

- [ ] **Step 1: Écrire le test de structure**

Vérifier la présence de `banner`, `navigation`, `main` et d'une zone d'inspection optionnelle. Vérifier que le projet actif montre moteur, `JA → FR` et provider.

- [ ] **Step 2: Créer l'AppShell**

```tsx
export function AppShell({ children, inspector }: AppShellProps) {
  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground antialiased">
      <ModeNavigation />
      <ContextBar />
      <div className="min-h-0 flex-1">{children}</div>
      {inspector}
    </div>
  );
}
```

- [ ] **Step 3: Réduire AppToolbar**

Déplacer identité, projet et mode hors d'`AppToolbar`. Il ne garde que les actions du mode Patch : traduire, tout traduire, exporter et menu Outils.

- [ ] **Step 4: Vérifier et commit**

Run: `pnpm test -- AppShell AppToolbar && pnpm typecheck`

```bash
git add src/App.tsx src/components/AppToolbar.tsx src/components/shell
git commit -m "feat(ui): introduce app shell and project context bar"
```

---

### Task 4: Redessiner la bibliothèque et l'état vide

**Files:**
- Create: `src/components/common/EmptyState.tsx`
- Create: `src/components/library/LibraryHome.tsx`
- Create: `src/components/library/ProjectCard.tsx`
- Create: `src/components/library/ModeCard.tsx`
- Modify: `src/components/editor/ProjectList.tsx`
- Test: `src/components/library/LibraryHome.test.tsx`

- [ ] **Step 1: Écrire les scénarios**

Tester bibliothèque vide, projets récents, reprise d'un projet, suppression et raccourcis vers Patch/Player/Images.

- [ ] **Step 2: Concevoir l'accueil**

L'état vide ne rend plus FileTree et Inspector. Afficher un titre `text-balance`, une description `text-pretty`, un bouton principal « Ouvrir un jeu », puis trois cartes de mode compactes.

- [ ] **Step 3: Concevoir ProjectCard**

La carte montre titre, moteur, paire de langues, progression et date. Progression et date utilisent `tabular-nums`; les actions secondaires apparaissent au focus autant qu'au hover.

- [ ] **Step 4: Vérifier les rayons**

Carte extérieure `rounded-2xl p-2`, contenu intérieur `rounded-lg`; ombre de surface au repos et au hover, aucune bordure décorative redondante.

- [ ] **Step 5: Vérifier et commit**

Run: `pnpm test -- LibraryHome ProjectList && pnpm lint`

```bash
git add src/components/common/EmptyState.tsx src/components/library src/components/editor/ProjectList.tsx
git commit -m "feat(ui): redesign personal game library"
```

---

### Task 5: Rendre l'espace Patch progressif et repliable

**Files:**
- Create: `src/components/shell/InspectorRail.tsx`
- Modify: `src/components/editor/TMPanel.tsx`
- Modify: `src/components/editor/QAPanel.tsx`
- Modify: `src/components/editor/GlossaryPanel.tsx`
- Modify: `src/components/editor/FileTree.tsx`
- Modify: `src/App.tsx`
- Test: `src/components/shell/InspectorRail.test.tsx`

- [ ] **Step 1: Écrire les tests du rail**

Tester ouverture/fermeture, sélection des trois onglets, conservation du panneau actif et accessibilité clavier.

- [ ] **Step 2: Remplacer les trois panneaux verticaux**

Afficher un seul panneau à la fois dans un rail de 320–440 px. Un segment sélectionné ouvre automatiquement QA uniquement si une erreur critique apparaît; sinon ne pas voler le focus.

- [ ] **Step 3: Rendre FileTree repliable**

Le panneau de fichiers garde sa largeur redimensionnable et peut se replier vers une barre de 44 px avec bouton explicite.

- [ ] **Step 4: Vérifier et commit**

Run: `pnpm test -- InspectorRail TMPanel QAPanel GlossaryPanel FileTree`

```bash
git add src/App.tsx src/components/shell/InspectorRail.tsx src/components/editor
git commit -m "feat(ui): add progressive patch workspace panels"
```

---

### Task 6: Améliorer la grille sans perdre sa densité

**Files:**
- Modify: `src/components/editor/SegmentGrid.tsx`
- Modify: `src/features/editor/columns.tsx`
- Create: `src/components/editor/GridDensityToggle.tsx`
- Test: `src/components/editor/SegmentGrid.test.tsx`

- [ ] **Step 1: Ajouter deux densités**

`compact` conserve la densité actuelle corrigée; `comfortable` augmente hauteur et padding. La préférence reste dans `ui.ts`.

- [ ] **Step 2: Renforcer la hiérarchie**

En-tête sticky, colonne source légèrement teintée, cible clairement éditable, statuts via `StatusChip`, QA avec score tabulaire et texte accessible en plus de la couleur.

- [ ] **Step 3: Préserver virtualisation et focus**

Le nombre de lignes DOM doit rester borné par TanStack Virtual. Après sauvegarde ou traduction, conserver la ligne active et la position de scroll.

- [ ] **Step 4: Vérifier et commit**

Run: `pnpm test -- SegmentGrid columns && pnpm typecheck`

```bash
git add src/components/editor/SegmentGrid.tsx src/components/editor/GridDensityToggle.tsx src/features/editor/columns.tsx src/stores/ui.ts
git commit -m "feat(ui): improve translation grid hierarchy and density"
```

---

### Task 7: Redessiner les paramètres providers et langues

**Files:**
- Modify: `src/components/settings/SettingsModal.tsx`
- Create: `src/components/settings/ProviderSettings.tsx`
- Create: `src/components/settings/LanguageSettings.tsx`
- Create: `src/components/settings/AppearanceSettings.tsx`
- Test: `src/components/settings/SettingsModal.test.tsx`

- [ ] **Step 1: Séparer les sections**

Utiliser une navigation latérale ou des onglets accessibles : Modèles, Langues, Apparence. Chaque section possède titre équilibré, description courte et validation locale.

- [ ] **Step 2: Améliorer le test de connexion**

Le bouton affiche success/error dans la section, avec icône contextuelle animée par CSS `opacity`, `scale 0.25→1` et `blur 4px→0`; le toast reste secondaire.

- [ ] **Step 3: Unifier les modales**

Extraire la transition montée/visible actuellement dupliquée entre Settings et About dans une primitive partagée. Transition 180 ms interruptible, sortie 120–150 ms, aucune animation initiale inutile.

- [ ] **Step 4: Vérifier et commit**

Run: `pnpm test -- SettingsModal ProviderSettings && pnpm lint`

```bash
git add src/components/settings src/components/settings/SettingsModal.tsx src/components/AboutModal.tsx
git commit -m "feat(ui): redesign settings around providers and languages"
```

---

### Task 8: Ajouter feedback, focus et responsive desktop

**Files:**
- Modify: `src/index.css`
- Modify: `src/components/AppToolbar.tsx`
- Modify: `src/components/UpdateBadge.tsx`
- Modify: `src/components/editor/GlobalSearchResults.tsx`
- Modify: `src/components/editor/SegmentSearchBar.tsx`
- Test: composants concernés.

- [ ] **Step 1: Audit zones interactives**

Run: `rg -n 'h-[45678]|w-[45678]|size-[45678]|text-\[10px\]|text-\[11px\]' src/components`

Inspecter chaque résultat interactif et étendre la zone sans collision.

- [ ] **Step 2: Audit transitions**

Run: `rg -n 'transition-all|\btransition\b' src`

Remplacer chaque transition générique par `transition-transform`, `transition-colors` ou `transition-[opacity,filter,scale]` selon la propriété réellement modifiée.

- [ ] **Step 3: Définir trois largeurs desktop**

- ≥ 1440 px : FileTree + grille + Inspector.
- 1024–1439 px : FileTree compact, Inspector en sheet.
- 800–1023 px : navigation condensée, grille principale, panneaux secondaires en sheet.

Hoshi2Star reste desktop; aucune adaptation mobile n'est requise.

- [ ] **Step 4: Vérifier au clavier**

Tab traverse uniquement les cibles visibles, Escape ferme sheet/modal, focus revient au déclencheur et aucune action hover-only n'est inaccessible.

- [ ] **Step 5: Commit**

```bash
git add src/index.css src/components
git commit -m "fix(ui): improve focus motion and desktop responsiveness"
```

---

### Task 9: Valider le redesign par MCP et E2E Tauri

**Files:**
- Create: `e2e-tests/package.json`
- Create: `e2e-tests/wdio.conf.ts`
- Create: `e2e-tests/specs/redesign.spec.ts`
- Modify: `src-tauri/capabilities/default.json`
- Create: `docs/design/redesign-acceptance.md`

- [ ] **Step 1: Garder MCP uniquement en debug**

Vérifier que `tauri-plugin-mcp-bridge` reste sous `#[cfg(debug_assertions)]`. La capability MCP ne doit pas élargir les permissions des builds release; documenter son usage de validation visuelle.

- [ ] **Step 2: Créer le parcours E2E**

Le test démarre l'app, vérifie Bibliothèque, ouvre les paramètres, change de provider sans secret, revient, active Patch, replie FileTree, ouvre l'Inspector et sélectionne QA.

- [ ] **Step 3: Faire les checkpoints MCP**

Pour chaque checkpoint, utiliser le bridge debug afin d'inspecter les éléments visibles et l'état de fenêtre :

1. bibliothèque vide sombre et claire;
2. bibliothèque avec trois projets;
3. Patch avec fichier et segment sélectionnés;
4. Inspector ouvert sur chaque onglet;
5. paramètres provider local puis cloud;
6. largeur 1024 px;
7. navigation clavier et focus visible.

- [ ] **Step 4: Captures comparatives**

Enregistrer sous `docs/screenshots/redesign/` une capture dark/light pour Bibliothèque, Patch et Paramètres. Chaque paire est listée dans `redesign-acceptance.md` avec résolution et état de données.

- [ ] **Step 5: Vérifier E2E**

Run Linux: `xvfb-run pnpm --dir e2e-tests test`

Expected: parcours complet réussi. Windows est exécuté en CI avec Edge Driver.

- [ ] **Step 6: Commit**

```bash
git add e2e-tests src-tauri/capabilities/default.json docs/design/redesign-acceptance.md docs/screenshots/redesign
git commit -m "test(ui): validate redesign with MCP and Tauri E2E"
```

---

### Task 10: Gate final et documentation

**Files:**
- Modify: `README.md`
- Modify: `README.fr.md`
- Modify: `docs/architecture.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Gate complet**

```bash
pnpm typecheck
pnpm test
pnpm lint
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

- [ ] **Step 2: Vérifier les contraintes visuelles**

Run: `rg -n 'transition-all|will-change:\s*all' src`

Expected: aucun résultat.

Vérifier manuellement : contrastes dark/light, focus visible, 40 px minimum, reduced motion, nombres tabulaires, rayons concentriques et aucune animation au chargement initial.

- [ ] **Step 3: Documenter l'app shell**

Ajouter à `docs/architecture.md` les quatre modes, `ui.ts`, AppShell, ContextBar et InspectorRail. README utilise les nouvelles captures sans promettre Player/Images avant leur livraison.

- [ ] **Step 4: Commit**

```bash
git add README.md README.fr.md docs/architecture.md CHANGELOG.md
git commit -m "docs: describe the personal workspace redesign"
```

---

## Critères de sortie

- L'écran de démarrage est une bibliothèque, pas un éditeur vide en trois colonnes.
- Les modes Patch, Player et Images sont compréhensibles dès l'accueil.
- Le mode Patch conserve toutes ses capacités et sa virtualisation.
- FileTree et Inspector sont repliables et mémorisés.
- La configuration multi-provider reste simple pour un utilisateur non technique.
- Toutes les actions sont utilisables au clavier et disposent d'un focus visible.
- Les cibles interactives font au moins 40×40 px.
- Les animations sont ciblées, interruptibles et respectent reduced motion.
- MCP est utilisé pour les checkpoints en debug; les tests Vitest/Rust/E2E restent la validation reproductible.
- Les captures dark/light sont validées avant de reprendre Player Mode.

## Self-review

- Le plan sépare fondation fonctionnelle et redesign pour faciliter les régressions.
- Aucun framework d'animation supplémentaire n'est ajouté.
- La densité CAT utile n'est pas remplacée par une interface de démonstration trop aérée.
- La navigation prépare Player et Images sans implémenter leurs fonctionnalités prématurément.
- Le MCP complète les tests, il ne les remplace pas.
- Les changements sont vérifiables par tests composants, E2E, clavier et captures comparatives.
