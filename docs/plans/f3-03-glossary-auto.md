# Plan F3-03 — Glossaire Auto + Manuel

## Objectif

Implémenter le glossaire Hoshi2Star complet : table SQL, CRUD Rust, extraction automatique
via LLM (Actors/Skills/Items/States), injection dans le prompt de traduction, vérification QA,
panel React (CRUD inline + bouton "Extraire auto"), et highlight des termes dans la grille.

## Statut : [x] Complété — 2026-05-28

## Prérequis

- F3-01 complet (VX Ace marshal-rs) — `[x]` Fait
- F2 complet (LLM pipeline, TM, QA v1) — `[x]` Fait
- `TranslationContext.glossary_terms: Vec<(String, String)>` déjà déclaré dans `provider.rs`
  et déjà injecté dans le system prompt Ollama (lines 144–153) — scaffolding existant ✅
- `commands/project.rs` construit `TranslationContext { glossary_terms: vec![] }` — à câbler

## Estimation

9 steps · ~50–75 min total

## Items ROADMAP concernés

```
F3 — Core Layer — Glossaire v1 :
  [ ] src-tauri/migrations/0003_glossary.sql
  [ ] src-tauri/src/core/glossary.rs — CRUD + merge global/projet-local
  [ ] Injection des termes dans le prompt de traduction (passe 1)

F3 — CAT UI — F3 :
  [ ] src/components/editor/GlossaryPanel.tsx
  [ ] Highlight inline des termes glossaire dans le source
  [ ] QA : warning si terme glossaire non respecté dans le target
```

---

## Steps

---

### Step 1 — Migration SQL glossaire

**Objectif :** Créer la table `glossary_terms` avec support portée globale/projet.

**Fichiers touchés :**
- `src-tauri/migrations/0003_glossary.sql` ← nouveau

**Dépend de :** *(aucun — migration indépendante)*

Tâches :
- [ ] Créer `0003_glossary.sql` avec la table suivante :

```sql
CREATE TABLE IF NOT EXISTS glossary_terms (
    id             TEXT    PRIMARY KEY NOT NULL,
    source_text    TEXT    NOT NULL,
    target_text    TEXT    NOT NULL,
    lang_pair      TEXT    NOT NULL,            -- 'ja-en', 'ja-fr', ...
    domain         TEXT    NOT NULL DEFAULT '', -- 'character', 'skill', 'item', 'state', ''
    project_id     TEXT    REFERENCES projects(id) ON DELETE CASCADE, -- NULL = global
    auto_generated INTEGER NOT NULL DEFAULT 0, -- 1 si généré par LLM
    created_at     TEXT    NOT NULL DEFAULT (datetime('now')),
    updated_at     TEXT    NOT NULL DEFAULT (datetime('now'))
);
-- Index composite : lookup par projet + paire de langues
CREATE INDEX IF NOT EXISTS idx_glossary_project_lang
    ON glossary_terms(project_id, lang_pair);
-- Index lookup global (project_id IS NULL)
CREATE INDEX IF NOT EXISTS idx_glossary_global_lang
    ON glossary_terms(lang_pair) WHERE project_id IS NULL;
```

> `project_id` nullable = terme global. Terme projet surcharge terme global (même source_text).

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml -- db::
```
Résultat attendu : migrations s'appliquent sans erreur (sqlx::migrate! au démarrage)

Commit message : `chore(db): add migration 0003_glossary — glossary_terms table`

---

### Step 2 — core/glossary.rs — CRUD de base

**Objectif :** Module Rust avec insert/update/delete/list + merge global+projet.

**Fichiers touchés :**
- `src-tauri/src/core/glossary.rs` ← nouveau
- `src-tauri/src/core/mod.rs` ← ajouter `pub mod glossary;`

**Dépend de :** Step 1

Tâches :
- [ ] Créer struct `GlossaryTerm` (Serialize, Deserialize, FromRow) avec `#[serde(rename_all = "camelCase")]`
- [ ] `insert_term(pool, term) -> Result<GlossaryTerm, sqlx::Error>` — génère UUID v4
- [ ] `update_term(pool, id, source_text, target_text, domain) -> Result<GlossaryTerm, sqlx::Error>`
- [ ] `delete_term(pool, id) -> Result<(), sqlx::Error>`
- [ ] `list_for_project(pool, project_id, lang_pair) -> Result<Vec<GlossaryTerm>, sqlx::Error>`
  - Retourne : termes globaux (project_id IS NULL) + termes du projet, fusionnés
  - Si collision source_text : terme projet surcharge terme global
  - Tri : globaux d'abord, puis projet-local — par source_text ASC
- [ ] Tests unitaires dans `#[cfg(test)] mod tests` :
  - test_insert_and_list_global : insérer terme global, le récupérer
  - test_project_overrides_global : même source_text → terme projet masque le global
  - test_delete_cascades_with_project : DELETE CASCADE quand projet supprimé

> **Note architecture** : `list_for_project` retourne un `Vec<GlossaryTerm>` fusionné en mémoire
> plutôt qu'une requête SQL complexe — cohérent avec le style de `tm.rs`.

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml core::glossary
```
Résultat attendu : 3 tests passent (green)

Commit message : `feat(core): add glossary.rs — CRUD + global/project merge`

---

### Step 3 — ⚠️ Extraction auto des termes (LLM)

**Objectif :** Fonction `extract_terms_from_project` qui interroge le LLM sur les fichiers
Actors/Skills/Items/States pour identifier des termes à glossariser.

**Fichiers touchés :**
- `src-tauri/src/core/glossary.rs` ← ajouter `extract_terms_from_project`
- `src-tauri/src/llm/provider.rs` ← ajouter méthode `extract_glossary_terms` sur le trait (optionnel : peut rester une fonction autonome)

**Dépend de :** Step 2

**Pourquoi ⚠️** : parsing de la réponse LLM, quotas DB, gestion d'erreurs LLM, format JSON fragile.

Tâches :
- [ ] Définir quels fichiers analyser :
  - `Actors.json` → champ `name` de chaque acteur (personnages principaux)
  - `Skills.json` → champ `name` de chaque compétence
  - `Items.json` → champ `name` de chaque objet
  - `States.json` → champ `name` de chaque état (buffs/debuffs)
  - Source : requête SQL sur `segments` filtrée par `file_type IN ('actors', 'skills', 'items', 'states')`
    et `json_key LIKE '%/name'`

- [ ] Construire le prompt d'extraction (system + user) :

```
System:
You are a Japanese game localization expert.
Identify proper nouns and game-specific terms that must be translated consistently.
Respond ONLY with a JSON array. No explanation. No markdown.

User:
Here are source texts from a Japanese RPG. Identify up to 50 terms worth glossarizing
(character names, place names, skill names, item names, class names).
For each term, provide a suggested English translation.
Format: [{"source":"JP term","target":"EN term","domain":"character|skill|item|state|other"}]

Source texts:
<list of up to 200 unique source_text values, one per line>
```

- [ ] Parser la réponse JSON :
  - Deserializer vers `Vec<ExtractedTerm>` (struct locale avec source/target/domain)
  - Si JSON invalide : log warning + retourner `vec![]` (pas d'erreur fatale)
  - Filtrer les termes avec source_text vide ou target_text vide
  - Limite : max 50 termes retenus

- [ ] Insérer les termes en DB avec `auto_generated = true` et `project_id = project_id`
  - Skip si un terme avec la même source_text existe déjà pour ce projet (pas d'écrasement)

- [ ] Signature : `pub async fn extract_terms_from_project(pool: &SqlitePool, provider: &impl LlmProvider, project_id: &str, lang_pair: &str) -> Result<Vec<GlossaryTerm>, String>`

- [ ] Test unitaire avec mock provider :
  - Vérifier qu'un JSON valide produit des termes en DB
  - Vérifier qu'un JSON invalide retourne `vec![]` sans paniquer
  - Vérifier la limite 50 termes

> **Stratégie de robustesse** : le JSON peut contenir du texte avant/après le tableau —
> chercher le premier `[` et le dernier `]` pour extraire le tableau.

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml core::glossary::tests::extract
```
Résultat attendu : tests mock passent, `cargo clippy -D warnings` vert

Commit message : `feat(core): glossary LLM extraction — extract_terms_from_project`

---

### Step 4 — Injection glossaire dans le prompt LLM

**Objectif :** Câbler les termes glossaire DB dans le `TranslationContext` lors d'une traduction.

**Fichiers touchés :**
- `src-tauri/src/commands/project.rs` ← modifier `translate_segments`

**Dépend de :** Step 2

> **Context important** : `TranslationContext.glossary_terms` et son injection dans le system
> prompt sont **déjà implémentés** dans `provider.rs` (lignes 144–153). Le field est simplement
> initialisé à `vec![]` dans `commands/project.rs:497`. Ce step ne touche pas `pipeline.rs` —
> uniquement le site de construction du `TranslationContext`.

Tâches :
- [ ] Dans `translate_segments` (commands/project.rs), avant de construire `TranslationContext` :
  1. Appeler `glossary::list_for_project(&state.db, &project_id, &lang_pair)`
  2. Convertir `Vec<GlossaryTerm>` → `Vec<(String, String)>` (source_text, target_text)
  3. Limiter à 30 termes max (les 30 premiers — termes globaux en tête, déjà triés par `list_for_project`)
- [ ] Passer les termes dans `TranslationContext { ..., glossary_terms: terms }`
- [ ] Ajouter `use crate::core::glossary;` dans les imports du fichier

> **Pourquoi 30 max** : avec 20 segments × 50 chars + 30 termes × 20 chars, le prompt reste
> sous ~2K tokens — safe pour tous les modèles Ollama courants (Qwen3 7B+).

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml commands::
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```
Résultat attendu : compile clean, clippy vert

Commit message : `feat(llm): wire glossary terms into TranslationContext before LLM call`

---

### Step 5 — QA glossaire

**Objectif :** Nouveau check QA qui détecte les termes glossaire absents du target traduit.

**Fichiers touchés :**
- `src-tauri/src/core/qa.rs` ← ajouter `QaError::GlossaryMismatch` + check + pénalité

**Dépend de :** Step 2

Tâches :
- [ ] Ajouter variant `QaError::GlossaryMismatch { source_term: String, expected_target: String }` dans l'enum `QaError`
- [ ] Pénalité : `-15` points (même niveau que `BomDetected`)
- [ ] Mettre à jour le doc-comment du module (liste des checks + score)
- [ ] Nouvelle fonction `check_glossary(source: &str, target: &str, terms: &[(String, String)]) -> Vec<QaError>` :
  - Pour chaque `(source_term, target_term)` :
    - Si `source.contains(source_term)` ET `!target.contains(target_term)` → `GlossaryMismatch`
  - Comparaison case-insensitive pour le target (un traducteur peut écrire "mage" ou "Mage")
- [ ] Modifier la signature de `run_qa` (ou la fonction publique équivalente) pour accepter
  `glossary_terms: &[(String, String)]`
- [ ] Appeler `check_glossary` dans la séquence de checks existante
- [ ] Tests unitaires :
  - Terme glossaire présent dans source + absent du target → `GlossaryMismatch` + score -15
  - Terme glossaire présent dans source + présent dans target → aucune erreur
  - Source sans terme glossaire → aucune erreur même si target vide
  - Vérifier score minimum reste 0 avec plusieurs `GlossaryMismatch`

> **Note** : la signature publique de `run_qa` change — vérifier et mettre à jour
> les appels dans `commands/project.rs` (ajouter `&[]` si pas encore de termes disponibles).

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml core::qa
```
Résultat attendu : tous les tests QA (existants + nouveaux) passent

Commit message : `feat(qa): add GlossaryMismatch check — -15 pts if glossary term absent from target`

---

### Step 6 — Commands Tauri glossaire

**Objectif :** Exposer 5 commands IPC pour le CRUD glossaire et l'extraction auto.

**Fichiers touchés :**
- `src-tauri/src/commands/glossary.rs` ← nouveau
- `src-tauri/src/commands/mod.rs` ← ajouter `pub mod glossary;`
- `src-tauri/src/lib.rs` ← enregistrer les 5 commands dans `generate_handler!`

**Dépend de :** Step 2, Step 3

Tâches :
- [ ] `get_glossary(project_id: String, lang_pair: String, state: State<AppState>) -> Result<Vec<GlossaryTerm>, String>`
- [ ] `add_glossary_term(source_text: String, target_text: String, lang_pair: String, domain: String, project_id: Option<String>, state: State<AppState>) -> Result<GlossaryTerm, String>`
- [ ] `update_glossary_term(id: String, source_text: String, target_text: String, domain: String, state: State<AppState>) -> Result<GlossaryTerm, String>`
- [ ] `delete_glossary_term(id: String, state: State<AppState>) -> Result<(), String>`
- [ ] `extract_glossary_terms(project_id: String, lang_pair: String, provider_config: ProviderConfig, state: State<AppState>, app: AppHandle) -> Result<(), String>`
  - `ProviderConfig` est **passé depuis le frontend à chaque appel** (même pattern que `translate_segments`)
    il N'EST PAS stocké dans AppState — le frontend lit `useLlmStore().providerConfig` et l'envoie
  - Construire `OllamaProvider` localement depuis `provider_config.url` et `provider_config.model`
    (identique à `translate_segments` ligne ~489 dans project.rs)
  - Lance dans `tokio::spawn` pour ne pas bloquer le thread IPC
  - Émet `h2s://glossary/extraction-done` avec les termes créés quand terminé
  - Retourne `Ok(())` immédiatement — le résultat arrive via event (pas dans la valeur de retour)
  - `ProviderConfig` est défini dans `commands/project.rs` — l'importer avec
    `use crate::commands::project::ProviderConfig;` en tête de `commands/glossary.rs`

> Pattern `Result<T, String>` OBLIGATOIRE sur toutes les commands (convention CLAUDE.md).
> Enregistrer dans le seul `generate_handler![..., get_glossary, add_glossary_term, ...]`.

Test de validation :
```bash
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
pnpm typecheck
```
Résultat attendu : 0 erreur compilation, clippy vert, typecheck vert

Commit message : `feat(commands): add 5 glossary Tauri commands + IPC event`

---

### Step 7 — GlossaryPanel.tsx

**Objectif :** Panneau React avec liste des termes, CRUD inline, et bouton d'extraction auto.

**Fichiers touchés :**
- `src/components/editor/GlossaryPanel.tsx` ← nouveau
- `src/lib/types.ts` ← ajouter type `GlossaryTerm` (miroir de la struct Rust)

**Dépend de :** Step 6

Tâches :
- [ ] Type TS `GlossaryTerm` dans `src/lib/types.ts` :
  ```ts
  export interface GlossaryTerm {
    id: string
    sourceText: string
    targetText: string
    langPair: string
    domain: string
    projectId: string | null
    autoGenerated: boolean
    createdAt: string
    updatedAt: string
  }
  ```
- [ ] `GlossaryPanel` : reçoit `projectId: string | null` et `langPair: string` en props
- [ ] Chargement initial : `invoke<GlossaryTerm[]>('get_glossary', { projectId, langPair })`
  - Rafraîchissement sur event `h2s://glossary/extraction-done`
- [ ] Affichage liste : colonnes Source | Target | Domaine | Portée (Global/Projet) | Actions
  - Badge "Auto" (shadcn `Badge` variant `secondary`) sur `autoGenerated === true`
  - Badge portée : "Global" (outline) ou "Projet" (default)
- [ ] CRUD inline :
  - Bouton "+" → formulaire inline (Input shadcn) pour ajouter un terme
  - Icône édition par ligne → inline edit source + target
  - Icône poubelle → `delete_glossary_term(id)` + confirmation (Alert Dialog shadcn)
- [ ] Bouton "Extraire auto" → `invoke('extract_glossary_terms', { projectId, langPair, providerConfig })`
  - État loading pendant l'extraction (spinner + disabled)
  - Toast success/error via `sonner`
- [ ] **i18n** — ajouter la section `glossaryPanel` dans `src/locales/en.json` et `src/locales/fr.json`
  en suivant le même pattern que `tmPanel` et `qaPanel` (section par panneau, clé racine = nom camelCase) :
  ```json
  "glossaryPanel": {
    "title": "Glossary",
    "extract": "Extract Auto",
    "extracting": "Extracting...",
    "addTerm": "Add term",
    "source": "Source",
    "target": "Target",
    "domain": "Domain",
    "scope": "Scope",
    "global": "Global",
    "project": "Project",
    "auto": "Auto",
    "noTerms": "No glossary terms yet",
    "deleteConfirm": "Delete this term?"
  }
  ```
  Version `fr.json` correspondante (mêmes clés, valeurs en français).
  Utiliser `const { t } = useTranslation()` dans `GlossaryPanel` avec les clés `glossaryPanel.xxx`.

Test de validation :
```bash
pnpm typecheck
pnpm lint
```
Résultat attendu : 0 erreur TS, 0 erreur lint

Commit message : `feat(ui): add GlossaryPanel — CRUD inline + auto extract button`

---

### Step 8 — Highlight termes glossaire dans SegmentGrid

**Objectif :** Dans la colonne Source, surligner les termes glossaire reconnus en vert.

**Fichiers touchés :**
- `src/components/editor/SegmentGrid.tsx` ← modifier la cellule Source
- `src/stores/editor.ts` ← ajouter `glossaryTerms: GlossaryTerm[]` dans le store

**Dépend de :** Step 7

> **Complexité** : le composant `HighlightedSource` existe déjà pour les placeholders.
> Le même pattern s'applique pour les termes glossaire — couleur distincte.
> ⚠️ Si la complexité de l'overlap placeholders+glossaire est trop élevée, reporter en backlog.

Tâches :
- [ ] Ajouter `glossaryTerms: GlossaryTerm[]` dans le store Zustand `editor.ts`
  - Chargé au changement de projet actif ou sur event `h2s://glossary/extraction-done`
- [ ] Créer fonction `highlightGlossaryTerms(text: string, terms: GlossaryTerm[]): ReactNode` :
  - Split le texte sur les terms.sourceText (tri décroissant par longueur pour éviter les sous-chaînes)
  - Wrap les matches en `<mark className="bg-green-200 dark:bg-green-900 rounded-sm px-0.5">`
  - Si 0 termes → retourner le texte brut (pas de régression perf)
- [ ] Dans la cellule Source de `SegmentGrid` : appliquer le highlight après les placeholders
  - Ordre : d'abord `HighlightedSource` (placeholders), ensuite `highlightGlossaryTerms`
  - Accepter que l'overlap ne soit pas parfait (cas rares en pratique)
- [ ] Test manuel : ouvrir un projet, vérifier le highlight vert sur un terme connu

> **Critère de report backlog** : si l'imbrication ReactNode + regex > 30 min de debug, créer
> une issue GitHub et passer au Step 9 sans le highlight.

Test de validation :
```bash
pnpm typecheck
pnpm lint
```
Résultat attendu : 0 erreur, pas de régression visuelle placeholders

Commit message : `feat(ui): highlight glossary terms in SegmentGrid source column`

---

### Step 9 — Intégration SidePanel

**Objectif :** Ajouter `GlossaryPanel` dans le panneau droit comme 3ème panel redimensionnable, aux côtés de TM et QA.

**Fichiers touchés :**
- `src/App.tsx` ← modifier le `ResizablePanelGroup` vertical du panneau droit

**Dépend de :** Step 7

> **Structure exacte lue dans App.tsx** : il n'y a **pas de Tabs** dans le SidePanel.
> Le panneau droit est un `ResizablePanel` (25% largeur) contenant un
> `ResizablePanelGroup orientation="vertical"` avec 2 panels :
> `TMPanel` (defaultSize=55, no props) et `QAPanel` (defaultSize=45,
> props `sourceText` et `targetText` lus depuis `useEditorStore`),
> séparés par un `ResizableHandle`. Aucun shadcn `Tabs` n'existe.

Tâches :
- [ ] Dans `src/App.tsx`, dans le `ResizablePanelGroup orientation="vertical"` existant,
  ajouter un 3ème panel après `QAPanel` :
  ```jsx
  <ResizableHandle />
  <ResizablePanel defaultSize={30} minSize={20}>
    <GlossaryPanel projectId={activeProjectId} langPair="ja-en" />
  </ResizablePanel>
  ```
- [ ] Rééquilibrer les `defaultSize` des panels existants :
  TM → `defaultSize={40}`, QA → `defaultSize={30}`, Glossaire → `defaultSize={30}`
  (total = 100)
- [ ] Importer `GlossaryPanel` depuis `@/components/editor/GlossaryPanel`
- [ ] `activeProjectId` est déjà disponible via `useProjectStore` dans `App` (ligne ~271)
- [ ] Afficher le count de termes dans le **header interne** de `GlossaryPanel`
  (texte `"{count} termes"` dans le header, pas sur un onglet — pattern identique au
  header texte de `TMPanel` ou `FileTree`)
- [ ] Vérifier que la démo complète fonctionne :
  - Ouvrir projet MV/MZ → 3ème panel Glossaire visible avec 0 termes
  - Cliquer "Extraire auto" → liste se peuple après extraction
  - Ajouter terme manuel → apparaît dans la liste immédiatement

Test de validation :
```bash
pnpm typecheck
pnpm tauri dev    # vérification visuelle
```
Résultat attendu : 3 panels redimensionnables dans le panneau droit, GlossaryPanel fonctionnel de bout en bout

Commit message : `feat(ui): integrate GlossaryPanel into SidePanel as third resizable panel`

---

## Tests obligatoires avant push GitHub

```bash
# Rust — unitaires + intégration
cargo test --manifest-path src-tauri/Cargo.toml
# Résultat attendu : tous les tests passent (inclut les nouveaux glossary + qa)

# Rust — linting qualité
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
# Résultat attendu : 0 warning, 0 erreur

# Rust — formatage
cargo fmt --manifest-path src-tauri/Cargo.toml
# Résultat attendu : aucun fichier modifié

# TypeScript — vérification types
pnpm typecheck
# Résultat attendu : 0 erreur

# TypeScript — lint
pnpm lint
# Résultat attendu : 0 erreur ESLint
```

## Commandes git de push final

```bash
# Depuis la branche feat/f3-03-glossary-auto (pas plan/)
git checkout main
git merge --no-ff feat/f3-03-glossary-auto -m "feat(f3-03): glossary auto+manual — CRUD, LLM extraction, QA, UI panel"
git push origin main
git branch -d feat/f3-03-glossary-auto
git branch -d plan/f3-03-glossary-auto
```

## Mise à jour après complétion

Fichiers à mettre à jour une fois tous les steps complétés :

- `ROADMAP.md` : cocher les 6 items F3 Core Layer Glossaire v1 + les 3 items CAT UI F3 concernés
- `CHANGELOG.md` : section `[Unreleased]` → entrée `Added` glossaire
- `docs/journal/` : nouvelle entrée `YYYY-MM-DD-f3-03-glossary.md`
- `CONTEXT.md` : ADR-003 note — glossaire deux niveaux (global/projet) documenté
