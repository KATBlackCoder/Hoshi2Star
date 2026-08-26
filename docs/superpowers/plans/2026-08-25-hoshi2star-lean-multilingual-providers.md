# Hoshi2Star Lean Multilingual Providers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Recentrer Hoshi2Star sur l'usage personnel, rendre les langues source/cible réellement configurables et remplacer le couplage Ollama par un unique protocole OpenAI-compatible couvrant Ollama, LM Studio, Hugging Face et les API cloud.

**Architecture:** Les parseurs, la CAT, la TM, le glossaire, le QA et les packs `.h2s` restent le cœur stable. Tous les moteurs LLM parlent désormais le même contrat HTTP (`/v1/models`, `/v1/chat/completions`) derrière un `OpenAiCompatibleProvider`; les différences de services sont uniquement des préréglages TypeScript. La paire de langues appartient au projet, les secrets restent en mémoire pour le MVP, et le Player Mode du plan précédent ne commence qu'après cette fondation.

**Tech Stack:** Tauri v2, Rust, reqwest/rustls, React 19, TypeScript strict, Zustand, SQLite/sqlx, Vitest, cargo test/httpmock.

---

## Résultat de l'audit et limites

Conserver : parseurs MV/MZ et Wolf, code VX Ace, CAT editor, mémoire de traduction, glossaire, QA, recherche globale, export ZIP, sauvegarde/import `.h2s`, prompts TOML et auto-updater déjà fonctionnel.

Simplifier ou retirer du chemin principal : monétisation, tiers payants, métriques commerciales, recrutement beta, promotion itch.io, collaboration Git, passe de ton, Scene Graph immédiat, estimation de coût, paramètres de reasoning par fournisseur et pauses manuelles « travail/repos ». Ces éléments ne doivent pas bloquer l'objectif personnel.

Ne pas embarquer de runtime de modèle Hugging Face. Les modèles locaux sont servis par Ollama, LM Studio, llama.cpp, vLLM ou TGI; Hugging Face cloud utilise `https://router.huggingface.co/v1`.

État de départ vérifié le 2026-08-25 : `pnpm typecheck` passe, 55 tests frontend passent, ESLint retourne 10 avertissements et aucune erreur, Clippy passe. `cargo test` donne 416 succès, 2 échecs de fixtures Wolf absentes et 4 tests ignorés.

## Ordre des sous-projets

1. **Fondation saine** : rendre les tests reproductibles et nettoyer la documentation active.
2. **Langues de projet** : supprimer tous les `ja-en` câblés en dur.
3. **Provider unique** : compatibilité Ollama, LM Studio, Hugging Face et cloud.
4. **UX minimale** : préréglage, URL, modèle, clé de session et test de connexion.
5. **Pipeline résilient** : erreurs HTTP propres et backoff automatique; retirer les pauses manuelles.
6. **Redesign** : exécuter `2026-08-25-hoshi2star-ui-redesign.md` pour installer l'app shell et la navigation par modes.
7. **Player Mode** : reprendre ensuite `2026-08-25-hoshi2star-personal-player-mode.md` à partir de P1.

## Structure de fichiers cible

### Backend

- Create: `src-tauri/migrations/0006_project_languages.sql` — langue source/cible par projet.
- Modify: `src-tauri/src/domain/types.rs` — `Project` et `ProviderConfig` génériques.
- Replace: `src-tauri/src/llm/provider.rs` — contrat, erreurs et parseur partagés.
- Create: `src-tauri/src/llm/openai_compatible.rs` — seul client HTTP de chat.
- Modify: `src-tauri/src/llm/mod.rs` — export du client.
- Modify: `src-tauri/src/commands/translate.rs` — résolution du projet et du provider en un point.
- Modify: `src-tauri/src/commands/glossary.rs` — réutilisation de la même construction.
- Modify: `src-tauri/src/lib.rs` — commande `list_provider_models`.
- Modify: `src-tauri/src/llm/pipeline.rs` — suppression du cooldown manuel.
- Modify: `src-tauri/src/llm/progress.rs` — suppression de l'événement cooling.

### Frontend

- Create: `src/lib/providerPresets.ts` — données de préréglages, sans logique réseau.
- Modify: `src/lib/types.ts` — paramètres provider et langues.
- Modify: `src/lib/constants.ts` — défauts génériques.
- Modify: `src/stores/settings.ts` — migration des anciennes clés Ollama.
- Modify: `src/stores/llm.ts` — clé API en mémoire et aucun état cooldown.
- Modify: `src/components/settings/SettingsModal.tsx` — formulaire provider/langues.
- Modify: `src/components/TranslateAllDialog.tsx` — confirmation simple.
- Modify: `src/hooks/useAppHandlers.ts` — paire de langues du projet.
- Modify: `src/components/AppToolbar.tsx` — suppression du badge cooling.
- Modify: `src/App.tsx`, `src/components/editor/TMPanel.tsx`, `SegmentGrid.tsx`, `GlossaryPanel.tsx`, `PackExportDialog.tsx`, `PackImportWizard.tsx` — paire dynamique.
- Modify: `src/locales/fr.json`, `src/locales/en.json` — libellés génériques.

### Documentation

- Modify: `README.md`, `README.fr.md`, `ROADMAP.md`, `CONTEXT.md`, `BACKLOG.md`.
- Create: `docs/providers.md`, `docs/providers.fr.md`.
- Delete after review: `docs/promo-plan.md`, `docs/itch-distribution.md`, `docs/runpod.md`, `docs/runpod.fr.md`.
- Delete after review: anciens `docs/journal/*.md` et plans terminés de `docs/plans/`; Git conserve leur historique.
- Rewrite: `tasks/todo.md` — uniquement les tâches actives, moins de 200 lignes.

---

### Task 1: Réparer la baseline de tests avant refactor

**Files:**
- Modify: `src-tauri/src/engines/wolf/v3_format/common_events.rs`
- Modify: `src-tauri/src/engines/wolf/v3_format/map.rs`
- Modify: `src-tauri/src/engines/wolf/placeholders.rs`

- [ ] **Step 1: Rendre la fixture CommonEvent optionnelle**

Remplacer le `expect` de lecture de fixture par :

```rust
let path = fixture_path("Densyanai_Inko_ver2.0/Data/BasicData/CommonEvent.dat");
if !path.exists() {
    eprintln!("skipping optional real-game fixture: {}", path.display());
    return;
}
let original = std::fs::read(&path).expect("read CommonEvent fixture");
```

- [ ] **Step 2: Rendre la fixture Map optionnelle**

```rust
let path = fixture_path("Densyanai_Inko_ver2.0/Data/MapData/Map001.mps");
if !path.exists() {
    eprintln!("skipping optional real-game fixture: {}", path.display());
    return;
}
let original = std::fs::read(&path).expect("read Map001 fixture");
```

- [ ] **Step 3: Corriger l'avertissement Rust**

Renommer `test_wolf_sysS_before_sys` en `test_wolf_sys_s_before_sys` sans modifier le corps du test.

- [ ] **Step 4: Vérifier**

Run: `cargo test --manifest-path src-tauri/Cargo.toml`

Expected: 418 tests ou plus passent, aucun échec dû à une fixture absente.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/engines/wolf/v3_format/common_events.rs src-tauri/src/engines/wolf/v3_format/map.rs src-tauri/src/engines/wolf/placeholders.rs
git commit -m "test: make optional wolf fixtures portable"
```

---

### Task 2: Recentrer la documentation sur le produit personnel

**Files:**
- Modify: `ROADMAP.md`
- Modify: `BACKLOG.md`
- Modify: `CONTEXT.md`
- Modify: `README.md`
- Modify: `README.fr.md`
- Modify: `tasks/todo.md`
- Delete: `docs/promo-plan.md`
- Delete: `docs/itch-distribution.md`

- [ ] **Step 1: Remplacer les objectifs commerciaux**

Le nouveau début de `ROADMAP.md` doit être :

```markdown
# Hoshi2Star — Personal Game Translation Roadmap

## Objectif

Traduire localement des jeux possédés par l'utilisateur, par patch structuré quand possible et par capture/OCR quand nécessaire. La qualité, la réversibilité et le plaisir d'utilisation priment sur la monétisation.

## Priorités

1. Langues configurables et providers LLM interchangeables.
2. Player Mode universel à la demande.
3. Overlay automatique et cache runtime.
4. Traduction d'images et remplacements d'assets réversibles.
5. Adaptateurs Unity, Bakin et moteurs supplémentaires.
```

- [ ] **Step 2: Réduire le todo actif**

Conserver seulement les tâches non terminées et un lien vers Git pour l'historique. Retirer les sections complétées, la monétisation, les tests beta et la collaboration multi-traducteurs.

- [ ] **Step 3: Retirer la promotion et la distribution commerciale**

Supprimer `docs/promo-plan.md` et `docs/itch-distribution.md`. Dans `AboutModal.tsx`, prévoir le retrait des adresses de donation dans Task 11, pas dans ce commit documentaire.

- [ ] **Step 4: Vérifier les références mortes**

Run: `rg -n "promo-plan|itch-distribution|Polar|LemonSqueezy|free tier|payants" README.md README.fr.md ROADMAP.md BACKLOG.md CONTEXT.md tasks docs --glob '!docs/journal/**' --glob '!docs/plans/**'`

Expected: aucun résultat dans la documentation active.

- [ ] **Step 5: Commit**

```bash
git add README.md README.fr.md ROADMAP.md BACKLOG.md CONTEXT.md tasks/todo.md docs/promo-plan.md docs/itch-distribution.md
git commit -m "docs: pivot roadmap to personal game translation"
```

---

### Task 3: Attacher les langues à chaque projet

**Files:**
- Create: `src-tauri/migrations/0006_project_languages.sql`
- Modify: `src-tauri/src/domain/types.rs`
- Modify: `src-tauri/src/commands/project.rs`
- Modify: `src/lib/types.ts`
- Test: `src-tauri/src/commands/project.rs`

- [ ] **Step 1: Écrire le test de migration**

Dans les tests DB, insérer un projet puis vérifier les défauts :

```rust
let pair: (String, String) = sqlx::query_as(
    "SELECT source_lang, target_lang FROM projects WHERE id = 'p1'",
)
.fetch_one(&pool)
.await
.unwrap();
assert_eq!(pair, ("ja".into(), "en".into()));
```

- [ ] **Step 2: Créer la migration**

```sql
ALTER TABLE projects ADD COLUMN source_lang TEXT NOT NULL DEFAULT 'ja';
ALTER TABLE projects ADD COLUMN target_lang TEXT NOT NULL DEFAULT 'en';
```

- [ ] **Step 3: Étendre le domaine**

```rust
pub struct Project {
    pub id: String,
    pub name: String,
    pub engine: String,
    pub game_path: String,
    pub source_lang: String,
    pub target_lang: String,
    pub created_at: String,
    pub updated_at: String,
}

impl Project {
    pub fn lang_pair(&self) -> String {
        format!("{}-{}", self.source_lang, self.target_lang)
    }
}
```

Ajouter les mêmes champs camelCase à l'interface TypeScript `Project`.

- [ ] **Step 4: Faire accepter les langues à `open_project`**

Ajouter `source_lang: Option<String>` et `target_lang: Option<String>` à la commande. Pour un nouveau projet, valider une chaîne BCP-47 simple et insérer les valeurs; pour une restauration, garder celles de la DB.

```rust
fn valid_lang(code: &str) -> bool {
    let len = code.len();
    (2..=15).contains(&len)
        && code.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}
```

- [ ] **Step 5: Vérifier**

Run: `cargo test --manifest-path src-tauri/Cargo.toml db::pool commands::project`

Expected: migration, création et restauration passent.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/migrations/0006_project_languages.sql src-tauri/src/domain/types.rs src-tauri/src/commands/project.rs src/lib/types.ts
git commit -m "feat(project): persist source and target languages"
```

---

### Task 4: Remplacer tous les `ja-en` câblés en dur

**Files:**
- Modify: `src-tauri/src/commands/translate.rs`
- Modify: `src-tauri/src/commands/glossary.rs`
- Modify: `src-tauri/src/commands/pack.rs`
- Modify: `src/components/editor/TMPanel.tsx`
- Modify: `src/components/editor/SegmentGrid.tsx`
- Modify: `src/components/editor/GlossaryPanel.tsx`
- Modify: `src/components/AppToolbar.tsx`
- Modify: `src/components/PackExportDialog.tsx`
- Modify: `src/components/PackImportWizard.tsx`
- Modify: `src/hooks/useAppHandlers.ts`
- Modify: `src/App.tsx`

- [ ] **Step 1: Ajouter un sélecteur de paire active**

```ts
export const useActiveLangPair = () =>
  useProjectStore((s) => {
    const project = s.projects.find((p) => p.id === s.activeProjectId);
    return project ? `${project.sourceLang}-${project.targetLang}` : "ja-fr";
  });
```

- [ ] **Step 2: Résoudre le contexte dans Rust depuis la DB**

Créer un helper privé réutilisé par les deux commandes de traduction :

```rust
struct ProjectLlmContext {
    id: String,
    engine: String,
    source_lang: String,
    target_lang: String,
}

impl ProjectLlmContext {
    fn lang_pair(&self) -> String {
        format!("{}-{}", self.source_lang, self.target_lang)
    }
}
```

La requête doit sélectionner `p.id, p.engine, p.source_lang, p.target_lang` depuis le segment ou le `project_id`.

- [ ] **Step 3: Remplacer les props et arguments fixes**

Tous les appels `langPair: "ja-en"` et `langPair="ja-en"` utilisent `useActiveLangPair()` ou `project.langPair` calculé. Les `TranslationContext` Rust utilisent les langues de `ProjectLlmContext`.

- [ ] **Step 4: Verrouiller avec une recherche**

Run: `rg -n 'ja-en|source_lang: "ja"|target_lang: "en"|sourceLang: "ja"|targetLang: "en"' src src-tauri/src --glob '!**/*test*'`

Expected: aucun hardcode de production; les valeurs par défaut et fixtures de tests restent permises.

- [ ] **Step 5: Commit**

```bash
git add src src-tauri/src/commands
git commit -m "refactor(i18n): derive language pair from project"
```

---

### Task 5: Définir les préréglages sans multiplier les providers

**Files:**
- Create: `src/lib/providerPresets.ts`
- Modify: `src/lib/types.ts`
- Modify: `src/lib/constants.ts`
- Test: `src/lib/providerPresets.test.ts`

- [ ] **Step 1: Écrire les types et préréglages**

```ts
export type ProviderPresetId =
  | "ollama"
  | "lm_studio"
  | "hugging_face"
  | "openai"
  | "openrouter"
  | "custom";

export interface ProviderPreset {
  id: ProviderPresetId;
  label: string;
  baseUrl: string;
  apiKeyRequired: boolean;
  modelHint: string;
}

export const PROVIDER_PRESETS: readonly ProviderPreset[] = [
  { id: "ollama", label: "Ollama", baseUrl: "http://localhost:11434/v1", apiKeyRequired: false, modelHint: "qwen3:8b" },
  { id: "lm_studio", label: "LM Studio", baseUrl: "http://localhost:1234/v1", apiKeyRequired: false, modelHint: "local-model" },
  { id: "hugging_face", label: "Hugging Face", baseUrl: "https://router.huggingface.co/v1", apiKeyRequired: true, modelHint: "Qwen/Qwen3-8B:provider" },
  { id: "openai", label: "OpenAI", baseUrl: "https://api.openai.com/v1", apiKeyRequired: true, modelHint: "gpt-5-mini" },
  { id: "openrouter", label: "OpenRouter", baseUrl: "https://openrouter.ai/api/v1", apiKeyRequired: true, modelHint: "provider/model" },
  { id: "custom", label: "Compatible OpenAI", baseUrl: "", apiKeyRequired: false, modelHint: "model-id" },
] as const;
```

- [ ] **Step 2: Généraliser la configuration**

```ts
export interface ProviderConfig {
  baseUrl: string;
  model: string;
  apiKey?: string;
  batchSize: number;
}
```

Le `presetId` reste un choix UI dans `AppSettings`; Rust n'en dépend jamais.

- [ ] **Step 3: Tester les contraintes de données**

```ts
it("has unique ids and normalized v1 base URLs", () => {
  expect(new Set(PROVIDER_PRESETS.map((p) => p.id)).size).toBe(PROVIDER_PRESETS.length);
  for (const p of PROVIDER_PRESETS.filter((p) => p.baseUrl)) {
    expect(p.baseUrl.endsWith("/v1")).toBe(true);
  }
});
```

- [ ] **Step 4: Commit**

```bash
git add src/lib/providerPresets.ts src/lib/providerPresets.test.ts src/lib/types.ts src/lib/constants.ts
git commit -m "feat(llm): define OpenAI-compatible provider presets"
```

---

### Task 6: Implémenter le provider OpenAI-compatible unique

**Files:**
- Modify: `src-tauri/src/llm/provider.rs`
- Create: `src-tauri/src/llm/openai_compatible.rs`
- Modify: `src-tauri/src/llm/mod.rs`
- Modify: `src-tauri/src/domain/types.rs`
- Test: `src-tauri/src/llm/openai_compatible.rs`

- [ ] **Step 1: Déplacer la logique de prompt hors du transport**

Ajouter dans `provider.rs` :

```rust
pub struct ChatMessages {
    pub system: String,
    pub user: String,
}

pub fn build_translation_messages(
    segments: &[String],
    context: &TranslationContext,
) -> ChatMessages {
    // reprendre exactement le rendu TOML, le glossaire, la numérotation et le marqueur ⏎ actuels
}
```

Conserver `parse_numbered_response` et `strip_think_blocks` dans ce module, avec leurs tests existants.

- [ ] **Step 2: Généraliser `ProviderConfig` Rust**

```rust
pub struct ProviderConfig {
    pub base_url: String,
    pub model: String,
    pub api_key: Option<String>,
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}
```

- [ ] **Step 3: Écrire le client HTTP minimal**

```rust
pub struct OpenAiCompatibleProvider {
    base_url: String,
    model: String,
    api_key: Option<String>,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
    temperature: f32,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    usage: Option<Usage>,
}
```

Utiliser `POST {base_url}/chat/completions`. Ajouter `Authorization: Bearer` seulement si la clé est non vide. Vérifier le statut HTTP avant le parsing JSON et ne jamais inclure la clé dans une erreur.

- [ ] **Step 4: Tester Ollama/LM Studio/HF au niveau contrat**

Avec `httpmock`, couvrir : succès, liste vide de choices, 401, 429 avec `Retry-After`, 500, réponse non JSON, bloc `<think>`, traduction multilignes et header Bearer présent/absent.

- [ ] **Step 5: Supprimer `OllamaProvider` après parité**

Le module ne doit plus contenir `/api/chat` ni `/api/tags`.

- [ ] **Step 6: Vérifier**

Run: `cargo test --manifest-path src-tauri/Cargo.toml llm::openai_compatible llm::provider`

Expected: tous les tests provider passent.

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/llm src-tauri/src/domain/types.rs
git commit -m "feat(llm): unify providers on OpenAI-compatible chat"
```

---

### Task 7: Unifier découverte de modèles, santé et création du provider

**Files:**
- Modify: `src-tauri/src/commands/translate.rs`
- Modify: `src-tauri/src/commands/glossary.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/commands/translate.rs`

- [ ] **Step 1: Ajouter un constructeur commun**

```rust
fn provider_from_config(config: &ProviderConfig) -> Result<OpenAiCompatibleProvider, String> {
    OpenAiCompatibleProvider::new(
        &config.base_url,
        &config.model,
        config.api_key.as_deref(),
        Duration::from_secs(180),
    )
    .map_err(|e| e.to_string())
}
```

- [ ] **Step 2: Remplacer les trois instanciations Ollama**

`translate_segments`, `translate_all_segments` et `extract_glossary_terms` utilisent le helper commun. Le message d'erreur devient : `Fournisseur LLM inaccessible ({base_url}) : {cause}`.

- [ ] **Step 3: Remplacer `get_ollama_models`**

```rust
#[tauri::command]
pub async fn list_provider_models(config: ProviderConfig) -> Result<Vec<String>, String> {
    provider_from_config(&config)?
        .list_models()
        .await
        .map_err(|e| e.to_string())
}
```

Le parseur accepte la forme OpenAI `{ "data": [{ "id": "..." }] }`. Une erreur de listing n'empêche jamais la saisie manuelle du modèle.

- [ ] **Step 4: Mettre à jour `generate_handler!`**

Retirer `get_ollama_models`, enregistrer `list_provider_models` dans l'unique handler de `lib.rs`.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/commands/translate.rs src-tauri/src/commands/glossary.rs src-tauri/src/lib.rs
git commit -m "refactor(llm): centralize provider construction and discovery"
```

---

### Task 8: Migrer les paramètres sans persister les secrets

**Files:**
- Modify: `src/stores/settings.ts`
- Modify: `src/stores/settings.test.ts`
- Modify: `src/stores/llm.ts`
- Modify: `src/stores/llm.test.ts`

- [ ] **Step 1: Définir les nouveaux paramètres**

```ts
export interface AppSettings {
  providerPresetId: ProviderPresetId;
  providerBaseUrl: string;
  providerModel: string;
  batchSize: number;
  defaultSourceLanguage: string;
  defaultTargetLanguage: string;
  theme: Theme;
  language: Language;
}
```

Défaut personnel : source `ja`, cible `fr`, preset Ollama.

- [ ] **Step 2: Migrer les anciennes clés**

Lors du chargement, utiliser `provider_base_url` puis l'ancien `ollama_url`, et `provider_model` puis l'ancien `ollama_model`. Après sauvegarde réussie, supprimer les anciennes clés si l'API du store le permet.

- [ ] **Step 3: Garder la clé uniquement en mémoire**

`AppSettings` ne contient pas `apiKey`. `useLlmStore` contient `sessionApiKey`, remis à `undefined` au redémarrage et injecté dans `ProviderConfig` juste avant `invoke`.

- [ ] **Step 4: Tester l'absence de secret**

```ts
expect(storeData.has("api_key")).toBe(false);
expect(storeData.has("provider_api_key")).toBe(false);
```

Tester aussi la migration depuis `ollama_url` et `ollama_model`.

- [ ] **Step 5: Commit**

```bash
git add src/stores/settings.ts src/stores/settings.test.ts src/stores/llm.ts src/stores/llm.test.ts
git commit -m "feat(settings): migrate providers without persisting secrets"
```

---

### Task 9: Construire l'interface provider et langues minimale

**Files:**
- Modify: `src/components/settings/SettingsModal.tsx`
- Modify: `src/locales/fr.json`
- Modify: `src/locales/en.json`
- Test: `src/components/settings/SettingsModal.test.tsx`

- [ ] **Step 1: Écrire le test utilisateur**

Le test ouvre les paramètres, sélectionne LM Studio, vérifie l'URL `http://localhost:1234/v1`, saisit un modèle, choisit Français comme cible et sauvegarde. Un second test sélectionne Hugging Face et vérifie que le champ clé apparaît sans que sa valeur soit sauvegardée.

- [ ] **Step 2: Remplacer les champs Ollama**

Ordre du formulaire : fournisseur, URL, modèle, clé de session si nécessaire, langue source, langue cible, taille de lot. La sélection d'un preset change l'URL et le hint, mais ne remplace pas silencieusement un modèle personnalisé après édition.

- [ ] **Step 3: Rendre la saisie manuelle permanente**

Le champ modèle doit toujours être éditable. La liste de `/models` est une aide, pas un verrou. Le bouton « Tester » affiche connexion réussie même si la liste est vide.

- [ ] **Step 4: Nettoyer le doublon i18n**

Supprimer la seconde clé JSON `settings.save` et renommer tous les libellés `Ollama` en `Fournisseur LLM` sauf dans le nom du preset.

- [ ] **Step 5: Vérifier**

Run: `pnpm test -- SettingsModal settings llm && pnpm typecheck && pnpm lint`

Expected: tests et types passent; aucun nouvel avertissement ESLint.

- [ ] **Step 6: Commit**

```bash
git add src/components/settings/SettingsModal.tsx src/components/settings/SettingsModal.test.tsx src/locales/fr.json src/locales/en.json
git commit -m "feat(settings): add provider presets and project languages"
```

---

### Task 10: Retirer le cooldown manuel et ajouter le backoff HTTP

**Files:**
- Modify: `src-tauri/src/llm/pipeline.rs`
- Modify: `src-tauri/src/llm/progress.rs`
- Modify: `src-tauri/src/commands/translate.rs`
- Modify: `src/stores/llm.ts`
- Modify: `src/components/TranslateAllDialog.tsx`
- Modify: `src/components/AppToolbar.tsx`
- Modify: `src/locales/fr.json`
- Modify: `src/locales/en.json`

- [ ] **Step 1: Écrire les tests du retry HTTP**

Tester qu'un `429 Retry-After: 1` provoque un seul retry, qu'un `401` ne retry pas et qu'un `500` utilise un backoff borné. Injecter une stratégie d'attente dans les tests pour ne pas dormir réellement.

- [ ] **Step 2: Classifier les erreurs**

```rust
pub enum LlmError {
    Unauthorized,
    RateLimited { retry_after: Option<Duration> },
    Server { status: u16, message: String },
    Unavailable { message: String },
    ResponseFormat(String),
}
```

- [ ] **Step 3: Supprimer `CooldownState`**

Retirer `cooldown_threshold_secs`, `cooldown_duration_secs`, `CoolingPayload`, `h2s://llm/cooling`, `isCooling`, `cooldownRemaining` et `CooldownBadge`. `TranslateAllDialog` ne conserve que le résumé et la confirmation.

- [ ] **Step 4: Ajouter un backoff automatique borné**

Maximum trois appels par lot. Respecter `Retry-After` jusqu'à 60 secondes; sinon 1 s puis 2 s. Ne jamais retry 400/401/403. Le split adaptatif de batch reste inchangé pour les erreurs de format et de placeholders.

- [ ] **Step 5: Vérifier**

Run: `rg -n 'cooldown|Cooling|isCooling|remainingSecs' src src-tauri/src`

Expected: aucun résultat de production.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/llm src-tauri/src/commands/translate.rs src/stores/llm.ts src/components/TranslateAllDialog.tsx src/components/AppToolbar.tsx src/locales
git commit -m "refactor(llm): replace manual cooldown with HTTP backoff"
```

---

### Task 11: Réduire la surface produit sans supprimer les capacités utiles

**Files:**
- Modify: `src/components/AboutModal.tsx`
- Modify: `src/App.tsx`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Modify: `src-tauri/Cargo.toml`
- Modify: `package.json`

- [ ] **Step 1: Simplifier l'écran À propos**

Retirer les adresses Bitcoin/Ethereum et garder version, licence, dépôt et description personnelle.

- [ ] **Step 2: Garder les fonctions avancées mais les regrouper**

Conserver TMX, rapport QA, debug dump et packs `.h2s`, mais déplacer les actions rarement utilisées sous un menu « Outils ». Ne pas supprimer leur backend dans ce plan.

- [ ] **Step 3: Durcir Tauri sans casser l'app**

Passer `withGlobalTauri` à `false`. Définir une CSP minimale compatible avec Vite/Tauri et limiter la capability MCP à la fenêtre principale uniquement sous debug si la configuration générée le permet. Tester tous les `invoke`.

- [ ] **Step 4: Auditer les dépendances avant retrait**

Run: `pnpm dlx depcheck` et `cargo machete --manifest-path src-tauri/Cargo.toml` si disponibles. Ne retirer une dépendance qu'après `rg` et gate complet. Ne pas retirer React Query, Radix, resizable panels, opener, updater ou process : ils sont utilisés.

- [ ] **Step 5: Commit**

```bash
git add src/components/AboutModal.tsx src/App.tsx src-tauri/tauri.conf.json src-tauri/capabilities/default.json src-tauri/Cargo.toml package.json pnpm-lock.yaml src-tauri/Cargo.lock
git commit -m "refactor(app): reduce personal product surface"
```

---

### Task 12: Documenter et valider les providers de bout en bout

**Files:**
- Create: `docs/providers.md`
- Create: `docs/providers.fr.md`
- Modify: `README.md`
- Modify: `README.fr.md`
- Modify: `CHANGELOG.md`
- Test: `src-tauri/tests/provider_flow.rs`

- [ ] **Step 1: Ajouter un test d'intégration local**

Le test démarre un serveur mock OpenAI-compatible, crée un projet `ja-fr`, traduit un segment avec placeholder, vérifie la cible française, la TM `ja-fr` et l'absence d'entrée `ja-en`.

- [ ] **Step 2: Documenter les cinq configurations**

Inclure Ollama `http://localhost:11434/v1`, LM Studio `http://localhost:1234/v1`, Hugging Face `https://router.huggingface.co/v1`, OpenAI `https://api.openai.com/v1`, OpenRouter `https://openrouter.ai/api/v1` et Compatible personnalisé. Expliquer que la clé n'est conservée que pendant la session.

- [ ] **Step 3: Remplacer les guides RunPod**

Supprimer `docs/runpod.md` et `docs/runpod.fr.md`; le guide générique explique qu'un Ollama distant ou un vLLM/TGI distant utilise simplement une URL compatible OpenAI.

- [ ] **Step 4: Gate complet**

```bash
pnpm typecheck
pnpm test
pnpm lint
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

Expected: zéro erreur; les avertissements frontend préexistants sont soit corrigés, soit consignés sans augmentation.

- [ ] **Step 5: Validation manuelle**

Tester au minimum Ollama et LM Studio localement. Pour Hugging Face et un fournisseur cloud, utiliser un petit appel explicite après saisie d'une clé de test; ne jamais committer la clé ni les logs de requête.

- [ ] **Step 6: Commit**

```bash
git add docs/providers.md docs/providers.fr.md README.md README.fr.md CHANGELOG.md docs/runpod.md docs/runpod.fr.md src-tauri/tests/provider_flow.rs
git commit -m "docs: document multilingual provider workflow"
```

---

## Critères de sortie avant Player Mode

- Un projet peut être créé en japonais → français, anglais, espagnol ou autre code valide.
- TM, glossaire, QA et packs utilisent tous la paire du projet.
- Ollama et LM Studio passent par le même code Rust.
- Hugging Face, OpenAI et OpenRouter sont des préréglages, pas des implémentations distinctes.
- Une URL compatible personnalisée fonctionne avec saisie manuelle du modèle.
- Les clés API ne sont ni loggées ni persistées.
- Les erreurs 401, 429 et 5xx sont compréhensibles et ont une politique de retry bornée.
- Le workflow « Tout traduire » n'impose plus de pauses arbitraires.
- Les tests sont reproductibles sans jeux propriétaires présents dans `test/`.
- La documentation active ne parle plus de paywall, licence commerciale ou objectifs de vente.

## Suite

Une fois ces critères validés, exécuter d'abord `docs/superpowers/plans/2026-08-25-hoshi2star-ui-redesign.md`, puis reprendre `docs/superpowers/plans/2026-08-25-hoshi2star-personal-player-mode.md` en ne réalisant que P1 : capture à la demande, OCR et panneau compagnon. L'overlay, les sidecars de qualité, Unity/Bakin et l'édition d'images restent des phases séparées et ne doivent pas être développés en parallèle.

## Self-review

- Les langues sont traitées avant les providers, car le produit actuel traduit réellement vers l'anglais malgré l'interface française.
- Un seul protocole couvre Ollama, LM Studio, Hugging Face et le cloud actuel.
- Les runtimes de modèles ne sont pas embarqués; Hoshi2Star reste léger.
- Les secrets ne sont pas stockés en clair pour gagner une commodité marginale.
- Le cœur déjà testé n'est pas réécrit.
- Les outils personnels utiles restent accessibles sans encombrer le chemin principal.
- Les deux fixtures manquantes sont corrigées avant tout refactor, afin que le gate soit fiable.
