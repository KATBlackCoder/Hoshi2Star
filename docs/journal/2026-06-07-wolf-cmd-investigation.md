# Investigation — Wolf RPG Command 0x09D20000

**Date:** 2026-06-07  
**Status:** Analysis only — no code changes  
**Symptom:** All 24 `.mps` files from 月咲流ホノカver1.03 fail with `panic!("Unknown command code 09d20000")`, caught by `catch_unwind` in `extract_map_segments`.

---

## 1. Command Identification

### Binary format of Wolf RPG command signatures

The 4-byte big-endian signature that `wolfrpg-map-parser` reads has the following structure (confirmed by cross-referencing `signature.rs` with `WolfTL/WolfRPG/Command.hpp`):

```
0xNN_CC_XX_XX
  NN  = total integer argument count + 1  (high byte)
  CC  = Wolf RPG command type (decimal 210 = hex 0xD2 = CommonEvent/CallEvent)
  XX  = secondary type discriminator (0x0000 for most, 0x0100 for some)
```

### Decoding 0x09D20000

| Field | Value | Meaning |
|-------|-------|---------|
| `NN` = `0x09` | 9 | 8 integer arguments (9 − 1) |
| `CC` = `0xD2` | 210 decimal | `CommonEvent` (Call Common Event) |
| `XX` = `0x0000` | 0 | base variant (not by-name) |

**Verdict: `0x09D20000` is a "Call Common Event" command with 8 integer arguments.**

This is confirmed by the WolfTL `CommandType` enum:

```cpp
// WolfTL/WolfRPG/Command.hpp
enum class CommandType {
    CommonEvent        = 210,   // 0xD2
    CommonEventReserve = 211,
    CommonEventByName  = 300,
};
```

---

## 2. wolfrpg-map-parser 0.6.0 Source Analysis

### Signature table (src/command/signature.rs)

All known `D2` (CommonEvent) signatures currently handled:

| Signature | Name | Int args (NN−1) |
|-----------|------|-----------------|
| `0x03D20000` | `CallEventByVariable1` | 2 |
| `0x05D20000` | `CallEvent2` | 4 |
| `0x06D20000` | `CallEvent1` | 5 |
| `0x07D20000` | `CallEvent3` | 6 |
| `0x0BD20000` | `CallEventByVariable2` | 10 |

**`0x09D20000` (8 args) is absent from the table.** It falls through to the wildcard arm:

```rust
// src/command/command.rs line 209–211
_ => |_: &[u8], signature: u32| {
    panic!("Unknown command code {:08x}", signature)
}
```

### Why the panic kills the whole file

`Command::parse_multiple` loops until it sees `Command::Exit()`. Any `panic!` inside — including an unknown signature — propagates upward, unwound only by `catch_unwind` in `extractor.rs`. The entire `.mps` file is then skipped.

### The options.rs secondary panic

`Options::is_arg_string` in `src/command/common_event_command/options.rs` also panics for `arg > 4`. This means even if the signature were added, parsing would require careful handling when `string_arguments > 4`.

---

## 3. WolfTL C++ Reference

Source: `WolfTL/WolfRPG/Command.hpp` (Sinflower/WolfTL, MIT license).

### How WolfTL parses the same bytes

```cpp
// Command::Init() in Command.hpp (lines ~709–750)
uint8_t argsCount = coder.ReadByte() - 1;        // reads NN, subtracts 1
CommandType cid   = static_cast<CommandType>(coder.ReadInt()); // reads cid
// ...reads argsCount int args, then indent, string args, terminator
```

WolfTL reads these as **separate bytes** — it does not combine them into a 4-byte signature. Its `switch(cid)` dispatches only on the command type (`210 = CommonEvent`), not on the argument count. **WolfTL handles 0x09D20000 transparently** because it never constrains which argument count is valid for `CommonEvent`.

### Text content of CommonEvent

WolfTL's `stringsOfCommand()` function:

```cpp
switch (command->GetType()) {
    case CommandType::Message:
    case CommandType::SetString:
    case CommandType::Database:
        strs.push_back(command->Text());
        break;
    case CommandType::Choices:
    case CommandType::StringCondition:
        return command->Texts();
    case CommandType::CommonEventByName:
        for (size_t i = 1; i <= 3; i++)
            strs.push_back(command->Texts().at(i));
        break;
    default:
        break;  // <-- CommonEvent (210) falls here
}
```

**`CommonEvent` (210, command code `0xD2`) returns no strings.** It is a control-flow call — it invokes another common event by ID using integer arguments only. No translatable text is stored in the command itself.

`CommonEventByName` (300, `0x12C`) is different — it can have string arguments (the event name and string parameters). That command uses a different signature family (`0x??2C0100`), not `0xD2`.

---

## 4. GitHub Issues / Crate Versions

### wolfrpg-map-parser GitHub issues

As of 2026-06-07, there are **2 open issues** on the repository:
- `#4` — "Add inline documentation"
- `#2` — "Add tests and a github action workflow"

**No issue has been filed for `0x09D20000`, unknown commands, or map parsing panics.**

### Crate versions (crates.io)

| Version | Date | Notes |
|---------|------|-------|
| **0.6.0** | Nov 1, 2025 | **Latest** — currently used |
| 0.5.5 | Oct 19, 2025 | |
| 0.5.4 | Aug 10, 2025 | |
| 0.5.x | Aug 2025 | Multiple patch releases |

**0.6.0 is the latest release.** There is no newer version that would fix this. The issue has not been reported upstream.

### Recent commits on main

The last 15 commits (up to the 0.6.0 tag) include:
- "Fix common event command string arguments not being parsed correctly"
- "Add Calculation::Nothing"
- "Fix issues in SetVariableCommand"
- "Add common events database parsing"

None address unknown `D2` signatures or the missing `0x09D20000` variant.

---

## 5. Conclusions

### Q1 — What is command 0x09D20000?

**A Call Common Event command with 8 integer arguments.** Wolf RPG command type 210 (`CommonEvent`). The leading byte `0x09` encodes argument count + 1 in the format that `wolfrpg-map-parser` uses as its 4-byte lookup key. The game 月咲流ホノカver1.03 calls common events that pass 8 numeric arguments (e.g., complex conditional calls with multiple variable parameters).

### Q2 — Does it contain translatable text?

**No.** `CommonEvent` (type 210) is pure control flow — it calls another common event by numeric ID, passing integer variables as arguments. All string content lives in the called event, not in the calling command. WolfTL explicitly skips it in `stringsOfCommand()` via `default: break`.

Skipping `0x09D20000` entirely loses **zero translatable text**.

### Q3 — Does a newer crate version support it?

**No.** 0.6.0 is the latest version and it does not handle `0x09D20000`. No fix exists upstream.

### Q4 — Can we safely skip it?

**Yes, completely safely.** It contains no text. The only consequence of skipping it (or treating it as an unknown/no-op) is that control-flow structure in the event page is not modeled. Since Hoshi2Star only extracts text segments — not reconstructing full event logic — this is irrelevant.

### Q5 — Best fix strategy

See section 6.

---

## 6. Recommended Fix Strategy

### Option A — Add `0x09D20000` to the signature table (PREFERRED)

Patch the `Signature` enum in `signature.rs` (or rather, patch the crate usage). Since the crate is a registry dependency (not a fork), the cleanest approach is:

1. **Fork** `wolfrpg-map-parser` or use a **path override** in `Cargo.toml`.
2. Add to `signature.rs`:
   ```rust
   CallEvent4 = 0x09D20000,   // 8-arg CommonEvent variant
   ```
3. Add the match arm in `command.rs`:
   ```rust
   Signature::CallEvent4 => Self::parse_call_common_event,
   ```
4. The existing `parse_call_common_event` → `CommonEventCommand::parse_call_event` → `Event::parse` already handles variable argument counts via `ArgumentCount::new(argument_count)`. **No new parsing logic is needed** — the existing path works.

**Risk:** Zero. The parser already handles CommonEvent correctly for other arg counts. Adding `0x09` (8 args) to the dispatch table is a one-line change.

**Benefit:** The file parses fully, all text commands in the same event page are extracted, and no segments are lost.

### Option B — Patch `catch_unwind` to log and continue per-command (PARTIAL FIX)

Instead of aborting the entire file on any unknown command, parse page-by-page and skip only the event page or individual command that panics. This recovers text from the other 23 files (or pages before the unknown command).

This is viable but more invasive to the extractor logic. It also leaves the root cause (missing signature) unresolved.

### Option C — Upstream PR

File a PR to `G1org1owo/wolfrpg-map-parser` adding `0x09D20000` (and potentially other missing variants like `0x04D20000`, `0x08D20000` if they exist in other games). This is the right long-term action but won't unblock the current game immediately.

### Recommendation

**Do Option A + C together:**
1. Fork the crate (or use a `[patch.crates-io]` path dependency to a local copy) with the one-line fix.
2. Open a PR upstream so the fix lands in 0.6.1.
3. Once upstream releases, remove the fork/patch.

The fix is so small (one enum variant + one match arm in `signature.rs`) that the risk is negligible and the benefit — all 24 `.mps` files parse correctly — is immediate.

---

## Appendix — Signature Encoding Reference

The wolfrpg-map-parser 4-byte big-endian signature encodes two fields:

```
Byte 0  = N+1  where N = number of integer arguments to the command
Byte 1  = Wolf RPG CommandType (u8 low byte of the u32 enum value)
Byte 2-3 = 0x0000 (standard) or 0x0100 (extended, e.g. party/effect commands)
```

Full `0xD2` (CommonEvent) family:

```
0x03D20000  CallEventByVariable1  (2 int args)
0x05D20000  CallEvent2            (4 int args)
0x06D20000  CallEvent1            (5 int args)
0x07D20000  CallEvent3            (6 int args)
0x09D20000  [MISSING]             (8 int args)  ← the panic source
0x0BD20000  CallEventByVariable2  (10 int args)
```

The gap at `0x09` (8 args) is simply an oversight in the parser — there is no structural reason this variant cannot be parsed identically to the others.
