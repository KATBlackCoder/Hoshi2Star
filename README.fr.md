<div align="center">

<img src="src-tauri/icons/128x128@2x.png" width="104" alt="Logo Hoshi2Star" />

# Hoshi2Star&nbsp;★

**星 → ★**

Éditeur CAT + orchestrateur LLM pour la traduction fan de jeux **RPG Maker** & **Wolf RPG** japonais

[![Version](https://img.shields.io/badge/version-0.4.3-6d5dfc)](CHANGELOG.md)
[![Licence](https://img.shields.io/badge/licence-MIT-3fb950)](LICENSE)
[![Plateformes](https://img.shields.io/badge/plateformes-Linux%20%7C%20Windows-8b949e)](https://github.com/KATBlackCoder/Hoshi2Star/releases)
[![Construit avec Tauri](https://img.shields.io/badge/construit%20avec-Tauri%20v2-24c8db)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-stable-ce422b)](https://www.rust-lang.org)

[🇬🇧 English](README.md)&nbsp;·&nbsp;🇫🇷 Français

</div>

---

<div align="center">

| Original&nbsp;(日本語) | Traduit&nbsp;(English) |
|:-:|:-:|
| <img src="docs/screenshots/01-game-original-jp.png" width="380" alt="Jeu en japonais" /> | <img src="docs/screenshots/05-game-translated-en.png" width="380" alt="Jeu en anglais" /> |

</div>

> **Hoshi2Star** extrait le texte d'un jeu RPG japonais, le traduit avec un LLM local (ou cloud) dans un vrai workflow CAT — mémoire de traduction, glossaire, QA — puis réinjecte les traductions dans le jeu.

**Aller à :** [Fonctionnalités](#-fonctionnalités) · [Moteurs](#-moteurs-supportés) · [Installation](#-installation) · [Démarrage](#-démarrage-rapide) · [Développement](#-développement)

---

## ✨ Fonctionnalités

**Traduire**
- 🤖 Traduction assistée par LLM via **Ollama local** — aucune clé API, fonctionne hors ligne
- ☁️ **GPU cloud** optionnel via RunPod (endpoint HTTPS)
- 🎯 Par batch, par segment, ou projet entier avec **« Tout traduire »** et pauses travail/repos automatiques
- ♻️ Retry adaptatif + découpe du batch en cas d'échec, préservation des placeholders (`\V[n]`, `\C[n]`, codes Wolf)

**Qualité**
- 🧪 **QA** automatique : placeholders manquants, longueur de ligne (en pixels), BOM UTF-8, écart glossaire
- 📄 Export d'un **rapport QA** en HTML autonome
- 📖 **Glossaire** — deux niveaux (global + projet), auto-extrait par le LLM à l'ouverture du projet
- 🧠 **Translation Memory** (cross-projets) : correspondance exacte + fuzzy (Levenshtein 80 %), export TMX

**Workflow**
- 🗂 Interface CAT 3 panneaux : **Fichiers · Grille · TM + QA**
- 📇 Liste de projets avec cartes de progression — continuer ou supprimer en un clic
- 📦 Réinjection des traductions dans les fichiers du jeu (packagées en ZIP)

---

## 🎮 Moteurs supportés

| Moteur | Statut | Formats |
|---|---|---|
| RPG Maker MV / MZ | ✅ Supporté | `.json`, `.rpgmvp` / `.rpgmvo` |
| Wolf RPG v1 / v2 / v3 | ⚠️ Partiel | `.dat`, `.mps`, `.wolf` |
| RPG Maker VX Ace | ⏸ Code prêt, désactivé | `.rvdata2` |
| RPG Developer Bakin | 🔜 Prévu (F5) | `.rbpack` |

> [!NOTE]
> Les archives Wolf RPG **v3.5+ / WolfX (Pro, chiffrées)** doivent d'abord être déchiffrées avec UberWolf, puis on ouvre le dossier `Data/` en clair.

---

## 🖼 Captures d'écran

<details>
<summary><b>Hoshi2Star en action</b> — cliquer pour déplier</summary>

<br />

**1. Ouvrir l'application**

![État initial](docs/screenshots/02-hoshi2star-empty.png)

**2. Charger un jeu — segments extraits**

![Segments avant traduction](docs/screenshots/03-segments-before.png)

**3. Configurer Ollama et traduire**

![Configuration LLM](docs/screenshots/06-llm-config.png)

**4. Segments traduits en 27s — score QA 100**

![Segments après traduction](docs/screenshots/04-segments-translated.png)

</details>

---

## 📦 Installation

Téléchargez le dernier build depuis les [**Releases GitHub**](https://github.com/KATBlackCoder/Hoshi2Star/releases).

**Linux** (AppImage) :
```bash
chmod +x hoshi2star_*.AppImage
./hoshi2star_*.AppImage
```
Des paquets `.deb` et `.rpm` sont aussi fournis.

**Windows :** téléchargez et exécutez le `.msi` (ou le `-setup.exe`).

### Prérequis

- **[Ollama](https://ollama.ai)** installé en local
- Modèle recommandé :
  ```bash
  ollama pull qwen3:4b-instruct-2507-q8_0
  ```
- **Linux :** `webkit2gtk-4.1` (généralement déjà installé) · **Windows :** aucun

> [!TIP]
> La variante `-instruct` répond directement, sans phase de « raisonnement » — traductions plus rapides et plus fiables.

---

## 🚀 Démarrage rapide

1. Démarrer Ollama : `ollama serve`
2. Ouvrir Hoshi2Star et cliquer sur **« Ouvrir un jeu »** → sélectionner le dossier du jeu
3. Choisir un fichier dans le panneau gauche
4. Cliquer sur **« Traduire »** → configurer Ollama (URL + modèle)
5. Réviser et modifier les segments dans la grille
6. Cliquer sur **« Exporter »** pour réinjecter les traductions dans le jeu

> [!NOTE]
> Vous préférez un GPU cloud ? Pointez Hoshi2Star vers un endpoint HTTPS [RunPod](https://runpod.io) au lieu d'Ollama local — voir le **[guide RunPod](docs/runpod.fr.md)**.

---

## 🛠 Développement

**Prérequis :** Rust stable (rustup), Node.js LTS + pnpm · **Linux en plus :** `webkit2gtk-4.1`, `base-devel`

```bash
git clone https://github.com/KATBlackCoder/Hoshi2Star
cd Hoshi2Star
pnpm install
pnpm tauri dev
```

**Vérifications :**
```bash
pnpm typecheck && pnpm test
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

---

## 🧱 Stack technique

| Couche | Technologie |
|---|---|
| Runtime desktop | Tauri v2 |
| Backend | Rust · sqlx · tokio |
| Frontend | React 19 · TypeScript |
| UI | shadcn/ui · TanStack Table v8 |
| État global | Zustand |
| Base de données | SQLite (embarquée) |
| LLM | Ollama (local) · RunPod (cloud) |
| Tests | Vitest · `cargo test` |

---

## 🗺 Feuille de route

Voir [ROADMAP.md](ROADMAP.md) pour le plan de développement complet et le [CHANGELOG](CHANGELOG.md) pour l'historique des versions.

---

## 📄 Licence

MIT — voir [LICENSE](LICENSE).
