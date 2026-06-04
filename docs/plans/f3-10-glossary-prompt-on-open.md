# Plan F3-10 — Prompt extraction glossaire après ouverture de projet

## Objectif

Après l'extraction d'un nouveau projet (`was_restored: false`), afficher une
`AlertDialog` demandant si l'utilisateur veut lancer l'extraction automatique
du glossaire. Si oui : afficher une bannière non-bloquante sous la toolbar pendant
l'extraction, et bloquer le bouton Traduire jusqu'à la fin.

**Comportement attendu :**
1. `open_project` retourne `was_restored: false` → `AlertDialog` apparaît
2. "Non" → dialog fermée, rien d'autre
3. "Oui" → `extract_glossary_terms` lancé (fire-and-forget), bannière visible,
   bouton Traduire désactivé avec label explicatif
4. Event `h2s://glossary/extraction-done` reçu → bannière disparaît, toast de
   confirmation, bouton Traduire réactivé

## Statut : [ ] En cours

## Prérequis

- F3-03 complet (glossaire CRUD + command `extract_glossary_terms` + event Tauri) — `[x]` Fait
- `extract_glossary_terms` déjà async fire-and-forget, émet `h2s://glossary/extraction-done` — `[x]` Fait
- `openProject` thunk retourne déjà `{ project, wasRestored }` — `[x]` Fait
- shadcn `AlertDialog` déjà installé (`src/components/ui/alert-dialog.tsx`) — `[x]` Fait
- Pattern `listen` / `UnlistenFn` établi dans `stores/llm.ts` — `[x]` Fait

## Estimation

4 steps · ~45–60 min total

## Périmètre — Backend

**Aucun changement Rust.** Toute l'infrastructure backend est déjà en place :
- `commands/glossary.rs::extract_glossary_terms` → lance tokio::spawn, retourne `Ok(())`
- Émet `h2s://glossary/extraction-done` avec `{ projectId, terms, error }`

---

## Steps

---

### Step 1 — `stores/project.ts` — état extraction glossaire

**Objectif :** Ajouter deux flags dans le store projet pour piloter le dialog et
la bannière depuis `App.tsx`, sans prop-drilling.

**Fichiers touchés :**
- `src/stores/project.ts` ← modifier

**Dépend de :** *(aucun)*

Tâches :
- [ ] Ajouter dans l'interface `ProjectState` :
  ```ts
  /** project.id en attente de réponse utilisateur (oui/non) — null si aucun */
  pendingGlossaryExtract: string | null;
  /** true pendant que extract_glossary_terms tourne en arrière-plan */
  isExtractingGlossary: boolean;
  // Actions
  setPendingGlossaryExtract: (id: string | null) => void;
  setExtractingGlossary: (v: boolean) => void;
  ```
- [ ] Initialiser les deux champs à `null` / `false` dans le store Zustand
- [ ] Implémenter les deux actions (simples `set()`)
- [ ] Dans le thunk `openProject`, après `setActiveProject` :
  ```ts
  if (!wasRestored) {
    useProjectStore.getState().setPendingGlossaryExtract(project.id);
  }
  ```
  > Le thunk `openProject` retourne déjà `wasRestored` — pas de changement de signature.

- [ ] Ajouter deux sélecteurs exportés en bas du fichier :
  ```ts
  export const usePendingGlossaryExtract = () =>
    useProjectStore((s) => s.pendingGlossaryExtract);
  export const useIsExtractingGlossary = () =>
    useProjectStore((s) => s.isExtractingGlossary);
  ```

Test de validation :
```bash
pnpm typecheck
```
Résultat attendu : 0 erreur TypeScript

Commit message : `feat(stores): add pendingGlossaryExtract + isExtractingGlossary to project store`

---

### Step 2 — `App.tsx` — AlertDialog + bannière + listener Tauri

**Objectif :** Réagir au flag `pendingGlossaryExtract`, afficher le dialog de
confirmation, gérer la réponse, et écouter l'event de fin d'extraction.

**Fichiers touchés :**
- `src/App.tsx` ← modifier

**Dépend de :** Step 1

#### 2a — Import et listener Tauri

- [ ] Ajouter les imports manquants en tête de `App.tsx` :
  ```ts
  import { listen } from "@tauri-apps/api/event";
  import { invoke } from "@tauri-apps/api/core"; // déjà présent
  import {
    AlertDialog, AlertDialogAction, AlertDialogCancel,
    AlertDialogContent, AlertDialogDescription, AlertDialogFooter,
    AlertDialogHeader, AlertDialogTitle,
  } from "@/components/ui/alert-dialog";
  import { BookOpen, Loader2 } from "lucide-react"; // BookOpen pour la bannière
  import {
    usePendingGlossaryExtract,
    useIsExtractingGlossary,
    useProjectStore,
  } from "@/stores/project";
  import { useTranslation } from "react-i18next"; // déjà présent dans App
  ```

- [ ] Dans le composant `App`, récupérer les sélecteurs :
  ```ts
  const pendingGlossaryExtract = usePendingGlossaryExtract();
  const isExtractingGlossary = useIsExtractingGlossary();
  const { setPendingGlossaryExtract, setExtractingGlossary } =
    useProjectStore.getState();
  ```

- [ ] Ajouter un `useEffect` pour le listener `h2s://glossary/extraction-done` :
  ```ts
  useEffect(() => {
    let unlisten: (() => void) | null = null;
    listen<{ projectId: string; terms: unknown[]; error: string | null }>(
      "h2s://glossary/extraction-done",
      (event) => {
        setExtractingGlossary(false);
        if (event.payload.error) {
          toast.error(t("glossaryPrompt.extractError"));
        } else {
          const count = event.payload.terms.length;
          toast.success(t("glossaryPrompt.extractDone", { count }));
        }
      },
    ).then((fn) => { unlisten = fn; });
    return () => { unlisten?.(); };
  }, [setExtractingGlossary, t]);
  ```

#### 2b — Handler de confirmation

- [ ] Ajouter la fonction `handleGlossaryConfirm` dans `App` :
  ```ts
  async function handleGlossaryConfirm() {
    if (!pendingGlossaryExtract) return;
    const projectId = pendingGlossaryExtract;
    setPendingGlossaryExtract(null);
    setExtractingGlossary(true);
    try {
      await invoke("extract_glossary_terms", {
        projectId,
        langPair: "ja-en",
        providerConfig: providerConfig,  // depuis useLlmStore déjà en scope
      });
    } catch {
      setExtractingGlossary(false);
      toast.error(t("glossaryPrompt.extractError"));
    }
  }

  function handleGlossaryDecline() {
    setPendingGlossaryExtract(null);
  }
  ```

#### 2c — AlertDialog dans le JSX

- [ ] Ajouter après `<SettingsModal>` dans le return de `App` :
  ```tsx
  <AlertDialog open={pendingGlossaryExtract !== null}>
    <AlertDialogContent>
      <AlertDialogHeader>
        <AlertDialogTitle>{t("glossaryPrompt.title")}</AlertDialogTitle>
        <AlertDialogDescription>
          {t("glossaryPrompt.description")}
        </AlertDialogDescription>
      </AlertDialogHeader>
      <AlertDialogFooter>
        <AlertDialogCancel onClick={handleGlossaryDecline}>
          {t("glossaryPrompt.no")}
        </AlertDialogCancel>
        <AlertDialogAction onClick={() => void handleGlossaryConfirm()}>
          {t("glossaryPrompt.yes")}
        </AlertDialogAction>
      </AlertDialogFooter>
    </AlertDialogContent>
  </AlertDialog>
  ```

  > `open` est contrôlé par `pendingGlossaryExtract !== null` — pas de `onOpenChange`
  > car la fermeture n'est autorisée que via les boutons (pas le clic extérieur).

#### 2d — Bannière sous la toolbar

- [ ] Dans le return de `App`, entre `<Toolbar .../>` et `<ResizablePanelGroup ...>`,
  ajouter conditionnellement :
  ```tsx
  {isExtractingGlossary && (
    <div className="flex h-7 shrink-0 items-center gap-2 border-b bg-muted/50 px-3 text-xs text-muted-foreground">
      <Loader2 className="h-3 w-3 animate-spin shrink-0" />
      <BookOpen className="h-3 w-3 shrink-0" />
      <span>{t("glossaryPrompt.extracting")}</span>
    </div>
  )}
  ```

  > La bannière est fine (h-7), non bloquante, positionnée entre toolbar et contenu.
  > Elle disparaît automatiquement quand `isExtractingGlossary` repasse à `false`.

Test de validation :
```bash
pnpm typecheck
pnpm lint
```
Résultat attendu : 0 erreur TS, 0 erreur ESLint

Commit message : `feat(ui): glossary prompt dialog + extraction banner after project open`

---

### Step 3 — Toolbar — bloquer le bouton Traduire

**Objectif :** Empêcher de lancer une traduction pendant que le glossaire est en
cours d'extraction — sinon les termes ne sont pas encore en DB et le prompt LLM
les ignore entièrement.

**Fichiers touchés :**
- `src/App.tsx` ← modifier la fonction `Toolbar`

**Dépend de :** Step 1

> `Toolbar` est une fonction interne de `App.tsx`. Elle reçoit ses props depuis
> `App` — mais `isExtractingGlossary` vient du store, accessible directement
> depuis `Toolbar` via le sélecteur.

Tâches :
- [ ] Dans `Toolbar`, ajouter le sélecteur :
  ```ts
  const isExtractingGlossary = useIsExtractingGlossary();
  ```

- [ ] Modifier le bouton Traduire :
  ```tsx
  <Button
    ...
    disabled={isTranslating || isExtractingGlossary}
  >
    {isExtractingGlossary ? (
      <Loader2 className="h-3.5 w-3.5 animate-spin" />
    ) : isTranslating ? (
      <Loader2 className="h-3.5 w-3.5 animate-spin" />
    ) : (
      <Play className="h-3.5 w-3.5" />
    )}
    {isExtractingGlossary
      ? t("glossaryPrompt.translationBlocked")
      : isTranslating
        ? `${t("toolbar.translating")} ${progress > 0 ? `${progress}%` : ""}`
        : t("toolbar.translate")}
  </Button>
  ```

  > `translationBlocked` = label court expliquant pourquoi le bouton est grisé,
  > ex. "Glossaire en cours…" — plus clair qu'un bouton grisé silencieux.

Test de validation :
```bash
pnpm typecheck
```
Résultat attendu : 0 erreur TypeScript

Commit message : `feat(ui): disable Translate button while glossary extraction is running`

---

### Step 4 — i18n — clés dialog et bannière

**Objectif :** Ajouter les clés de traduction pour le dialog, la bannière et le
label du bouton bloqué, en anglais et français.

**Fichiers touchés :**
- `src/locales/en.json` ← ajouter section `glossaryPrompt`
- `src/locales/fr.json` ← ajouter section `glossaryPrompt`

**Dépend de :** *(peut être fait en parallèle de Step 2 et 3)*

Tâches :
- [ ] Dans `src/locales/en.json`, ajouter :
  ```json
  "glossaryPrompt": {
    "title": "Extract glossary terms?",
    "description": "The LLM can scan actor, skill, item and state names to build a glossary for consistent translations. This takes 10–30 seconds.",
    "yes": "Extract",
    "no": "Skip",
    "extracting": "Extracting glossary terms…",
    "translationBlocked": "Glossary in progress…",
    "extractDone": "Glossary ready — {{count}} terms extracted.",
    "extractDone_zero": "Glossary extraction complete — no new terms found.",
    "extractError": "Glossary extraction failed. You can retry from the Glossary panel."
  }
  ```

- [ ] Dans `src/locales/fr.json`, ajouter :
  ```json
  "glossaryPrompt": {
    "title": "Extraire le glossaire ?",
    "description": "Le LLM peut analyser les noms d'acteurs, compétences, objets et états pour construire un glossaire de traduction cohérent. Durée : 10–30 secondes.",
    "yes": "Extraire",
    "no": "Ignorer",
    "extracting": "Extraction du glossaire en cours…",
    "translationBlocked": "Glossaire en cours…",
    "extractDone": "Glossaire prêt — {{count}} termes extraits.",
    "extractDone_zero": "Extraction terminée — aucun nouveau terme trouvé.",
    "extractError": "Extraction du glossaire échouée. Réessayez depuis le panneau Glossaire."
  }
  ```

  > `extractDone` utilise l'interpolation `{{count}}` compatible react-i18next.
  > La clé `extractDone_zero` active la pluralisation i18next pour count=0
  > (pas de "0 termes" qui semble un échec silencieux).

Test de validation :
```bash
pnpm typecheck
pnpm lint
```
Résultat attendu : 0 erreur — les clés i18n ne sont pas typechecked mais lint
détecte les imports inutilisés

Commit message : `feat(i18n): add glossaryPrompt keys — en + fr`

---

## Tests obligatoires avant push GitHub

```bash
# TypeScript — vérification types
pnpm typecheck
# Résultat attendu : 0 erreur

# TypeScript — lint
pnpm lint
# Résultat attendu : 0 erreur ESLint

# Test manuel (pnpm tauri dev) :
# 1. Ouvrir un nouveau projet MV/MZ → AlertDialog apparaît
# 2. Cliquer "Ignorer" → dialog fermée, aucune bannière
# 3. Ouvrir un autre projet neuf → AlertDialog apparaît
# 4. Cliquer "Extraire" → dialog fermée, bannière visible, bouton Traduire grisé
#    avec label "Glossaire en cours…"
# 5. Attendre la fin → bannière disparaît, toast "X termes extraits", bouton réactivé
# 6. Rouvrir le même projet (was_restored=true) → aucun dialog
```

## Mise à jour après complétion

- `ROADMAP.md` : ajouter item coché sous F3 Polissage UI
- `CHANGELOG.md` : section `[Unreleased]` → entrée `Added`
- `docs/journal/` : nouvelle entrée `YYYY-MM-DD-f3-10-glossary-prompt.md`
