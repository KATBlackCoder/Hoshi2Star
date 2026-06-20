> 🇫🇷 [Lire en français](README.fr.md)

# Hoshi2Star ★

![Version](https://img.shields.io/badge/version-0.4.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows-lightgrey)

**星 → ★ — CAT editor + LLM orchestrator for fan translation of Japanese RPG games**

## Screenshots

### Before & After
| Original (日本語) | Translated (English) |
|:-:|:-:|
| ![Game in Japanese](docs/screenshots/01-game-original-jp.png) | ![Game in English](docs/screenshots/05-game-translated-en.png) |

### Hoshi2Star in action

**1. Open the app**
![Empty state](docs/screenshots/02-hoshi2star-empty.png)

**2. Load a game — segments extracted**
![Segments before translation](docs/screenshots/03-segments-before.png)

**3. Configure Ollama and translate**
![LLM configuration](docs/screenshots/06-llm-config.png)

**4. Segments translated in 27s — QA score 100**
![Segments after translation](docs/screenshots/04-segments-translated.png)

---

## Features

- Open and extract RPG Maker MV/MZ games
- LLM-assisted translation (local Ollama, no API key needed)
- Cross-project Translation Memory (TM) with exact lookup
- Automatic QA: placeholders, line length, UTF-8 BOM
- Export translations back into the game files
- 3-panel CAT interface: Files | Grid | TM + QA
- Project list — recent projects with progress cards, continue or delete with one click
- Per-segment translate button — retranslate a single segment without batch
- Glossary auto-extraction — LLM detects key terms automatically on project open
- TM fuzzy matching — 80% similarity threshold with Levenshtein distance
- Export QA report as standalone HTML

---

## Supported Engines

| Engine | Status | Formats |
|---|---|---|
| RPG Maker MV | ✅ Supported | .json, .rpgmvp |
| RPG Maker MZ | ✅ Supported | .json, .rpgmvp |
| RPG Maker VX Ace | 🔜 F3 | .rvdata2 |
| Wolf RPG | ⚠️ Partial | .dat |
| RPG Developer Bakin | 🔜 F5 | .rbpack |

---

## Prerequisites

- **Ollama** installed: https://ollama.ai
- Recommended model: `ollama pull qwen3:4b-instruct-2507-q8_0`

> The `-instruct` variant responds directly without a thinking
> phase, which produces faster and more reliable translations.
- **Linux**: webkit2gtk-4.1 (usually already installed)
- **Windows**: no additional prerequisites

---

## Cloud LLM with RunPod (optional)

Instead of running Ollama locally, you can use a cloud GPU on [RunPod](https://runpod.io) for faster translation with larger models.

### Setup

1. Create a pod on RunPod (recommended: RTX 4090 or A40)
2. Set these **Environment Variables**:
   ```
   OLLAMA_HOST   = 0.0.0.0
   OLLAMA_MODELS = /workspace/ollama-models
   ```
3. Expose HTTP port `11434`
4. Set this **Start Command** (replace `qwen3:14b` with your preferred model):
   ```bash
   bash -c "mkdir -p /workspace/ollama-models && apt-get update && apt-get install -y zstd && curl -fsSL https://ollama.com/install.sh | sh && ollama serve & sleep 5 && ollama pull qwen3:14b && wait"
   ```
5. Once the pod is running, copy the proxy URL from RunPod → Connect:
   ```
   https://[POD_ID]-11434.proxy.runpod.net
   ```
6. Paste this URL in Hoshi2Star **Settings → Ollama URL** and click **Test**

### Recommended batch sizes

| GPU | VRAM | Batch size |
|---|---|---|
| RTX 4090 | 24 GB | 30–40 |
| A40 | 48 GB | 40–50 |
| A100 | 80 GB | 50–60 |

> **Remember to stop your pod** after translation — RunPod charges per minute. Models stored in `/workspace` persist across restarts (Volume disk), so you only download them once.

---

## Installation

**Linux:**
```bash
chmod +x hoshi2star_*.AppImage
./hoshi2star_*.AppImage
```

**Windows:** download and run the `.msi` from GitHub Releases.

---

## Quick Start

1. Start Ollama: `ollama serve`
2. Open Hoshi2Star
3. Click **"Open Game"** → select the game folder
4. Select a file in the left panel
5. Click **"Translate"** → configure Ollama (URL + model)
6. Start translation
7. Review and edit segments in the grid
8. Click **"Export"** to apply translations to the game

---

## Development

**Prerequisites:** Rust stable (rustup), Node.js LTS + pnpm

**Linux extra:** webkit2gtk-4.1, base-devel

```bash
git clone https://github.com/KATBlackCoder/Hoshi2Star
cd Hoshi2Star
pnpm install
pnpm tauri dev
```

**Tests:**
```bash
cargo test --manifest-path src-tauri/Cargo.toml
pnpm typecheck
```

---

## Tech Stack

| Layer | Technology |
|---|---|
| Desktop runtime | Tauri v2 |
| Backend | Rust, sqlx, tokio |
| Frontend | React 19, TypeScript |
| UI | shadcn/ui, TanStack Table v8 |
| State | Zustand |
| Database | SQLite (embedded) |
| LLM | Ollama (local) |

---

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the full development plan.

---

## License

MIT — see [LICENSE](LICENSE)
