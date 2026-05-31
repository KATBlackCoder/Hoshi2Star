# Plan F3-09 — Settings Modal (tauri-plugin-store)

## Objectif

Implémenter un modal de paramètres accessible via un bouton ⚙ dans la toolbar,
persisté sur disque via `tauri-plugin-store`. Remplace et absorbe le `LlmConfigModal`
existant (actuellement accessible uniquement via le bouton "Traduire" — UX incorrecte).

**Contenu du modal :**
- Section LLM : URL Ollama + sélecteur de modèle (avec fetch dynamique)
- Section Apparence : bascule light / dark (prévisualisation immédiate)
- Section Langue : bascule FR / EN (prévisualisation immédiate)
- Boutons : Réinitialiser · Annuler · Enregistrer

**Persistence :** `tauri-plugin-store` → fichier `settings.json` dans
`~/.local/share/hoshi2star/` (Linux). Les changements ne sont écrits sur disque
qu'au clic sur Enregistrer. Annuler restaure l'état précédent, y compris les
prévisualisations de thème et de langue.

## Statut : [x] Complété — 2026-05-31

## Prérequis

- `tauri-plugin-dialog` déjà installé (pattern d'installation connu) ✅
- `get_ollama_models` command déjà implémentée dans `commands/project.rs` ✅
- `LlmConfigModal` existant dans `App.tsx` (à supprimer) ✅
- shadcn `Button`, `Input`, `Select` déjà disponibles ✅
- Système i18n `react-i18next` en place avec `en.json` / `fr.json` ✅
- `useLlmStore.setProviderConfig` pour mettre à jour la config LLM ✅

## Estimation

4 steps · ~90–120 min total

## Items ROADMAP concernés

Aucun item existant ne couvre cette feature. Ajouter sous F3 après complétion :
```
### Polissage UI
- [x] Settings modal — Ollama URL/modèle, thème, langue, persistence tauri-plugin-store
```

---

## Steps

---

### Step 1 — Installer tauri-plugin-store (Rust + JS + capabilities)

**Objectif :** Ajouter le plugin au projet côté Rust et JS, l'enregistrer dans `lib.rs`,
et déclarer les permissions dans `capabilities/default.json`.

**Fichiers touchés :**
- `src-tauri/Cargo.toml` ← ajouter dépendance
- `src-tauri/src/lib.rs` ← enregistrer le plugin
- `src-tauri/capabilities/default.json` ← ajouter permissions store
- `package.json` (via pnpm) ← ajouter `@tauri-apps/plugin-store`

**Dépend de :** *(aucun — installation autonome)*

Tâches :

- [ ] Ajouter la dépendance Rust dans `src-tauri/Cargo.toml` :
  ```toml
  tauri-plugin-store = "2"
  ```

- [ ] Installer le package JS :
  ```bash
  pnpm add @tauri-apps/plugin-store
  ```

- [ ] Enregistrer le plugin dans `src-tauri/src/lib.rs`, après `tauri_plugin_dialog::init()` :
  ```rust
  .plugin(tauri_plugin_store::Builder::default().build())
  ```

- [ ] Ajouter les permissions dans `src-tauri/capabilities/default.json` :
  ```json
  "store:allow-load",
  "store:allow-get",
  "store:allow-set",
  "store:allow-save"
  ```

Test de validation :
```bash
cargo build --manifest-path src-tauri/Cargo.toml
pnpm typecheck
```
Résultat attendu : build Rust OK, 0 erreur TS.

Commit message : `feat(deps): add tauri-plugin-store v2 — Rust + JS + capabilities`

---

### Step 2 — Settings store (src/stores/settings.ts)

**Objectif :** Créer un store Zustand qui encapsule les lectures/écritures
`tauri-plugin-store`. Ce store est la source de vérité pour tous les paramètres
persistés. Il synchronise aussi les effets de bord (llmStore, i18n, classe DOM `.dark`).

**Fichiers touchés :**
- `src/stores/settings.ts` ← nouveau fichier

**Dépend de :** Step 1

**Décision de design — pas de `zustand/persist` ici :** `tauri-plugin-store` stocke dans
les données de l'application OS (pas dans le cache WebKit). C'est le bon niveau pour une
app desktop. `zustand/persist` n'est pas utilisé pour ne pas dupliquer les données
entre localStorage et settings.json.

**Décision thème — classe DOM immédiate :** Le thème s'applique via
`document.documentElement.classList` pour que shadcn CSS variables fonctionnent.
Cette application est séparée de la persistence (on peut prévisualiser sans sauvegarder).

Tâches :

- [ ] Déclarer les types et constantes :
  ```typescript
  export type Theme = 'light' | 'dark'
  export type Language = 'fr' | 'en'

  export interface AppSettings {
    ollamaUrl: string
    ollamaModel: string
    theme: Theme
    language: Language
  }

  export const DEFAULT_SETTINGS: AppSettings = {
    ollamaUrl: 'http://localhost:11434',
    ollamaModel: 'qwen3:4b-instruct-2507-q8_0',
    theme: 'dark',
    language: 'fr',
  }

  const STORE_FILE = 'settings.json'
  ```

- [ ] Helper DOM (non exporté — usage interne + depuis le modal pour preview) :
  ```typescript
  export function applyThemeToDom(theme: Theme) {
    document.documentElement.classList.toggle('dark', theme === 'dark')
  }
  ```

- [ ] Interface du store :
  ```typescript
  interface SettingsState {
    settings: AppSettings
    loadSettings: () => Promise<void>
    saveSettings: (draft: AppSettings) => Promise<void>
  }
  ```

- [ ] Implémenter `loadSettings` :
  - Ouvrir le store : `await load(STORE_FILE, { autoSave: false })`
  - Lire chaque clé avec `await store.get<string>('ollama_url')` etc.
  - Merger avec `DEFAULT_SETTINGS` (les clés absentes au premier lancement gardent le défaut)
  - Mettre à jour `settings` dans le state Zustand
  - Effets de bord :
    - `useLlmStore.getState().setProviderConfig({ url, model })`
    - `i18n.changeLanguage(language)` (import `i18next` direct — pas le hook)
    - `applyThemeToDom(theme)`

- [ ] Implémenter `saveSettings` :
  - Ouvrir le store
  - `await store.set('ollama_url', draft.ollamaUrl)` pour chaque clé
  - `await store.save()` — écriture effective sur disque
  - Mettre à jour `settings` dans le state Zustand
  - Mêmes effets de bord que `loadSettings` (url/model → llmStore, langue → i18n, thème → DOM)

- [ ] Exporter le selector :
  ```typescript
  export const useSettings = () => useSettingsStore((s) => s.settings)
  ```

**Note clés de store :** Utiliser `snake_case` pour les clés (`'ollama_url'`, `'ollama_model'`,
`'theme'`, `'language'`) — cohérent avec la convention Rust et lisible si l'utilisateur
édite manuellement le fichier `settings.json`.

Test de validation :
```bash
pnpm typecheck
```
Résultat attendu : 0 erreur TS. Vérification du fichier généré via `pnpm tauri dev`
(lancer l'app, cliquer Save → vérifier `~/.local/share/hoshi2star/settings.json`).

Commit message : `feat(stores): add settings.ts — tauri-plugin-store persistence (url, model, theme, lang)`

---

### Step 3 — Composant SettingsModal

**Objectif :** Créer le modal de paramètres avec les 3 sections (LLM, Apparence, Langue)
et les 3 boutons (Réinitialiser, Annuler, Enregistrer). Le thème et la langue ont une
prévisualisation immédiate — Annuler les restaure.

**Fichiers touchés :**
- `src/components/settings/SettingsModal.tsx` ← nouveau fichier

**Dépend de :** Step 2

**Décision preview vs staged :**
- **Thème et Langue :** s'appliquent immédiatement au clic (via `applyThemeToDom` et
  `i18n.changeLanguage`) pour permettre la prévisualisation. Annuler restaure les valeurs
  d'origine sauvegardées dans `originalSettings`.
- **URL et Modèle :** staged uniquement dans le draft local — aucun effet immédiat.
  Le backend n'est pas contacté tant que l'utilisateur n'a pas cliqué Enregistrer.

Tâches :

- [ ] Props et state interne :
  ```typescript
  interface SettingsModalProps {
    open: boolean
    onClose: () => void
  }

  // Dans le composant :
  const currentSettings = useSettings()
  const [draft, setDraft] = useState<AppSettings>(currentSettings)
  const [originalSettings] = useState<AppSettings>(currentSettings)
  // originalSettings capturé à l'ouverture du modal — ref stable
  ```

- [ ] Fetch des modèles Ollama (logique identique à l'ancien `LlmConfigModal`) :
  - State local : `models: string[]`, `modelsLoading: boolean`, `modelsError: string | null`
  - Fonction `fetchModels(url: string)` : `invoke<string[]>('get_ollama_models', { url })`
  - `useEffect(() => { fetchModels(draft.ollamaUrl) }, [])` — au montage uniquement
  - Appel manuel via bouton "Test" sur blur de l'URL

- [ ] Rendu — Section LLM :
  ```tsx
  <section>
    <h3>{t('settings.llm.section')}</h3>
    {/* URL */}
    <label>{t('settings.llm.urlLabel')}</label>
    <Input
      value={draft.ollamaUrl}
      onChange={(e) => setDraft(d => ({ ...d, ollamaUrl: e.target.value }))}
      onBlur={() => fetchModels(draft.ollamaUrl)}
    />
    {/* Modèle */}
    <label>{t('settings.llm.modelLabel')}</label>
    {/* Select si models.length > 0, Input manuel sinon — même logique que LlmConfigModal */}
  </section>
  ```

- [ ] Rendu — Section Apparence (thème) :
  - Deux boutons shadcn : variant `default` si actif, `outline` si inactif
  - Au clic : `setDraft(d => ({ ...d, theme: 'dark' }))` + `applyThemeToDom('dark')`
  ```tsx
  <section>
    <h3>{t('settings.appearance.section')}</h3>
    <div className="flex gap-2">
      <Button
        variant={draft.theme === 'light' ? 'default' : 'outline'}
        onClick={() => { setDraft(d => ({ ...d, theme: 'light' })); applyThemeToDom('light') }}
      >
        <Sun className="h-4 w-4 mr-1.5" /> {t('settings.appearance.light')}
      </Button>
      <Button
        variant={draft.theme === 'dark' ? 'default' : 'outline'}
        onClick={() => { setDraft(d => ({ ...d, theme: 'dark' })); applyThemeToDom('dark') }}
      >
        <Moon className="h-4 w-4 mr-1.5" /> {t('settings.appearance.dark')}
      </Button>
    </div>
  </section>
  ```

- [ ] Rendu — Section Langue :
  - Même pattern que thème (deux boutons, preview immédiate)
  - Au clic : `setDraft(d => ({ ...d, language: 'fr' }))` + `i18n.changeLanguage('fr')`

- [ ] Footer — trois boutons :
  ```tsx
  {/* Réinitialiser */}
  <Button variant="ghost" onClick={() => {
    setDraft(DEFAULT_SETTINGS)
    applyThemeToDom(DEFAULT_SETTINGS.theme)
    i18n.changeLanguage(DEFAULT_SETTINGS.language)
  }}>
    {t('settings.reset')}
  </Button>

  {/* Annuler — restaure les previews */}
  <Button variant="outline" onClick={() => {
    applyThemeToDom(originalSettings.theme)
    i18n.changeLanguage(originalSettings.language)
    onClose()
  }}>
    {t('settings.cancel')}
  </Button>

  {/* Enregistrer */}
  <Button onClick={async () => {
    await saveSettings(draft)
    toast.success(t('settings.saved'))
    onClose()
  }}>
    {t('settings.save')}
  </Button>
  ```

- [ ] Utiliser le composant `AlertDialog` de shadcn comme wrapper du modal (déjà installé)
  OU une `<div>` fixed overlay comme le `LlmConfigModal` existant (même style, cohérence).
  **Préférer le pattern `fixed inset-0` déjà établi** — pas de nouvelle dépendance shadcn.

Test de validation :
```bash
pnpm typecheck
```
Vérification manuelle (app lancée) :
- Ouvrir le modal → changer thème → prévisualisation immédiate
- Cliquer Annuler → thème revient à l'état d'origine
- Cliquer Enregistrer → thème persisté, `settings.json` mis à jour sur disque

Commit message : `feat(ui): SettingsModal — LLM, theme, language, reset/cancel/save`

---

### Step 4 — Câblage App.tsx + suppression LlmConfigModal + i18n

**Objectif :** Intégrer `SettingsModal` dans `App.tsx`, supprimer `LlmConfigModal` et
le bouton langue de la toolbar, appeler `loadSettings()` au montage, et ajouter toutes
les clés i18n.

**Fichiers touchés :**
- `src/App.tsx` ← supprimer LlmConfigModal + props, ajouter SettingsModal + loadSettings
- `src/locales/en.json` ← ajouter section `settings`, mettre à jour `noModelConfigured`
- `src/locales/fr.json` ← idem

**Dépend de :** Step 2, Step 3

Tâches App.tsx :

- [ ] Supprimer le composant `LlmConfigModal` (lignes 108–264) — entièrement remplacé
  par `SettingsModal`. Vérifier qu'aucun autre fichier ne l'importe avant suppression :
  ```bash
  grep -r "LlmConfigModal" src/
  ```

- [ ] Supprimer `showLlmConfig` state et `onOpenLlmConfig` prop dans `App` et `Toolbar`

- [ ] Dans `Toolbar` :
  - Supprimer le bouton langue (`🇬🇧 EN / 🇫🇷 FR`)
  - Supprimer les imports `i18n` de la toolbar (gardé uniquement dans SettingsModal)
  - Remplacer `onOpenLlmConfig` par `onOpenSettings` :
    ```tsx
    function Toolbar({ onOpenSettings }: { onOpenSettings: () => void }) {
    ```
  - Ajouter à droite de la toolbar (après le nom du projet) :
    ```tsx
    <Button variant="ghost" size="sm" className="h-7 w-7 p-0 ml-auto"
      onClick={onOpenSettings} title={t('settings.title')}>
      <Settings className="h-4 w-4" />
    </Button>
    ```
    Import lucide : `Settings` (pas de conflit avec le nom du store)

- [ ] Bouton Traduire — ne plus ouvrir de modal. Si `providerConfig.model` est vide,
  afficher un toast et ouvrir Settings automatiquement :
  ```tsx
  async function handleTranslate() {
    if (!providerConfig.model.trim()) {
      toast.warning(t('segmentGrid.noModelConfigured'))
      setShowSettings(true)
      return
    }
    void startTranslation([], activeFileId ?? undefined)
  }
  ```

- [ ] Dans `App` :
  - Remplacer `showLlmConfig` par `showSettings`
  - Ajouter `loadSettings()` au montage :
    ```tsx
    const { loadSettings } = useSettingsStore()
    useEffect(() => { void loadSettings() }, [loadSettings])
    ```
  - Remplacer `<LlmConfigModal .../>` par `<SettingsModal open={showSettings} onClose={() => setShowSettings(false)} />`

Tâches i18n :

- [ ] Dans `src/locales/en.json`, ajouter section `"settings"` et mettre à jour
  `segmentGrid.noModelConfigured` :
  ```json
  "settings": {
    "title": "Settings",
    "llm": {
      "section": "LLM (Ollama)",
      "urlLabel": "Ollama URL",
      "modelLabel": "Model",
      "modelLoading": "Loading models...",
      "modelNone": "No model available",
      "modelManual": "Enter model name manually",
      "modelError": "Cannot reach Ollama — check the URL",
      "testButton": "Test"
    },
    "appearance": {
      "section": "Appearance",
      "light": "Light",
      "dark": "Dark"
    },
    "language": {
      "section": "Language"
    },
    "reset": "Reset to defaults",
    "cancel": "Cancel",
    "save": "Save",
    "saved": "Settings saved"
  }
  ```
  Mettre à jour :
  ```json
  "noModelConfigured": "No LLM model configured — open Settings (⚙)"
  ```
  Laisser `"llmModal"` en place pour l'instant (nettoyage backlog — ne pas casser
  d'éventuelles traductions en cours).

- [ ] Dans `src/locales/fr.json`, même structure :
  ```json
  "settings": {
    "title": "Paramètres",
    "llm": {
      "section": "LLM (Ollama)",
      "urlLabel": "URL Ollama",
      "modelLabel": "Modèle",
      "modelLoading": "Chargement des modèles...",
      "modelNone": "Aucun modèle disponible",
      "modelManual": "Saisir le nom du modèle manuellement",
      "modelError": "Impossible de contacter Ollama — vérifier l'URL",
      "testButton": "Tester"
    },
    "appearance": {
      "section": "Apparence",
      "light": "Clair",
      "dark": "Sombre"
    },
    "language": {
      "section": "Langue"
    },
    "reset": "Réinitialiser",
    "cancel": "Annuler",
    "save": "Enregistrer",
    "saved": "Paramètres enregistrés"
  }
  ```
  Mettre à jour :
  ```json
  "noModelConfigured": "Aucun modèle LLM configuré — ouvrez les Paramètres (⚙)"
  ```

Test de validation :
```bash
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
pnpm typecheck
```
Résultat attendu : 0 erreur Rust, 0 erreur TS.

Vérification manuelle complète :
1. Lancer `pnpm tauri dev`
2. Cliquer ⚙ → modal s'ouvre avec l'URL et modèle actuels
3. Changer thème → prévisualisation immédiate sans fermer le modal
4. Cliquer Annuler → thème revient, modal se ferme
5. Changer URL, modèle, thème, langue → Enregistrer → fermeture
6. Relancer l'app → tous les paramètres restaurés depuis `settings.json`
7. Cliquer Traduire sans modèle → toast + Settings s'ouvre automatiquement
8. Bouton Réinitialiser → revient aux valeurs par défaut (dark, fr, localhost:11434, qwen3:4b)

Commit message : `feat(app): wire SettingsModal — remove LlmConfigModal, add loadSettings on mount, i18n en+fr`

---

## Tests obligatoires avant push GitHub

```bash
# Rust — build + tests + linting
cargo build --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo fmt --manifest-path src-tauri/Cargo.toml

# TypeScript
pnpm typecheck
```

## Mise à jour après complétion

- `ROADMAP.md` : ajouter + cocher "Settings modal" sous F3 / Polissage UI
- `CHANGELOG.md` : section `[Unreleased]` → entrée `Added` Settings modal + persistence
- `docs/journal/` : nouvelle entrée `YYYY-MM-DD-f3-09-settings-modal.md`
