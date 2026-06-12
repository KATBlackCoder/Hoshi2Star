# Lessons Learned

Format: `[YYYY-MM-DD] Mistake → Root cause → Rule to follow`

---

<!-- Append new lessons below this line -->

[2026-06-11] Wolf RPG v3.x (Inko) `.mps`/`CommonEvent.dat` support via fork-surgery on `wolfrpg-map-parser` was abandoned mid-implementation → Root cause: v3.x's LZ4-wrapped header + WolfTL's flat, non-recursive `Command::Init` model (with a v3.5+ `v35Unknown` trailer per command) diverges too far from `wolfrpg-map-parser`'s recursive v2.x-shaped container model — patching it on top would require invasive upstream changes for a format the crate was never designed for → Rule: v3.x (Inko) parsing/writing lives in an in-house module (`src-tauri/src/engines/wolf/v3_format/`) ported byte-for-byte from WolfTL's flat model (MIT, github.com/Sinflower/WolfTL), validated via byte-exact `parse(bytes).dump() == bytes` round-trip on real files before wiring into extractor/injector. v2.x (Honoka) continues to use `wolfrpg_map_parser` — do not try to unify the two formats under one parser.

[2026-06-11] `llm/pipeline.rs::translate_batch` tokenized every project with `TokEngine::MvMz` regardless of `project.engine` → Root cause: the pipeline never threaded the project's engine through, so Wolf-only codes (`\E`, `\cself[n]`, `\r[Base,Ruby]`, `<L>/<C>/<R>`, etc. — verified ~28 of ~41 Wolf codes are NOT in `RE_MVMZ`) were sent raw to the LLM with no preservation instruction and silently dropped; `Tokenizer::validate`/`restore` couldn't catch it because the placeholder map was empty from the start → Rule: always set `TranslationContext.engine` from `projects.engine` and call `Tokenizer::tokenize(text, Engine::from_project_engine(&context.engine))` — this is the single source of truth, also reused by `core/qa.rs::check`.
