> 🇫🇷 [Lire en français](README.fr.md)

# Hoshi2Star ★

![Version](https://img.shields.io/badge/version-0.1.0-blue)
![License](https://img.shields.io/badge/license-MIT-green)
![Platform](https://img.shields.io/badge/platform-Linux%20%7C%20Windows-lightgrey)

**星 → ★ — CAT editor + LLM orchestrator for fan translation of Japanese RPG games**

> Screenshot coming soon

---

## Features

- Open and extract RPG Maker MV/MZ games
- LLM-assisted translation (local Ollama, no API key needed)
- Cross-project Translation Memory (TM) with exact lookup
- Automatic QA: placeholders, line length, UTF-8 BOM
- Export translations back into the game files
- 3-panel CAT interface: Files | Grid | TM + QA

---

## Supported Engines

| Engine | Status | Formats |
|---|---|---|
| RPG Maker MV | ✅ Supported | .json, .rpgmvp |
| RPG Maker MZ | ✅ Supported | .json, .rpgmvp |
| RPG Maker VX Ace | 🔜 F3 | .rvdata2 |
| Wolf RPG | 🔜 F4 | .dat, .wolf |
| RPG Developer Bakin | 🔜 F5 | .rbpack |

---

## Prerequisites

- **Ollama** installed: https://ollama.ai
- Recommended model: `ollama pull qwen3:4b-instruct-2507-q4_K_M`

> The `-instruct` variant responds directly without a thinking
> phase, which produces faster and more reliable translations.
- **Linux**: webkit2gtk-4.1 (usually already installed)
- **Windows**: no additional prerequisites

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
