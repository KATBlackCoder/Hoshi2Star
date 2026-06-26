> 🇫🇷 [Lire en français](README.fr.md)

# Hoshi2Star ★

![Version](https://img.shields.io/badge/version-0.4.2-blue)
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

You can use a cloud GPU on [RunPod](https://runpod.io) instead of running Ollama locally.

→ **[Full RunPod setup guide](docs/runpod.md)**

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
