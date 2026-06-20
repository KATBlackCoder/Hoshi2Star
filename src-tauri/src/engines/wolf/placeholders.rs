//! Wolf RPG placeholder pattern — used by `llm::tokenizer::Tokenizer` for `Engine::Wolf`.

use regex::Regex;
use std::sync::LazyLock;

/// Wolf RPG placeholder patterns — ordered by specificity (longest prefix first).
///
/// Priority order (must not be changed):
///   1. `\r[Base,Ruby]`       — ruby annotation (opaque, contains comma)
///   2. DB refs `\udb/cdb/sdb[\d+:\d+:\d+]` — before simple codes
///   3. `\sysS[n]`            — before `\sys[n]` (longer prefix)
///   4. `\cself[n]`           — before `\self[n]` (longer prefix)
///   5. `\self[n]`, `\sys[n]` — event/system variables
///   6. `\space[n]`           — before `\sp[n]` (longer prefix)
///   7. `\v?[n]`              — reserve variable (? is literal in Wolf)
///   8. `\sp/mx/my/ax/ay[n]`, `\-[n]`, `\font[n]` — multi-char codes
///   9. `\v/c/s/f/i[n]` (maj+min) — standard codes
///  10. `\m[n]`               — max line (m excluded from group 9 to avoid duplication with \f)
///  11. `<L>/<C>/<R>`         — alignment tags
///  12. `\A+`, `\A-`          — anti-aliasing
///  13. `\E \N \\ \! \. \^ \> \<` — no-arg display control codes
///  14. `^@\d+\n`             — Wolf v3 speaker sprite index prefix (leading only)
///  15. `\n`                  — literal newline
pub(crate) static RE_WOLF: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
          \\r\[[^\[\],]+,[^\[\]]*\]           # \r[Base,Ruby] — ruby opaque (PREMIER)
        | \\(?:udb|cdb|sdb)\[\d+:\d+:\d+\]   # DB refs 3 params (avant codes simples)
        | \\sysS\[\d+\]                        # system string (avant \sys)
        | \\cself\[\d{1,2}\]                   # common-event self (avant \self)
        | \\self\[\d\]                         # self variable événement
        | \\sys\[\d+\]                         # system variable
        | \\space\[\d+\]                       # line height (avant \sp)
        | \\v\?\[\d+\]                         # reserve variable \v?[n]
        | \\(?:sp|mx|my|ax|ay)\[\d+\]          # speed/offset codes
        | \\-\[\d+\]                           # pixel spacing
        | \\font\[\d\]                         # sub-font
        | \\[vcsfiVCSFI]\[\d+\]                # standard codes v/c/s/f/i (maj+min)
        | \\m\[\d+\]                           # max line (\m[n])
        | <[LCR]>                              # alignment tags
        | \\A[+\-]                             # anti-aliasing on/off
        | \\[EN\\!.^><]                        # no-arg codes: \E \N \\ \! \. \^ \> \<
        | ^@\d+\n                              # Wolf v3 speaker prefix (@N\n) — leading only
        | \n                                   # literal newline
        ",
    )
    .expect("RE_WOLF regex must compile")
});

#[cfg(test)]
mod tests {
    use crate::llm::tokenizer::{Engine, Tokenizer};

    #[test]
    fn test_wolf_ruby_opaque() {
        // \r[Base,Ruby] must be tokenized as a single opaque token
        let result = Tokenizer::tokenize(r"\r[魔法,まほう]", Engine::Wolf);
        assert_eq!(result.map.len(), 1);
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), r"\r[魔法,まほう]");
        let restored = Tokenizer::restore(&result.text, &result.map).unwrap();
        assert_eq!(restored, r"\r[魔法,まほう]");
    }

    #[test]
    fn test_wolf_ruby_with_text() {
        // Text after ruby must be preserved
        let original = r"\r[魔法,まほう]が使えます";
        let result = Tokenizer::tokenize(original, Engine::Wolf);
        assert_eq!(result.map.len(), 1);
        assert!(result.text.contains("が使えます"));
        let restored = Tokenizer::restore(&result.text, &result.map).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn test_wolf_db_refs() {
        // \udb, \cdb, \sdb — 3-param DB references → 3 separate tokens
        let original = r"\udb[0:1:2] \cdb[3:4:5] \sdb[6:7:8]";
        let result = Tokenizer::tokenize(original, Engine::Wolf);
        assert_eq!(result.map.len(), 3);
        let restored = Tokenizer::restore(&result.text, &result.map).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn test_wolf_sysS_before_sys() {
        // \sysS[10] must yield exactly 1 token — not \sys[10] + "S" residual
        let result = Tokenizer::tokenize(r"\sysS[10]", Engine::Wolf);
        assert_eq!(result.map.len(), 1);
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), r"\sysS[10]");
    }

    #[test]
    fn test_wolf_cself_before_self() {
        // \cself[99] must yield exactly 1 token — not \self[9] + "9" residual
        let result = Tokenizer::tokenize(r"\cself[99]", Engine::Wolf);
        assert_eq!(result.map.len(), 1);
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), r"\cself[99]");
    }

    #[test]
    fn test_wolf_reserve_variable() {
        // \v?[30] — the ? is a literal character in Wolf RPG (not a regex quantifier)
        let result = Tokenizer::tokenize(r"\v?[30]", Engine::Wolf);
        assert_eq!(result.map.len(), 1);
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), r"\v?[30]");
        let restored = Tokenizer::restore(&result.text, &result.map).unwrap();
        assert_eq!(restored, r"\v?[30]");
    }

    #[test]
    fn test_wolf_alignment_tags() {
        // <L> and <R> are alignment placeholders; Japanese text between them is preserved
        let original = r"<L>テキスト<R>";
        let result = Tokenizer::tokenize(original, Engine::Wolf);
        assert_eq!(result.map.len(), 2);
        assert!(result.text.contains("テキスト"));
        let restored = Tokenizer::restore(&result.text, &result.map).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn test_wolf_standard_codes() {
        // \v[5], \c[3], \s[10] — identical syntax to MV/MZ standard codes
        let original = r"\v[5]\c[3]\s[10]";
        let result = Tokenizer::tokenize(original, Engine::Wolf);
        assert_eq!(result.map.len(), 3);
        let restored = Tokenizer::restore(&result.text, &result.map).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn test_wolf_no_arg_codes() {
        // \E, \N, \A+, \A- — no-argument display control codes
        let original = r"\E\N\A+\A-";
        let result = Tokenizer::tokenize(original, Engine::Wolf);
        assert_eq!(result.map.len(), 4);
        let restored = Tokenizer::restore(&result.text, &result.map).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn test_wolf_multiline() {
        // Literal newline between Wolf dialogue lines → 1 token, round-trip OK
        let original = "テキスト\nつぎの行";
        let result = Tokenizer::tokenize(original, Engine::Wolf);
        assert_eq!(result.map.len(), 1);
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), "\n");
        let restored = Tokenizer::restore(&result.text, &result.map).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn test_wolf_no_interference_with_mvmz() {
        // \V[12] tokenized via Engine::MvMz — must be unaffected by RE_WOLF addition
        let result = Tokenizer::tokenize(r"\V[12]", Engine::MvMz);
        assert_eq!(result.map.len(), 1);
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), r"\V[12]");
    }

    #[test]
    fn test_wolf_v3_speaker_prefix_round_trip() {
        // @2\n is the Wolf v3 speaker sprite index — leading @N\n becomes one token
        let original = "@2\n「ねえ、今日はどこ行く？」";
        let result = Tokenizer::tokenize(original, Engine::Wolf);
        assert_eq!(
            result.map.len(),
            1,
            "only the speaker prefix must be tokenized"
        );
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), "@2\n");
        assert!(result.text.contains("「ねえ、今日はどこ行く？」"));
        let restored = Tokenizer::restore(&result.text, &result.map).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn test_wolf_v3_speaker_prefix_two_digit() {
        // @10\n — two-digit index must also match
        let original = "@10\nテキスト";
        let result = Tokenizer::tokenize(original, Engine::Wolf);
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), "@10\n");
        let restored = Tokenizer::restore(&result.text, &result.map).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn test_wolf_v3_speaker_prefix_not_mid_text() {
        // @N\n mid-text must NOT be tokenized as a speaker prefix (anchor is leading-only)
        let original = "テスト@2\nつぎ";
        let result = Tokenizer::tokenize(original, Engine::Wolf);
        // The \n inside @2\n is still tokenized as a bare newline
        assert!(
            result.map.values().any(|v| v == "\n"),
            "bare \\n must still be a token"
        );
        // But @2 before the \n is not a speaker token
        assert!(
            !result.map.values().any(|v| v.starts_with('@')),
            "@2 mid-text must not be a speaker token"
        );
    }
}
