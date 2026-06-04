# Audit Refactoring — Hoshi2Star

> Généré le 2026-06-04. Analyse statique uniquement — aucun fichier modifié.
> Objectif : planifier les refactorings avant F4 (Wolf RPG) pour éviter que la dette technique s'accumule.

---

## Résumé exécutif

**Score global : 6/10**

La codebase est bien architecturée (ADRs respectés, séparation des couches Rust claire, conventions Tauri v2 correctes) mais souffre de duplication systématique au niveau implémentation : listeners dupliqués 7 fois, helpers utilitaires éparpillés entre modules, un fichier Rust de 1 539 lignes qui concentre 17 commandes.

**3 problèmes critiques :**
1. `commands/project.rs` — 1 539 lignes, 17 fonctions, responsabilités mélangées → risque élevé avant F4
2. `llm.ts` — `startTranslation` et `startTranslateAll` à 85 % identiques, teardown listeners répété 7 fois
3. `llm/pipeline.rs` — 718 lignes, fonction récursive complexe mélangée avec orchestration

**Estimation effort total : 20–26 h** (répartis en 7–9 PRs focalisés)

---

## Problèmes par priorité

### 🔴 Critique (à faire avant F4)

#### R-01 — `commands/project.rs` : 1 539 lignes, 17 responsabilités mélangées

**Fichier :** `src-tauri/src/commands/project.rs`
**Nature :** Longueur excessive + responsabilités mélangées

Ce fichier contient :
- Types domaine (lignes 29–134) : `Project`, `SourceFile`, `Segment`, `ProviderConfig`, `QaReport`, `ProjectStats`, `PaginatedSegments`, `OpenProjectResult`
- Types Ollama locaux (lignes 1075–1080) : `OllamaModel`, `OllamaTagsResponse`
- CRUD projet (lignes 136–335) : `open_project`
- CRUD fichiers/segments (lignes 336–496) : `get_source_files`, `get_segments`, `update_segment`
- Export (lignes 497–564) : `export_project`
- Traduction LLM (lignes 565–958) : `translate_segments`, `translate_all_segments`
- TM (lignes 959–973) : `get_tm_suggestions`
- QA (lignes 974–1073) : `qa_check_segment`, `get_qa_report`, `export_qa_report`
- Ollama meta (lignes 1074–1099) : `get_ollama_models`
- Debug (lignes 1301–1359) : `export_debug_json`
- TM export (lignes 1360–1393) : `export_tm`
- Stats/liste/suppression (lignes 1394–1539) : `get_project_stats`, `list_projects`, `delete_project`

**Solution proposée :**
```
src-tauri/src/commands/
  project.rs        ← CRUD projet + fichiers + segments (open, list, delete, get_segments, update_segment)
  translate.rs      ← translate_segments, translate_all_segments, get_ollama_models
  export.rs         ← export_project, export_qa_report, export_tm, export_debug_json
  stats.rs          ← get_project_stats, get_qa_report, qa_check_segment, get_tm_suggestions

src-tauri/src/domain/
  types.rs          ← Project, SourceFile, Segment, ProviderConfig, QaReport, ProjectStats (extraits de commands/)
```

**Effort : L** (6–8 h — split mécanique mais beaucoup de `use` à mettre à jour dans `lib.rs`)

---

#### R-02 — `llm.ts` : teardown listeners répété 7 fois + duplication 85 % entre deux fonctions

**Fichier :** `src/stores/llm.ts` (293 lignes)
**Nature :** Duplication code critique

Le même bloc de teardown apparaît 7 fois :
- Lignes 86–90 (début `startTranslation`)
- Lignes 110–114 (dans callback `completed` de `startTranslation`)
- Lignes 130–134 (dans callback `error` de `startTranslation`)
- Lignes 161–164 (dans callback `warning` de `startTranslation`)
- Lignes 180–184 (début `startTranslateAll`)
- Lignes 203–207 (dans callback `completed` de `startTranslateAll`)
- Lignes 223–227 (dans callback `error` de `startTranslateAll`)

De plus, `startTranslation` (lignes 79–166) et `startTranslateAll` (lignes 168–271) partagent 85 % du même code (setup listeners, progress, completed, error, warning) — seule la commande Tauri invoquée et la logique de cooling diffèrent.

**Solution proposée :**
```typescript
// src/lib/use-translation-listeners.ts
function createTranslationListeners(opts: {
  onProgress: (p: ProgressPayload) => void;
  onCompleted: () => void;
  onError: (msg: string) => void;
  onWarning?: (id: string) => void;
  onCooling?: (remaining: number) => void;
}): Promise<() => void>  // retourne teardown unique

// llm.ts — startTranslation et startTranslateAll utilisent createTranslationListeners()
// teardown : une seule variable `teardown: (() => void) | null = null`
```

**Effort : M** (3 h)

---

#### R-03 — `PH_RE` dupliqué entre `App.tsx` et `columns.tsx`

**Fichiers :** `src/App.tsx:66`, `src/features/editor/columns.tsx:118`
**Nature :** Duplication exacte — même regex `const PH_RE = /\\[+\-]\w+\[\d+\]|\\[VNPCI]\[\d+\]|\\[G\\$.|!><^{}]|\[%\d+\]/g`

Si le pattern de placeholder change (ex : ajout d'un opérateur Wolf RPG en F4), il faudra le mettre à jour dans deux endroits sans lien visible.

**Solution proposée :**
```typescript
// src/lib/constants.ts (nouveau fichier)
export const PH_RE = /\\[+\-]\w+\[\d+\]|\\[VNPCI]\[\d+\]|\\[G\\$.|!><^{}]|\[%\d+\]/g;
export function clonePH_RE() { return new RegExp(PH_RE.source, PH_RE.flags); }
// → les deux fichiers importent depuis @/lib/constants
```

**Effort : S** (30 min)

---

### 🟡 Important (à faire en F4)

#### R-04 — `App.tsx` : 632 lignes, logique mélangée avec UI

**Fichier :** `src/App.tsx` (632 lignes)
**Nature :** Responsabilités mélangées

Contenu actuel du fichier :
- Layout principal (ResizablePanelGroup) — légitime dans App.tsx
- 9 handlers async (lignes 350–480) : glossary extract, export, translate, translate-all, project open/close/delete
- 6 états locaux (lignes 303–340) : `showSettings`, `showAbout`, `showTranslateAll`, `translateAllStats`, `glossaryPromptProject`, `extractingGlossary`
- Toolbar complète inline (lignes 480–580)
- Toutes les modales inline (`SettingsModal`, `AboutModal`, `TranslateAllDialog`, `AlertDialog`)
- Chargement initial (`loadAllProjects` à la ligne 348)

**Solution proposée :**
```
src/
  App.tsx                  ← layout uniquement (ResizablePanelGroup + providers)
  components/
    AppToolbar.tsx          ← barre d'outils + CooldownBadge (extrait de App.tsx:480–580)
    AppDialogs.tsx          ← toutes les modales conditionnelles (SettingsModal, AlertDialog, etc.)
  hooks/
    useAppHandlers.ts       ← handlers async (openProject, export, translate) retournent les fns + états
```

**Effort : M** (4 h — refactor UI, pas de logique métier changée)

---

#### R-05 — `llm/pipeline.rs` : 718 lignes, fonction récursive de 100 lignes

**Fichier :** `src-tauri/src/llm/pipeline.rs` (718 lignes)
**Nature :** Longueur excessive + fonction trop complexe

`llm_translate_with_split` (environ lignes 176–282) : fonction récursive `Box::pin` qui gère à la fois le cas nominal, le split récursif sur `ResponseFormat::Incomplete`, et l'envoi d'événements de progression. Difficile à débugger quand la LLM retourne des réponses partielles en production.

**Solution proposée :**
```
src-tauri/src/llm/
  pipeline.rs      ← orchestration haut niveau (iterate files, call batch, emit events)
  split.rs         ← llm_translate_with_split + logique de split récursif isolée
  progress.rs      ← types ProgressPayload + fonctions emit_progress / emit_cooling
```

**Effort : M** (3 h)

---

#### R-06 — Helpers Rust dupliqués entre modules core

**Fichiers :**
- `src-tauri/src/core/tm.rs:222` — `fn xml_escape(s: &str)`
- `src-tauri/src/core/report.rs:87` — `fn html_escape(s: &str)` (même logique, nom différent)
- `src-tauri/src/core/manifest.rs:121` — `fn now_iso8601() -> String`

Les fonctions `xml_escape` et `html_escape` ont une implémentation quasi-identique (remplacement de `&`, `<`, `>`, `"`) mais sont définies séparément et non partagées. `now_iso8601` sera probablement nécessaire en F4 (logs Wolf, manifests).

**Solution proposée :**
```rust
// src-tauri/src/utils/text.rs (nouveau module)
pub fn xml_escape(s: &str) -> String { … }
pub fn html_escape(s: &str) -> String { … }  // alias ou même impl

// src-tauri/src/utils/time.rs (nouveau module)
pub fn now_iso8601() -> String { … }

// src-tauri/src/utils/mod.rs
pub mod text;
pub mod time;
```

**Effort : S** (1 h)

---

#### R-07 — Fonctions de formatage TS dupliquées entre composants frontend

**Fichiers :**
- `src/components/editor/FileTree.tsx:78` — `function formatDuration(seconds: number)`
- `src/components/editor/ProjectList.tsx:15` — `function engineLabel(engine: string)`
- `src/components/editor/ProjectList.tsx:30` — `function relativeDate(iso: string)`

Ces trois fonctions utilitaires sont définies localement et non réutilisables. `engineLabel` sera particulièrement utile en F4 quand Wolf RPG sera ajouté comme moteur.

**Solution proposée :**
```typescript
// src/lib/format.ts (nouveau fichier)
export function formatDuration(seconds: number): string { … }
export function engineLabel(engine: string): string { … }
export function relativeDate(iso: string): string { … }
```

**Effort : S** (45 min)

---

#### R-08 — `core/report.rs` : 705 lignes, génération HTML mélangée avec labels d'erreurs

**Fichier :** `src-tauri/src/core/report.rs` (705 lignes)
**Nature :** Responsabilités mélangées

Le fichier contient :
- Labels d'erreurs QA en anglais (lignes 103–131) et français (lignes 133–161)
- Structure `Labels` pour les titres du rapport (lignes 163–190)
- Génération HTML complète inline dans des strings format! (lignes 360–625)

Les labels d'erreurs dupliquent les variants de l'enum QA définis dans `core/qa.rs`. Si un nouveau type d'erreur QA est ajouté, il faut mettre à jour 3 endroits.

**Solution proposée :**
```rust
// Déplacer error_label_en / error_label_fr dans core/qa.rs
// comme méthodes de l'enum QaErrorKind : fn label(&self, lang: &str) -> &str

// report.rs : ne garde que la génération HTML + collecte des données
```

**Effort : M** (2 h)

---

### 🟢 Nice-to-have (backlog)

#### R-09 — `buildHighlightedNodes` dans `columns.tsx` difficile à tester

**Fichier :** `src/features/editor/columns.tsx:130–193`
**Nature :** Logique extractible

La fonction `buildHighlightedNodes` (60 lignes) gère la surbrillance simultanée des placeholders et des termes de glossaire. Algorithme correct mais couplé à la définition des colonnes. Aucun test unitaire possible en l'état.

**Solution proposée :**
```typescript
// src/lib/highlight-utils.ts
export function buildHighlightedNodes(
  text: string,
  glossaryTerms: string[],
  phRe: RegExp
): React.ReactNode[] { … }
// → testable avec Vitest pur, sans dépendance shadcn
```

**Effort : S** (1 h)

---

#### R-10 — `GlossaryPanel.tsx` : formulaires imbriqués dans le fichier principal

**Fichier :** `src/components/editor/GlossaryPanel.tsx` (469 lignes)
**Nature :** Composants extractibles

`AddTermForm` et `EditTermRow` sont définis comme composants locaux dans le fichier. Pas de bug, mais rend le fichier difficile à naviguer et les composants non réutilisables si un second panneau glossaire est ajouté (ex : gestion globale vs projet).

**Solution proposée :**
```
src/components/editor/
  GlossaryPanel.tsx        ← logique principale + layout
  GlossaryAddForm.tsx      ← formulaire ajout terme (extrait)
  GlossaryEditRow.tsx      ← ligne édition inline (extrait)
```

**Effort : S** (1.5 h)

---

#### R-11 — `SegmentGrid.tsx` : listeners dupliqués + état local épars

**Fichier :** `src/components/editor/SegmentGrid.tsx` (472 lignes)
**Nature :** Duplication listen/unlisten + état local épars

Pattern `listen/unlisten` répété aux lignes 126–135 et 151–168. État local (`qaFilter`, `rowSelection`, refs virtualiseur) qui pourrait partiellement rejoindre `editor.ts` pour persistance entre changements de fichier.

**Solution proposée :**
```typescript
// src/hooks/useSegmentListeners.ts
// Regroupe les deux listen() de SegmentGrid + cleanup automatique
// (après R-02, ce hook peut réutiliser createTranslationListeners)
```

**Effort : S** (1 h — après R-02)

---

#### R-12 — Types domaine Rust dans `commands/` plutôt que dans un module partagé

**Fichier :** `src-tauri/src/commands/project.rs:29–134`
**Nature :** Types mal placés

`Project`, `SourceFile`, `Segment`, `ProviderConfig` etc. sont définis dans `commands/project.rs` avec `pub` mais appartiennent conceptuellement à la couche domaine. Si un futur module (ex : `sync/git.rs` en F5) a besoin de ces types, il devra dépendre de `commands/` ce qui crée un couplage inversé.

Lié à R-01 — ce problème est résolu dans le cadre du split de `commands/project.rs`.

**Effort : inclus dans R-01**

---

## Frontend — Propositions détaillées

### App.tsx

**Contenu actuel :** 632 lignes — layout + 9 handlers + 6 états locaux + toolbar inline + 4 modales
**Problème principal :** Un changement mineur dans la toolbar ou une modale nécessite de naviguer dans un fichier de 600+ lignes

**Proposition :**
```
src/
  App.tsx                    ← ~150 lignes : layout ResizablePanelGroup + providers + AppDialogs
  components/
    AppToolbar.tsx            ← ~120 lignes : boutons toolbar + CooldownBadge (lignes 480–580 actuelles)
    AppDialogs.tsx            ← ~80 lignes : modales conditionnelles (SettingsModal, TranslateAllDialog, AlertDialog)
  hooks/
    useAppHandlers.ts         ← ~120 lignes : handlers async (handleExport, handleTranslate, handleGlossaryExtract)
```

### stores/

**`llm.ts` (293 lignes)** — voir R-02 (critique)
**`project.ts` (112 lignes)** — `openProject`, `loadAllProjects`, `deleteProject` sont des thunks qui font des `invoke()` directs dans le store. Acceptable pour l'instant mais si le projet grossit, une couche `services/project-service.ts` éviterait que le store gère les effets de bord. **Non urgent.**
**`editor.ts` (57 lignes)** — propre, rien à faire.

### features/editor/columns.tsx

**Contenu actuel :** 328 lignes — définitions colonnes + `EditableCell` + `buildHighlightedNodes` + `PH_RE`
**Proposition :** Extraire `PH_RE` (R-03), `buildHighlightedNodes` (R-09), garder `EditableCell` inline (raisonnable vu le couplage avec la colonne target).

---

## Backend — Propositions détaillées

### commands/project.rs

**Contenu actuel :** 1 539 lignes, 17 fonctions publiques + 8 structs domaine
**Proposition de découpage (R-01) :**

| Nouveau fichier | Fonctions | Lignes estimées |
|----------------|-----------|----------------|
| `commands/project.rs` | `open_project`, `get_source_files`, `get_segments`, `update_segment`, `list_projects`, `delete_project`, `get_project_stats` | ~500 |
| `commands/translate.rs` | `translate_segments`, `translate_all_segments`, `get_ollama_models` | ~450 |
| `commands/export.rs` | `export_project`, `export_qa_report`, `export_tm`, `export_debug_json` | ~400 |
| `commands/qa.rs` | `qa_check_segment`, `get_qa_report`, `get_tm_suggestions` | ~150 |
| `domain/types.rs` | structs `Project`, `SourceFile`, `Segment`, `ProviderConfig`, `QaReport`, `ProjectStats`, `PaginatedSegments`, `OpenProjectResult` | ~110 |

**Impact `lib.rs` :** aucun changement visible (les fonctions gardent les mêmes noms dans `generate_handler!`), seulement les `use` en tête de fichier.

### llm/pipeline.rs

**Contenu actuel :** 718 lignes
**Proposition (R-05) :**

| Nouveau fichier | Responsabilité |
|----------------|---------------|
| `llm/pipeline.rs` | Orchestration : itérer sur les fichiers, appeler batch, émettre progression |
| `llm/split.rs` | `llm_translate_with_split` + logique de split récursif |
| `llm/progress.rs` | Types `ProgressPayload` + fonctions `emit_progress`, `emit_cooling` |

### core/report.rs

**Contenu actuel :** 705 lignes
**Proposition (R-08) :**
- Déplacer `error_label_en`/`error_label_fr` dans `core/qa.rs` comme méthodes de l'enum
- `report.rs` ne garde que la génération HTML et la collecte de données (~400 lignes)

---

## Plan d'exécution suggéré

Ordre recommandé pour minimiser les conflits de merge et maximiser la valeur immédiate :

### Sprint 1 — Duplications critiques (avant F4, ~5 h)
```
branch: chore/refactor-sprint1-dedup
```
1. **R-03** — Extraire `PH_RE` dans `src/lib/constants.ts` (30 min)
2. **R-07** — Créer `src/lib/format.ts` avec `formatDuration`, `engineLabel`, `relativeDate` (45 min)
3. **R-06** — Créer `src-tauri/src/utils/` avec `text.rs` et `time.rs` (1 h)
4. **R-02** — Refactoriser `llm.ts` (teardown + factorisation startTranslation/startTranslateAll) (3 h)

### Sprint 2 — Split fichiers lourds backend (~8 h)
```
branch: chore/refactor-sprint2-split-commands
```
5. **R-01** — Découper `commands/project.rs` en 4 fichiers + `domain/types.rs`
   - Créer `domain/types.rs` + vérifier `cargo clippy`
   - Créer `commands/translate.rs` + mettre à jour `lib.rs`
   - Créer `commands/export.rs` + mettre à jour `lib.rs`
   - Créer `commands/qa.rs` + mettre à jour `lib.rs`

### Sprint 3 — Split pipeline LLM + report (~5 h)
```
branch: chore/refactor-sprint3-llm-report
```
6. **R-05** — Découper `llm/pipeline.rs` (split.rs + progress.rs)
7. **R-08** — Déplacer labels d'erreurs QA dans `core/qa.rs`

### Sprint 4 — Frontend App.tsx (~6 h)
```
branch: chore/refactor-sprint4-app-tsx
```
8. **R-04** — Extraire `AppToolbar`, `AppDialogs`, `useAppHandlers` depuis `App.tsx`
9. **R-09** — Extraire `buildHighlightedNodes` dans `src/lib/highlight-utils.ts`

### Backlog (post-F4 si temps disponible)
- **R-10** — GlossaryPanel split (GlossaryAddForm + GlossaryEditRow)
- **R-11** — SegmentGrid useSegmentListeners hook
- **R-12** — inclus dans R-01

---

## Critère de validation pour chaque sprint

Après chaque sprint :
```bash
pnpm typecheck                                            # 0 erreur TS
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings  # 0 warning
cargo test --manifest-path src-tauri/Cargo.toml          # tous les tests passent
pnpm test                                                 # Vitest vert
```

Aucun refactoring ne doit modifier le comportement observable (pas de changement de signature des commandes Tauri, pas de renommage d'events `h2s://`).
