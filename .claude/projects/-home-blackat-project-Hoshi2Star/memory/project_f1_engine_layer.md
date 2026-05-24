---
name: F1 Engine Layer MV/MZ state
description: État des modules engines/mv_mz et llm/tokenizer après session F1 — ce qui est fait, ce qui reste
type: project
---

Engine Layer F1 **complet** (session 2026-05-24). 46 tests verts.

Fichiers créés :
- `src-tauri/src/engines/mv_mz/extractor.rs` — JSON Pointer RFC 6901 keys, tous types de fichiers MV/MZ
- `src-tauri/src/engines/mv_mz/injector.rs` — `pointer_mut()` pour round-trip
- `src-tauri/src/engines/mv_mz/decryptor.rs` — XOR 16 bytes, accepte magic RPGMV et RPGMZ
- `src-tauri/src/engines/detector.rs` — détecte `data/` ou `www/data/` + `System.json > gameTitle`
- `src-tauri/src/llm/tokenizer.rs` — tokens `⟦ph_0⟧`, deux variantes MvMz / MzOnly

**Why:** F1 session — engine layer d'abord avant DB et UI.
**How to apply:** Prochaine session F1 = DB (migrations, pool, AppState) puis commands Tauri puis UI React.

Décisions clés :
- Pas de `rvpacker-txt-rs` — implémentation custom JSON Pointer plus propre
- Regex crate : `\\[G\\$.|!><^{}]` et non `\\[G\\\$\.\|\!\>\<\^\{\}]` (escape invalides dans char class)
- tokenizer.rs anticipé depuis F2 car requis dans instructions session
