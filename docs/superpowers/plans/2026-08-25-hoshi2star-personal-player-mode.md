# Hoshi2Star Personal Player Mode Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Transformer Hoshi2Star v0.4.10 en application locale personnelle capable de traduire des jeux déjà compilés pendant le jeu, de conserver le mode CAT/patch existant, de traduire des images et d'appliquer des remplacements d'assets réversibles fournis par l'utilisateur.

**Architecture:** Le mode CAT existant reste inchangé et devient la voie « Patch », toujours préférable quand les fichiers du moteur sont accessibles. Un nouveau « Player Mode » ajoute capture d'écran, OCR, cache/TM, traduction Ollama et affichage overlay ou panneau compagnon; les intégrations moteurs spécialisées réutilisent ce même pipeline. Les modifications de jeux passent obligatoirement par un `PatchPlan` avec hash, sauvegarde et restauration; aucun contournement DRM, archive protégée, anti-cheat ou injection dans un jeu multijoueur n'est implémenté.

**Tech Stack:** Tauri v2, Rust stable, React 19, TypeScript strict, Zustand, SQLite/sqlx, xcap, image/imageproc, Tesseract 5 puis PaddleOCR PP-OCRv5 via sidecar Python/uv, Ollama, Tauri global-shortcut, Tauri WebviewWindow overlay.

---

## Découpage des livrables

1. **P0 — Pivot et garde-fous** : documentation, modèle de données, conservation du mode CAT.
2. **P1 — Traducteur universel à la demande** : sélectionner une fenêtre/zone, capturer, OCR, traduire, afficher dans un panneau compagnon. Ce livrable fonctionne avec tous les moteurs et Proton.
3. **P2 — Overlay et mode automatique** : cache visuel, polling limité, sous-titres transparents et fallback panneau.
4. **P3 — Adaptateurs moteurs** : MV/MZ et Wolf existants, VX Ace réactivé, Unity via XUnity/BepInEx, Bakin via dictionnaire accessible.
5. **P4 — Traduction d'images et remplacements réversibles** : OCR par zones, effacement du texte, rendu traduit, couche d'override utilisateur.
6. **P5 — Stabilisation Linux/Windows** : Wayland/KDE/NVIDIA, X11, Windows, Wine/Proton, packaging des modèles OCR.

Chaque phase produit un logiciel testable indépendamment. Ne commencer P2 qu'après validation réelle de P1 sur au moins un jeu RPG Maker et un jeu sous Proton.

## Structure de fichiers

### Backend Rust

- Create: `src-tauri/src/player/mod.rs` — façade Player Mode.
- Create: `src-tauri/src/player/profile.rs` — profils de jeux et régions.
- Create: `src-tauri/src/player/capture.rs` — trait de capture et backend xcap.
- Create: `src-tauri/src/player/preprocess.rs` — crop, agrandissement, contraste et hash visuel.
- Create: `src-tauri/src/player/ocr.rs` — trait OCR et provider Tesseract.
- Create: `src-tauri/src/player/translate.rs` — TM → glossaire → Ollama pour une observation runtime.
- Create: `src-tauri/src/player/session.rs` — boucle annulable et événements Tauri.
- Create: `src-tauri/src/player/overlay.rs` — création/positionnement de fenêtres overlay.
- Create: `src-tauri/src/player/image_translate.rs` — description des régions d'image et résultat rendu.
- Create: `src-tauri/src/player/overrides.rs` — plan, backup, application et restauration d'assets.
- Create: `src-tauri/src/commands/player.rs` — commandes IPC Player Mode.
- Create: `src-tauri/src/commands/image.rs` — commandes IPC image/override.
- Create: `src-tauri/src/engines/unity/mod.rs` — détection Unity et profil d'intégration externe.
- Create: `src-tauri/src/engines/bakin/mod.rs` — détection Bakin et dictionnaire accessible.
- Create: `src-tauri/migrations/0006_player_mode.sql` — tables du Player Mode.
- Modify: `src-tauri/src/lib.rs` — plugins, AppState, commandes et événements.
- Modify: `src-tauri/src/state.rs` — registre de sessions annulables.
- Modify: `src-tauri/src/domain/types.rs` — types IPC camelCase.
- Modify: `src-tauri/src/engines/detector.rs` — sondes Unity/Bakin et réactivation VX Ace.
- Modify: `src-tauri/Cargo.toml` — dépendances de capture/image/hotkey.
- Modify: `src-tauri/capabilities/default.json` — permissions minimales.

### Frontend React

- Create: `src/stores/player.ts` — profils, session, observation et overlay.
- Create: `src/stores/player.test.ts` — transitions du store.
- Create: `src/features/player/PlayerHome.tsx` — choix du jeu et mode de fonctionnement.
- Create: `src/features/player/RegionPicker.tsx` — sélection et test d'une zone.
- Create: `src/features/player/LiveTranslationPanel.tsx` — source, cible, historique et corrections.
- Create: `src/features/player/OverlayView.tsx` — contenu d'une fenêtre overlay.
- Create: `src/features/player/PlayerSettings.tsx` — fréquence, OCR, source/cible, hotkeys.
- Create: `src/features/image/ImageTranslator.tsx` — édition d'une image.
- Create: `src/features/image/OverrideManager.tsx` — liste, activation et restauration.
- Create: `src/components/ModeSwitcher.tsx` — CAT / Player / Image.
- Modify: `src/App.tsx` — router interne par mode.
- Modify: `src/lib/types.ts` — miroirs des types Rust.
- Modify: `src/stores/settings.ts` — langue cible et réglages OCR globaux.
- Modify: `src/locales/fr.json` et `src/locales/en.json` — chaînes de l'interface.

### Tests et documentation

- Create: `src-tauri/tests/fixtures/ocr/japanese-dialogue.png` — fixture synthétique libre.
- Create: `src-tauri/tests/player_pipeline.rs` — capture fixture → OCR simulé → TM/LLM simulé.
- Create: `docs/adr/ADR-008-personal-player-mode.md` — décision de pivot.
- Create: `docs/player-mode.md` — utilisation Linux/Wayland, Windows et Proton.
- Create: `docs/engine-support.md` — matrice patch/hook/OCR.
- Modify: `ROADMAP.md`, `README.md`, `README.fr.md`, `CONTEXT.md`, `CHANGELOG.md`.

---

### Task 1: Documenter le pivot et verrouiller le périmètre

**Files:**
- Create: `docs/adr/ADR-008-personal-player-mode.md`
- Modify: `ROADMAP.md`
- Modify: `CONTEXT.md`
- Test: documentation review only

- [ ] **Step 1: Écrire l'ADR**

Inclure explicitement ces décisions : conserver le CAT existant; priorité au patch statique puis à l'OCR; local-first/Ollama; pas de monétisation; pas de DRM/anti-cheat/multijoueur; les remplacements « uncensor » sont des images fournies par l'utilisateur et jamais une reconstruction automatique de contenu caché.

- [ ] **Step 2: Remplacer les objectifs commerciaux de la roadmap**

Retirer les tâches licence/prix/bêta payante et introduire P0 à P5 avec le critère principal : « le propriétaire peut jouer du début à la fin dans sa langue avec une restauration fiable des fichiers originaux ».

- [ ] **Step 3: Vérifier qu'aucune ancienne promesse commerciale ne reste active**

Run: `rg -n "Polar|LemonSqueezy|payant|29 \\$|free tier|Indie tier" ROADMAP.md CONTEXT.md`

Expected: aucune ligne active; les mentions historiques éventuelles sont clairement marquées comme abandonnées.

- [ ] **Step 4: Commit**

```bash
git add docs/adr/ADR-008-personal-player-mode.md ROADMAP.md CONTEXT.md
git commit -m "docs: pivot Hoshi2Star to personal player translation"
```

### Task 2: Ajouter le schéma Player Mode

**Files:**
- Create: `src-tauri/migrations/0006_player_mode.sql`
- Modify: `src-tauri/src/domain/types.rs`
- Test: `src-tauri/src/domain/types.rs`

- [ ] **Step 1: Écrire un test de sérialisation IPC qui échoue**

Le test doit sérialiser un `PlayerProfile` et vérifier les clés `executablePath`, `captureMode`, `sourceLang` et `targetLang`.

- [ ] **Step 2: Ajouter la migration**

Créer les tables `player_profiles`, `capture_regions`, `runtime_observations` et `asset_overrides`. Utiliser `ON DELETE CASCADE`, indexer `profile_id` et rendre unique `(profile_id, source_hash)` dans `runtime_observations`.

Le modèle minimal est :

```rust
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
#[serde(rename_all = "camelCase")]
pub struct PlayerProfile {
    pub id: String,
    pub name: String,
    pub game_path: String,
    pub executable_path: String,
    pub engine: String,
    pub launch_kind: String,
    pub capture_mode: String,
    pub source_lang: String,
    pub target_lang: String,
    pub created_at: String,
    pub updated_at: String,
}
```

- [ ] **Step 3: Lancer les tests de migration et sérialisation**

Run: `cargo test --manifest-path src-tauri/Cargo.toml player_profile`

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/migrations/0006_player_mode.sql src-tauri/src/domain/types.rs
git commit -m "feat(player): add profile and runtime observation schema"
```

### Task 3: Prototyper la capture Linux Wayland/X11 et Windows

**Files:**
- Create: `src-tauri/src/player/capture.rs`
- Create: `src-tauri/src/player/preprocess.rs`
- Create: `src-tauri/src/player/mod.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/Cargo.toml`
- Test: `src-tauri/src/player/capture.rs`

- [ ] **Step 1: Ajouter les dépendances**

Ajouter `xcap`, `image`, `imageproc` et `img_hash`. Ne pas ajouter OpenCV au binaire Rust.

- [ ] **Step 2: Définir la frontière testable**

```rust
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRect { pub x: i32, pub y: i32, pub width: u32, pub height: u32 }

pub trait CaptureProvider: Send + Sync {
    fn list_targets(&self) -> Result<Vec<CaptureTarget>, CaptureError>;
    fn capture(&self, target_id: &str, rect: CaptureRect) -> Result<image::RgbaImage, CaptureError>;
}
```

- [ ] **Step 3: Écrire les tests purs de crop et hash visuel**

Vérifier qu'un crop hors limites retourne une erreur, que deux images identiques ont le même hash et qu'un changement de quelques pixels sous le seuil est ignoré.

- [ ] **Step 4: Implémenter `XcapProvider`**

Sur Wayland, accepter le dialogue de portail système et conserver le target sélectionné pour toute la session. Sur X11/Windows, énumérer les fenêtres. Retourner une erreur traduisible quand le portail/compositeur refuse la capture.

- [ ] **Step 5: Test manuel matériel**

Run: `pnpm tauri:linux`

Expected: une commande debug liste la fenêtre d'un jeu Proton et sauvegarde un crop PNG dans le dossier temporaire de l'application.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/Cargo.toml src-tauri/src/player src-tauri/src/lib.rs
git commit -m "feat(player): capture game regions on Wayland X11 and Windows"
```

### Task 4: Ajouter OCR Tesseract avec prétraitement

**Files:**
- Create: `src-tauri/src/player/ocr.rs`
- Modify: `src-tauri/src/player/preprocess.rs`
- Test: `src-tauri/src/player/ocr.rs`
- Test fixture: `src-tauri/tests/fixtures/ocr/japanese-dialogue.png`

- [ ] **Step 1: Définir le provider OCR**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OcrResult { pub text: String, pub confidence: f32, pub boxes: Vec<TextBox> }

pub trait OcrProvider: Send + Sync {
    fn recognize(&self, image: &image::DynamicImage, lang: &str) -> Result<OcrResult, OcrError>;
}
```

- [ ] **Step 2: Écrire le test avec un exécutable OCR simulé**

Le provider reçoit le chemin du binaire dans son constructeur. Le test fournit un script fixture qui renvoie `星を探す` et vérifie normalisation Unicode NFC, suppression des espaces parasites et conservation de la ponctuation japonaise.

- [ ] **Step 3: Implémenter `TesseractProvider`**

Écrire le PNG dans un `tempfile`, appeler `tesseract <png> stdout -l jpn --psm 6`, capturer stdout/stderr et supprimer le fichier par RAII. Ajouter les profils `jpn`, `jpn_vert`, `kor`, `chi_sim`, `chi_tra`.

- [ ] **Step 4: Ajouter trois variantes de prétraitement**

Produire `original_upscaled`, `grayscale_contrast` et `threshold_inverted`; retenir le résultat OCR non vide avec la meilleure confiance.

- [ ] **Step 5: Vérifier l'absence de pack japonais**

Quand `tesseract --list-langs` ne contient pas `jpn`, retourner `MissingLanguagePack("jpn")` avec une instruction d'installation adaptée à Arch/CachyOS, Debian et Windows.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/player/ocr.rs src-tauri/src/player/preprocess.rs src-tauri/tests/fixtures/ocr/japanese-dialogue.png
git commit -m "feat(player): recognize Japanese game text with Tesseract"
```

### Task 5: Réutiliser TM, glossaire et Ollama pour une ligne runtime

**Files:**
- Create: `src-tauri/src/player/translate.rs`
- Modify: `src-tauri/src/core/tm.rs`
- Modify: `src-tauri/src/llm/provider.rs`
- Test: `src-tauri/src/player/translate.rs`

- [ ] **Step 1: Écrire les tests d'ordre de résolution**

Cas obligatoires : observation déjà traduite → cache; TM exacte → aucun appel LLM; absence TM → appel LLM; réponse vide → source affichée avec `needsReview`; correction manuelle → insertion TM.

- [ ] **Step 2: Implémenter le service**

```rust
pub async fn translate_observation<P: LlmProvider>(
    source: &str,
    profile: &PlayerProfile,
    previous_lines: &[String],
    provider: &P,
    db: &SqlitePool,
) -> Result<RuntimeTranslation, RuntimeTranslationError>
```

Utiliser `lang_pair = format!("{}-{}", source_lang, target_lang)`; rechercher d'abord `runtime_observations`, puis TM exacte; injecter au maximum les 4 lignes précédentes dans le contexte; ne jamais envoyer une image au modèle texte.

- [ ] **Step 3: Limiter la latence**

Pour une seule ligne, forcer `batch_size = 1`, timeout 30 secondes et zéro passe tone/review supplémentaire. La correction humaine reste disponible dans l'historique.

- [ ] **Step 4: Run tests**

Run: `cargo test --manifest-path src-tauri/Cargo.toml player::translate`

Expected: PASS sans Ollama réel grâce au provider simulé.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/player/translate.rs src-tauri/src/core/tm.rs src-tauri/src/llm/provider.rs
git commit -m "feat(player): translate runtime observations through TM and Ollama"
```

### Task 6: Créer les sessions à la demande

**Files:**
- Create: `src-tauri/src/player/session.rs`
- Create: `src-tauri/src/commands/player.rs`
- Modify: `src-tauri/src/commands/mod.rs`
- Modify: `src-tauri/src/state.rs`
- Modify: `src-tauri/src/lib.rs`
- Test: `src-tauri/src/player/session.rs`

- [ ] **Step 1: Étendre AppState**

Ajouter un registre `Arc<tokio::sync::Mutex<HashMap<String, CancellationToken>>>`. Utiliser `tokio-util::sync::CancellationToken`; une session active au maximum par profil.

- [ ] **Step 2: Ajouter les commandes**

Implémenter `list_capture_targets`, `capture_region_once`, `ocr_region_once`, `translate_region_once`, `start_player_session`, `stop_player_session`, `list_runtime_history` et `correct_runtime_translation`.

- [ ] **Step 3: Définir les événements**

Émettre uniquement : `h2s://player/captured`, `h2s://player/recognized`, `h2s://player/translated`, `h2s://player/error`, `h2s://player/stopped`.

- [ ] **Step 4: Tester annulation et déduplication**

Avec une horloge/capture simulée, vérifier qu'un arrêt termine la boucle en moins de 250 ms et que deux frames identiques ne déclenchent qu'un OCR.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/player/session.rs src-tauri/src/commands/player.rs src-tauri/src/commands/mod.rs src-tauri/src/state.rs src-tauri/src/lib.rs src-tauri/Cargo.toml
git commit -m "feat(player): add cancellable live translation sessions"
```

### Task 7: Construire le Player Mode frontend

**Files:**
- Create: `src/stores/player.ts`
- Create: `src/stores/player.test.ts`
- Create: `src/features/player/PlayerHome.tsx`
- Create: `src/features/player/RegionPicker.tsx`
- Create: `src/features/player/LiveTranslationPanel.tsx`
- Create: `src/features/player/PlayerSettings.tsx`
- Create: `src/components/ModeSwitcher.tsx`
- Modify: `src/App.tsx`
- Modify: `src/lib/types.ts`
- Modify: `src/locales/fr.json`
- Modify: `src/locales/en.json`

- [ ] **Step 1: Écrire les tests du store**

Vérifier les transitions `idle → capturing → recognizing → translating → showing`, l'erreur récupérable et l'arrêt qui revient à `idle` sans perdre l'historique.

- [ ] **Step 2: Ajouter les types TypeScript**

Miroiter exactement `PlayerProfile`, `CaptureTarget`, `CaptureRegion`, `OcrResult`, `RuntimeTranslation` et `PlayerSessionStatus` avec les clés camelCase.

- [ ] **Step 3: Ajouter le sélecteur de mode**

Le mode initial reste `editor`; mémoriser `editor | player | image` dans le store Tauri. Changer de mode ne détruit aucun projet CAT ouvert.

- [ ] **Step 4: Implémenter le flux à la demande**

Écran : sélectionner profil → sélectionner fenêtre → définir rectangle → « Capturer et traduire ». Afficher source, traduction, temps OCR, temps LLM et bouton « Corriger ».

- [ ] **Step 5: Tests composants**

Simuler les `invoke()` et vérifier qu'une traduction réussie apparaît, qu'un pack OCR absent propose une instruction et qu'un échec de portail garde le bouton réessayable.

- [ ] **Step 6: Commit**

```bash
git add src/stores/player.ts src/stores/player.test.ts src/features/player src/components/ModeSwitcher.tsx src/App.tsx src/lib/types.ts src/locales/fr.json src/locales/en.json
git commit -m "feat(player): add on-demand game translation interface"
```

### Task 8: Ajouter overlay et traduction automatique contrôlée

**Files:**
- Create: `src-tauri/src/player/overlay.rs`
- Create: `src/features/player/OverlayView.tsx`
- Modify: `src-tauri/src/player/session.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src-tauri/capabilities/default.json`
- Test: `src-tauri/src/player/session.rs`

- [ ] **Step 1: Créer une fenêtre overlay par région**

Utiliser `WebviewWindowBuilder` avec `transparent(true)`, `decorations(false)`, `always_on_top(true)`, `skip_taskbar(true)` et une route `?view=overlay&region=<id>`.

- [ ] **Step 2: Prévoir le fallback Wayland**

Si positionnement, transparence ou click-through n'est pas disponible, afficher la traduction dans `LiveTranslationPanel` sans considérer la session comme échouée.

- [ ] **Step 3: Ajouter les raccourcis globaux**

Avec `tauri-plugin-global-shortcut`, définir par défaut : `Ctrl+Shift+T` traduire, `Ctrl+Shift+A` auto on/off, `Ctrl+Shift+H` masquer/afficher. Les raccourcis sont éditables et détectent les collisions.

- [ ] **Step 4: Ajouter le polling économe**

Valeur par défaut 500 ms; OCR seulement si la distance de hash dépasse le seuil; après texte vide, backoff à 1 seconde; après texte identique, réutiliser la traduction sans LLM.

- [ ] **Step 5: Smoke test KDE Wayland/NVIDIA**

Vérifier jeu fenêtré et borderless sous Proton, changement d'échelle 100/125/150 %, overlay visible, absence de blocage clavier/souris et fallback panneau.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/player/overlay.rs src-tauri/src/player/session.rs src/features/player/OverlayView.tsx src-tauri/src/lib.rs src-tauri/tauri.conf.json src-tauri/capabilities/default.json src-tauri/Cargo.toml
git commit -m "feat(player): add cached live overlay translation"
```

### Task 9: Ajouter PaddleOCR comme provider qualité

**Files:**
- Create: `sidecars/ocr/pyproject.toml`
- Create: `sidecars/ocr/hoshi_ocr/main.py`
- Create: `sidecars/ocr/hoshi_ocr/protocol.py`
- Create: `sidecars/ocr/tests/test_protocol.py`
- Modify: `src-tauri/src/player/ocr.rs`
- Modify: `src-tauri/tauri.conf.json`

- [ ] **Step 1: Définir le protocole JSON Lines**

Entrée : `{"id":"...","imagePath":"...","lang":"japan"}`. Sortie : `{"id":"...","text":"...","confidence":0.94,"boxes":[...]}`. Une ligne invalide retourne un objet erreur et ne tue pas le processus.

- [ ] **Step 2: Créer l'environnement uv**

Verrouiller Python et PaddleOCR; les modèles sont téléchargés dans le dossier app-data au premier usage après confirmation utilisateur.

- [ ] **Step 3: Implémenter `PaddleOcrProvider`**

Lancer un sidecar persistant, associer requêtes/réponses par ID et redémarrer une seule fois après crash. Ne jamais lancer un processus Python par frame.

- [ ] **Step 4: Ajouter le benchmark personnel**

Tester 30 captures réelles anonymisées : menus, dialogues, vertical, police pixel et fond complexe. Conserver Tesseract si sa précision est suffisante; choisir Paddle comme défaut seulement s'il améliore clairement la reconnaissance.

- [ ] **Step 5: Commit**

```bash
git add sidecars/ocr src-tauri/src/player/ocr.rs src-tauri/tauri.conf.json
git commit -m "feat(ocr): add persistent PaddleOCR multilingual sidecar"
```

### Task 10: Consolider les moteurs patchables existants

**Files:**
- Modify: `src-tauri/src/engines/detector.rs`
- Modify: `src-tauri/src/engines/vx_ace/mod.rs`
- Modify: `src-tauri/src/commands/project.rs`
- Create: `src-tauri/tests/fixtures/vx_ace/`
- Modify: `docs/engine-support.md`

- [ ] **Step 1: Réactiver VX Ace uniquement pour données libres**

Réactiver la détection lorsque `Data/System.rvdata2` existe. Si seul un `.rgss3a` est présent, afficher « archive non prise en charge; utiliser le mode OCR » sans extraction automatique.

- [ ] **Step 2: Tester MV/MZ, Wolf et VX Ace**

Pour chaque fixture libre : extraction → traduction d'un segment → export vers un dossier temporaire → réouverture → texte traduit présent. Comparer les fichiers non ciblés bit à bit.

- [ ] **Step 3: Ajouter la matrice de choix**

`MV/MZ loose JSON = patch`, `Wolf compatible/unpacked = patch`, `VX Ace rvdata2 loose = patch`, `archive protégée/inconnue = OCR`.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/engines/detector.rs src-tauri/src/engines/vx_ace src-tauri/src/commands/project.rs src-tauri/tests/fixtures/vx_ace docs/engine-support.md
git commit -m "feat(engine): enable safe VX Ace loose-data translation"
```

### Task 11: Intégrer Unity sans réimplémenter XUnity

**Files:**
- Create: `src-tauri/src/engines/unity/mod.rs`
- Create: `src-tauri/src/engines/unity/probe.rs`
- Create: `src-tauri/src/engines/unity/xunity.rs`
- Modify: `src-tauri/src/engines/mod.rs`
- Modify: `src-tauri/src/engines/detector.rs`
- Create: `src/features/player/UnitySetupWizard.tsx`
- Test: `src-tauri/src/engines/unity/probe.rs`

- [ ] **Step 1: Détecter runtime et plateforme**

Identifier Mono par `<Game>_Data/Managed/Assembly-CSharp.dll`; IL2CPP par `GameAssembly.dll` et `il2cpp_data`; détecter exécution native Linux ou Windows/Proton.

- [ ] **Step 2: Construire un assistant non destructif**

Le wizard explique et vérifie BepInEx/XUnity. Il accepte un dossier d'outils déjà téléchargé par l'utilisateur, calcule les hashes, affiche le plan de copie et crée un backup avant application. Aucun téléchargement silencieux et aucune installation dans un jeu avec anti-cheat détecté/déclaré.

- [ ] **Step 3: Échanger via les caches XUnity**

Lire les nouvelles sources produites par XUnity, traduire par le pipeline Hoshi2Star puis écrire le fichier de traductions attendu dans un fichier temporaire suivi d'un rename atomique. Préserver toutes les lignes/commentaires inconnus.

- [ ] **Step 4: Proton**

Afficher les paramètres BepInEx Proton adaptés, vérifier le log BepInEx au premier lancement et proposer immédiatement le mode OCR si le loader ne démarre pas.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/engines/unity src-tauri/src/engines/mod.rs src-tauri/src/engines/detector.rs src/features/player/UnitySetupWizard.tsx
git commit -m "feat(engine): integrate Unity translation through XUnity"
```

### Task 12: Ajouter Bakin par dictionnaire réversible

**Files:**
- Create: `src-tauri/src/engines/bakin/mod.rs`
- Create: `src-tauri/src/engines/bakin/dictionary.rs`
- Modify: `src-tauri/src/engines/mod.rs`
- Modify: `src-tauri/src/engines/detector.rs`
- Test: `src-tauri/src/engines/bakin/dictionary.rs`

- [ ] **Step 1: Détecter Bakin**

Sonder `data.rbpack`, le player Bakin et les dossiers `data`. La détection ne décompresse rien.

- [ ] **Step 2: Supporter `dic.txt`**

Parser `source<TAB>target` en préservant ordre, lignes inconnues et fins de ligne. Traduire uniquement les valeurs vides ou identiques à la source.

- [ ] **Step 3: Appliquer avec backup**

Utiliser le gestionnaire d'override de Task 14. Si le player doit être remplacé par un outil tiers, le wizard exige le chemin fourni par l'utilisateur, affiche les hashes et permet restauration en un clic.

- [ ] **Step 4: Commit**

```bash
git add src-tauri/src/engines/bakin src-tauri/src/engines/mod.rs src-tauri/src/engines/detector.rs
git commit -m "feat(engine): support reversible Bakin translation dictionaries"
```

### Task 13: Construire le traducteur d'images

**Files:**
- Create: `src-tauri/src/player/image_translate.rs`
- Create: `src-tauri/src/commands/image.rs`
- Create: `src/features/image/ImageTranslator.tsx`
- Create: `sidecars/image/pyproject.toml`
- Create: `sidecars/image/hoshi_image/main.py`
- Test: `sidecars/image/tests/test_render.py`

- [ ] **Step 1: Import et régions**

Ouvrir PNG/JPEG/WebP, détecter les boîtes OCR, permettre ajout/suppression/resize manuel et conserver les coordonnées dans un fichier projet non destructif.

- [ ] **Step 2: Effacer le texte**

Pour fonds unis, utiliser un remplissage couleur médiane. Pour fonds complexes, utiliser `cv2.inpaint` avec masque dilaté. Toujours afficher un comparatif avant/après et permettre retouche du masque.

- [ ] **Step 3: Rendre la traduction**

Utiliser Pillow avec une police choisie par l'utilisateur, auto-fit avec taille minimale, alignement horizontal/vertical et contour/ombre. Ne jamais écraser l'image source.

- [ ] **Step 4: Export**

Exporter `nom.h2s-translated.png` et un manifeste JSON comprenant source hash, régions, OCR, traductions, police et paramètres de rendu.

- [ ] **Step 5: Tests**

Vérifier dimensions identiques, pixels hors masque inchangés, texte cible présent et export reproductible à paramètres identiques.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/player/image_translate.rs src-tauri/src/commands/image.rs src/features/image/ImageTranslator.tsx sidecars/image
git commit -m "feat(image): translate text embedded in game images"
```

### Task 14: Ajouter les remplacements d'assets réversibles

**Files:**
- Create: `src-tauri/src/player/overrides.rs`
- Create: `src/features/image/OverrideManager.tsx`
- Modify: `src-tauri/src/commands/image.rs`
- Test: `src-tauri/src/player/overrides.rs`

- [ ] **Step 1: Définir PatchPlan**

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PatchPlan {
    pub profile_id: String,
    pub operations: Vec<PatchOperation>,
    pub warnings: Vec<String>,
}
```

Chaque opération contient chemin relatif validé, hash original, source remplacement, chemin backup et type `translatedImage | userReplacement`.

- [ ] **Step 2: Dry-run obligatoire**

Refuser chemins absolus, `..`, symlinks sortant du jeu, original absent, hash changé et cible hors racine. Le dry-run ne crée aucun fichier.

- [ ] **Step 3: Application atomique**

Copier l'original dans `<app-data>/overrides/<profile>/<timestamp>/`, écrire le remplacement dans un fichier voisin temporaire, fsync puis rename. Enregistrer le manifeste seulement après succès total; en cas d'erreur, restaurer les opérations déjà appliquées.

- [ ] **Step 4: Restauration**

Vérifier le hash actuel avant restauration. Si le jeu a été mis à jour, ne pas écraser et demander une décision explicite.

- [ ] **Step 5: Limite « uncensor »**

Le logiciel gère uniquement un remplacement fourni par l'utilisateur. Il ne prétend pas reconstruire automatiquement ce qui était caché et ne traite pas les archives protégées.

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/player/overrides.rs src-tauri/src/commands/image.rs src/features/image/OverrideManager.tsx
git commit -m "feat(image): manage reversible user asset overrides"
```

### Task 15: QA, packaging et validation de bout en bout

**Files:**
- Create: `src-tauri/tests/player_pipeline.rs`
- Create: `docs/player-mode.md`
- Create: `docs/engine-support.md`
- Modify: `.github/workflows/release.yml`
- Modify: `README.md`
- Modify: `README.fr.md`
- Modify: `CHANGELOG.md`

- [ ] **Step 1: Matrice automatisée**

Tester Rust sur Linux et Windows; frontend Vitest; migration DB depuis v0.4.10; sidecars Python; fixtures MV/MZ/Wolf/VX Ace; restauration d'override après échec injecté.

- [ ] **Step 2: Matrice manuelle personnelle**

Valider au minimum : un MV/MZ patché, un Wolf patché, un VX Ace loose, un Unity Mono Proton via XUnity, un jeu quelconque OCR Wayland, une image traduite et un remplacement restauré.

- [ ] **Step 3: Mesures d'acceptation**

Capture à la demande < 150 ms hors OCR; OCR + lookup cache < 1 s sur une frame déjà connue; aucun second appel LLM pour texte identique; arrêt session < 250 ms; restauration byte-identical; aucune écriture hors racine du jeu ou app-data.

- [ ] **Step 4: Gate complet**

```bash
pnpm typecheck
pnpm test
pnpm lint
cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
uv run --project sidecars/ocr pytest
uv run --project sidecars/image pytest
```

Expected: toutes les commandes réussissent; les warnings ESLint préexistants sont documentés séparément.

- [ ] **Step 5: Release personnelle**

Bumper `src-tauri/Cargo.toml` et `src-tauri/tauri.conf.json` avec la même version, documenter les modèles OCR optionnels et publier AppImage + Windows MSI. Ne pas embarquer les fichiers de tiers XUnity/BepInEx/Bakin sans licence explicite de redistribution.

- [ ] **Step 6: Commit**

```bash
git add .github/workflows/release.yml docs/player-mode.md docs/engine-support.md README.md README.fr.md CHANGELOG.md src-tauri/tests/player_pipeline.rs
git commit -m "test: validate personal player translation end to end"
```

---

## Ordre de réalisation recommandé

Ne pas commencer par Unity, Bakin ou l'édition d'images. Construire d'abord Tasks 1 à 7 : cela donne un traducteur universel à la demande utilisable rapidement sur la machine actuelle. Continuer avec Task 8 seulement après une session de jeu réelle. Exécuter ensuite Task 10, puis 11 et 12. Terminer par 13–15.

## Self-review

- Le mode CAT existant reste fonctionnel et constitue la meilleure voie pour MV/MZ et Wolf.
- Linux KDE Wayland/NVIDIA et Proton sont explicitement couverts; Windows reste une cible de release.
- OCR, TM, glossaire, Ollama, overlay, historique et correction manuelle sont couverts.
- MV/MZ, MZ, VX Ace, Wolf RPG, Unity et Bakin disposent chacun d'une stratégie concrète et d'un fallback OCR.
- Traduction d'images, sauvegarde, application et restauration des remplacements sont couvertes.
- Aucun contournement DRM, archive protégée, anti-cheat ou génération automatique de contenu caché n'est inclus.
- Les types IPC utilisent camelCase, les commandes restent dans un seul `generate_handler!`, les accès DB restent côté Rust et les événements gardent le préfixe `h2s://`.
