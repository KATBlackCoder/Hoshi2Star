# Journal — 2026-06-03 — F3-10 Glossary prompt après ouverture de projet

**Phase** : F3
**Durée estimée** : 45–60 min
**Statut** : ✅ Complété

---

## Ce qui a été fait

- Ajout de `pendingGlossaryExtract` (string | null) et `isExtractingGlossary` (boolean) dans `ProjectState` (Zustand)
- Ajout des actions `setPendingGlossaryExtract` et `setExtractingGlossary`
- Dans le thunk `openProject` : si `!wasRestored` → `setPendingGlossaryExtract(project.id)` après `setActiveProject`
- Ajout des sélecteurs exportés `usePendingGlossaryExtract` et `useIsExtractingGlossary`
- Dans `App.tsx` : `useEffect` écoutant `h2s://glossary/extraction-done` → désactive `isExtractingGlossary`, toast succès (count) ou erreur
- Handlers `handleGlossaryConfirm` (invoke `extract_glossary_terms`, active la bannière) et `handleGlossaryDecline` (ferme le dialog)
- `AlertDialog` contrôlé par `pendingGlossaryExtract !== null` — fermeture uniquement via boutons (pas clic extérieur)
- Bannière `h-7` non-bloquante entre toolbar et `ResizablePanelGroup` — disparaît automatiquement à la fin
- Bouton Traduire : `disabled={isTranslating || isExtractingGlossary}`, label "Glossaire en cours…" pendant l'extraction
- Clés i18n `glossaryPrompt.*` ajoutées en EN et FR (titre, description, oui/non, extracting, translationBlocked, extractDone avec interpolation count, extractDone_zero, extractError)

## Fichiers créés

- `docs/journal/2026-06-03-f3-10-glossary-prompt.md` (ce fichier)

## Fichiers modifiés

- `src/stores/project.ts` — 2 nouveaux champs + actions + sélecteurs + setPendingGlossaryExtract dans openProject
- `src/App.tsx` — imports listen/AlertDialog/BookOpen, listener Tauri, handlers, AlertDialog JSX, bannière, Toolbar isExtractingGlossary
- `src/locales/en.json` — section `glossaryPrompt`
- `src/locales/fr.json` — section `glossaryPrompt`
- `ROADMAP.md` — item coché F3-10 sous Polissage UI
- `CHANGELOG.md` — 3 entrées Added sous [Unreleased]

## Fichiers supprimés

- Aucun

## Dépendances ajoutées

- Aucune (`@tauri-apps/api/event` déjà présent, `AlertDialog` déjà installé via shadcn)

## Décisions prises

- Dialog non dismissible par clic extérieur (`open` contrôlé, pas de `onOpenChange`) — l'utilisateur doit choisir explicitement oui ou non
- `extract_glossary_terms` est fire-and-forget côté Rust : l'invoke retourne immédiatement, l'event `h2s://glossary/extraction-done` signale la fin
- `setPendingGlossaryExtract` et `setExtractingGlossary` récupérés via `useProjectStore.getState()` dans `App` (stable ref, pas de re-render)

## Problèmes rencontrés

- `pnpm lint` échoue à cause de l'absence de `eslint.config.js` (ESLint v9) — problème pré-existant non lié à F3-10, typecheck passe avec 0 erreur

## Tâches ROADMAP cochées

- [x] Glossary prompt on project open — AlertDialog après ouverture neuve, bannière non-bloquante, bouton Traduire désactivé jusqu'à fin extraction

## Prochaine session

- Beta privée F3 : recrutement testeurs, feedback form
- Ou démarrage F4 : Wolf RPG parser

---
*Généré par Claude Code — Hoshi2Star*
