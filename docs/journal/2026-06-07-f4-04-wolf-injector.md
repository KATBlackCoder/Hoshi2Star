# F4-04 — Wolf RPG Injector

**Date:** 2026-06-07  
**Branch:** feat/f4-04-wolf-injector → merged to main  
**Tests:** 247 green (was 232 at session start, +15 new)

---

## What was built

Steps 1–6 of the Wolf RPG injector in one session:

- **`injector.rs`** — `InjectorError`, `WolfTranslation`, `InjectionResult`, `encode_for_wolf`, `inject_map`, `inject_dat`, `serialize_dat`, `inject_all`
- **`dat_parser.rs`** — removed `#[allow(dead_code)]` from `int_values` and `is_utf8` (now consumed by serializer); added `pub(crate)` test helpers `make_minimal_project_pub` / `make_minimal_dat_pub` / `STRING_INDICATOR_PUB`

---

## Key technical decisions

### Step 1 — wolfrpg-map-parser API: NO offsets (Approach B confirmed)

Investigated the public API of `wolfrpg-map-parser 0.6.0` before writing any code:

```
grep -r "pub.*offset\|pub.*position\|pub.*byte" ~/.cargo/registry/src/.../wolfrpg*/src/
```

Result: `offset` variables are local parsing variables only — not exposed in any public struct field. `Map::parse()` returns `Map { events() → pages() → commands() }` with no byte positions.

**Verdict: Approach B (sequential scan + splice).** Approach A (store offsets in `ExtractedSegment`) was never viable without modifying the crate.

### Step 2 — inject_map: sequential scan + splice

Build an ordered list of `(source_encoded, target_encoded)` pairs from the parsed Map (same traversal order as the binary stream), then walk the raw bytes and splice each ReadString whose size+content matches the expected source frame.

Key property: `wolfrpg-map-parser` preserves insertion order, so the replacement list index aligns with the binary stream's occurrence order. No false matches for Japanese text in command bodies.

Identity test: when `src == dst`, `updated_count` stays 0 and bytes are bit-identical.

### Step 4 — serialize_dat: exact mirror of parse_dat_types

`serialize_dat` preserves the indicator byte (`dat_bytes[0]`) and version byte (`dat_bytes[10]`) from the original header. The magic bytes are chosen from `dat.is_utf8`. Result: **bytes identical to input when no translations are applied** (confirmed by `test_inject_dat_identity` and `test_round_trip_dat_identity`).

`unknown1` is written as 0 for all files in scope. Files with `STRING_INDICATOR (0x0001_D4C0)` would need the original `unknown1` value preserved — not present in test fixtures or common Wolf RPG databases.

### Step 5 — inject_all: Option A export

Writes patched files to `Data/MapData/` and `Data/BasicData/`. Creates directories if absent. Never reads or writes `.wolf` archives (confirmed by `test_inject_all_does_not_overwrite_wolf`). Option B (DXA re-pack) deferred to F5.

Key: for `.dat` entries, only the `.dat` is written — `.project` is read as schema but never modified.

---

## Tests added (15 new)

| Test | Module |
|------|--------|
| `test_encode_french_accents_in_v2` | injector |
| `test_encode_french_accents_in_v3` | injector |
| `test_encode_ascii_both_versions` | injector |
| `test_inject_map_identity` | injector |
| `test_inject_map_translation` | injector |
| `test_inject_map_wrong_key` | injector |
| `test_inject_dat_identity` | injector |
| `test_inject_dat_name` | injector |
| `test_inject_dat_wrong_key` | injector |
| `test_round_trip_mps_identity` | injector |
| `test_round_trip_mps_translation` | injector |
| `test_round_trip_dat_identity` | injector |
| `test_round_trip_dat_translation` | injector |
| `test_inject_all_creates_files` | injector |
| `test_inject_all_does_not_overwrite_wolf` | injector |

---

## `#[allow(dead_code)]` removed from F4-03

| Field | Reason |
|-------|--------|
| `DatEntry::int_values` | Consumed by `serialize_dat_type` (writes int values back) |
| `DatFile::is_utf8` | Consumed by `serialize_dat` (selects SJIS vs UTF-8 magic) |
| `DatEntry::name` | Still unused in injector (indexed by position, not name) — **keep** |
| `DatType::name` | Still unused in injector — **keep** |

---

## Verification gate

```
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings  → 0 warnings
cargo test --manifest-path src-tauri/Cargo.toml                   → 247 passed
cargo fmt --manifest-path src-tauri/Cargo.toml --check            → exit 0
```

---

## Limitations documented

- Wolf v2: only Shift-JIS representable characters are supported in translations (`encode_for_wolf` returns `Err(Encoding)` for accented/emoji chars)
- `unknown1 == STRING_INDICATOR` path in `serialize_dat_type` is written defensively but not exercised by any test fixture
- Option B (DXA re-encryption): deferred to F5
- CommonEvents injection: deferred to F4-05 (same as extractor stub)
