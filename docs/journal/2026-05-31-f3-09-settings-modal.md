# Journal — 2026-05-31 — F3-09 Settings Modal

**Phase** : F3 — Polissage UI
**Durée estimée** : ~120 min
**Statut** : ✅ Complété

---

## Ce qui a été fait

### Step 1 — tauri-plugin-store v2.4.3
- Ajout `tauri-plugin-store = "2"` dans `src-tauri/Cargo.toml`
- Installation `@tauri-apps/plugin-store@2.4.3` via pnpm
- Enregistrement `.plugin(tauri_plugin_store::Builder::default().build())` dans `lib.rs`
- Ajout permissions `store:allow-load/get/set/save` dans `capabilities/default.json`

### Step 2 — src/stores/settings.ts
- Store Zustand avec `loadSettings` et `saveSettings`
- Persistence via `tauri-plugin-store` → `settings.json` dans le répertoire app data OS
- Clés snake_case (`ollama_url`, `ollama_model`, `theme`, `language`) — lisibles si édition manuelle
- `DEFAULT_SETTINGS` : dark, fr, localhost:11434, `qwen3:4b-instruct-2507-q8_0`
- Helper `applyThemeToDom` exporté — utilisé dans le modal pour prévisualisation immédiate
- Fix TypeScript : `StoreOptions.defaults` est requis → `{ defaults: {}, autoSave: false }`

### Step 3 — src/components/settings/SettingsModal.tsx
- Pattern `fixed inset-0` existant (cohérent avec l'ancien LlmConfigModal)
- 3 sections : LLM (url + modèle avec fetch dynamique), Apparence (thème), Langue
- Prévisualisation immédiate thème/langue — Annuler restaure les valeurs d'origine
- `originalSettings` capturé à l'ouverture (useState stable)
- Fetch modèles au montage (`useEffect [open]`) + bouton Tester + onBlur URL
- Auto-select `models[0]` si modèle actuel absent de la liste
- 3 boutons footer : Réinitialiser (DEFAULT_SETTINGS), Annuler (restaure preview), Enregistrer

### Step 4 — App.tsx + i18n
- Suppression complète de `LlmConfigModal` (108–264) — aucun autre fichier ne l'importait
- Suppression `showLlmConfig`, `onOpenLlmConfig`, bouton langue toolbar, import `i18n` toolbar
- `Toolbar` reçoit `onOpenSettings` + `onTranslate` au lieu de `onOpenLlmConfig`
- Bouton ⚙ à droite dans toolbar (`ml-auto`) avec icône `Settings` lucide
- `handleTranslate` : si `providerConfig.model` vide → toast + `setShowSettings(true)` ; sinon `startTranslation` directement
- `loadSettings()` dans `useEffect` au montage de `App` (dependency `[loadSettings]`)
- Ajout section `settings` i18n en.json + fr.json (title, llm.*, appearance.*, language.*, reset/cancel/save/saved)
- Mise à jour `segmentGrid.noModelConfigured` (référence Settings ⚙ au lieu du menu Traduire)

## Fichiers créés
- `src/stores/settings.ts`
- `src/components/settings/SettingsModal.tsx`

## Fichiers modifiés
- `src-tauri/Cargo.toml` — tauri-plugin-store = "2"
- `src-tauri/Cargo.lock` — lock file mis à jour
- `src-tauri/src/lib.rs` — plugin store enregistré
- `src-tauri/capabilities/default.json` — 4 permissions store
- `package.json` + `pnpm-lock.yaml` — @tauri-apps/plugin-store 2.4.3
- `src/App.tsx` — LlmConfigModal supprimé, SettingsModal câblé, loadSettings au montage
- `src/locales/en.json` — section settings + noModelConfigured
- `src/locales/fr.json` — section settings + noModelConfigured

## Décisions prises

- `StoreOptions.defaults` requis en v2.4.3 (non-optionnel dans le type TS) → `{ defaults: {}, autoSave: false }` pour contrôler manuellement la sauvegarde
- `originalSettings` capturé en useState (pas ref) pour stabilité si le store est mis à jour pendant que le modal est ouvert
- Fetch modèles déclenché sur `open` (pas `[]`) pour relancer à chaque ouverture du modal
- Bouton Traduire sans modal intermédiaire — UX plus directe quand le modèle est déjà configuré

## Résultats des tests

- `cargo test` : 163 passed, 0 failed
- `cargo clippy -D warnings` : 0 warning
- `cargo fmt --check` : OK
- `pnpm typecheck` : 0 erreur

## Tâches ROADMAP cochées

- [x] Settings modal — Ollama URL/modèle, thème, langue, persistence tauri-plugin-store (F3 / Polissage UI)

## Prochaine session

- Test manuel complet avec `pnpm tauri dev` (vérifier persistence settings.json, prévisualisation thème/langue, flow Traduire sans modèle)
- Beta privée : recrutement testeurs Discord/F95zone, feedback form
- LLM passe 2 (review / consistency)

---
*Généré par Claude Code — Hoshi2Star*
