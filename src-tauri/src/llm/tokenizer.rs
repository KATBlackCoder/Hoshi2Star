//! Placeholder tokenizer — protects RPG Maker escape codes from LLM corruption.
//!
//! Reference: `docs/engines/mv-mz-placeholders.md`
//!
//! ## Workflow
//! ```text
//! Source : "Vous avez \V[12] pièces et \N[1] vous attend."
//!   ① tokenize  → "Vous avez ⟦ph_0⟧ pièces et ⟦ph_1⟧ vous attend."
//!   ② LLM       → "You have ⟦ph_0⟧ coins and ⟦ph_1⟧ is waiting."
//!   ③ validate  → all tokens present, none duplicated ✓
//!   ④ restore   → "You have \V[12] coins and \N[1] is waiting."
//! ```
//!
//! If a token is missing or duplicated after the LLM response, `restore`/`validate`
//! return an error and the segment must be retried.

use regex::Regex;
use std::collections::HashMap;
use std::sync::LazyLock;

// ---------------------------------------------------------------------------
// Regex patterns (compiled once at first use)
// ---------------------------------------------------------------------------

/// MV/MZ combined: Groupe A + Groupe B + Groupe D (MV `[%n]` form) + Groupe E (plugins).
/// NOTE: `\PX/\PY/\FS` (Groupe C) are NOT included — they are MZ-only.
static RE_MVMZ: LazyLock<Regex> = LazyLock::new(|| {
    // Inside a character class [..], only \\ needs escaping — other chars are literal.
    // Characters matched after the leading backslash: G \ $ . | ! > < ^ { }
    // Groupe E MUST come before Groupe A to avoid partial matches on \+word.
    // \n (literal newline U+000A) is tokenized last — preserves structural line breaks
    // in multi-line fields (description, profile) so the segment stays on one line.
    Regex::new(
        r"(?x)
          \\[+\-]\w+\[\d+\]        # Groupe E — plugin codes (\+switch[n], \-var[n], …)
        | \\[VNPCIvnpci]\[\d+\]   # Groupe A — codes avec argument numérique (maj + min)
        | \\[G\\$.|!><^{}]        # Groupe B — codes sans argument
        | \[%\d+\]                # Groupe D (MV) — [%1] [%2] …
        | \n                      # structural line break (description/profile fields)
        ",
    )
    .expect("RE_MVMZ regex must compile")
});

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
///  14. `\n`                  — literal newline
static RE_WOLF: LazyLock<Regex> = LazyLock::new(|| {
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
        | \n                                   # literal newline
        ",
    )
    .expect("RE_WOLF regex must compile")
});

/// MZ-only: Groupe C (before A!) + Groupe A + Groupe B + Groupe D (MZ bare `%n` form) + Groupe E.
/// Groupe C MUST come before Groupe A to prevent `\P` from consuming `\PX`/`\PY`/`\FS`.
static RE_MZONLY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?x)
          \\[+\-]\w+\[\d+\]                # Groupe E — plugin codes (\+switch[n], …)
        | \\(?:PX|PY|FS|px|py|fs)\[\d+\]  # Groupe C — MZ position/font codes (avant A!)
        | \\[VNPCIvnpci]\[\d+\]            # Groupe A — codes avec argument numérique (maj + min)
        | \\[G\\$.|!><^{}]                 # Groupe B — codes sans argument
        | %\d+                             # Groupe D (MZ) — %1 %2 … (sans crochets)
        | \n                               # structural line break (description/profile fields)
        ",
    )
    .expect("RE_MZONLY regex must compile")
});

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Which placeholder variant to apply.
///
/// - `MvMz`   — Patterns A + B + D(`[%n]`). Use for MV games and MZ dialogue.
/// - `MzOnly` — Patterns C + A + B + D(`%n`). Use for MZ `System.json > terms` only.
/// - `Wolf`   — Wolf RPG specific patterns (ruby, DB refs, sys vars, alignment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Engine {
    /// RPG Maker MV *or* MZ dialogue files — patterns A, B, D[%n]
    MvMz,
    /// MZ `System.json > terms` — patterns C, A, B, D[%n bare]
    MzOnly,
    /// Wolf RPG — ruby, DB refs, sys/self vars, alignment, display control
    Wolf,
}

/// Map from token (`⟦ph_0⟧`) to original placeholder (`\V[12]`).
pub type PlaceholderMap = HashMap<String, String>;

/// Result of tokenizing a text segment.
#[derive(Debug)]
pub struct Tokenized {
    /// Text with placeholders replaced by opaque tokens.
    pub text: String,
    /// Map of token → original placeholder for later restoration.
    pub map: PlaceholderMap,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TokenizerError {
    #[error("LLM response is missing placeholder token '{uuid}' (original: '{original}')")]
    MissingPlaceholder { uuid: String, original: String },
    #[error("LLM response contains duplicate placeholder token '{uuid}'")]
    DuplicatePlaceholder { uuid: String },
}

// ---------------------------------------------------------------------------
// Tokenizer
// ---------------------------------------------------------------------------

pub struct Tokenizer;

impl Tokenizer {
    /// Replace all placeholders in `text` with opaque tokens `⟦ph_0⟧`, `⟦ph_1⟧`, …
    ///
    /// Returns a `Tokenized` value containing the modified text and the
    /// token→original map needed for `restore`.
    pub fn tokenize(text: &str, engine: Engine) -> Tokenized {
        let re = match engine {
            Engine::MvMz => &*RE_MVMZ,
            Engine::MzOnly => &*RE_MZONLY,
            Engine::Wolf => &*RE_WOLF,
        };

        let mut map = PlaceholderMap::new();
        let mut counter = 0usize;

        let tokenized_text = re
            .replace_all(text, |caps: &regex::Captures| {
                let original = caps[0].to_string();
                let token = format!("⟦ph_{counter}⟧");
                counter += 1;
                map.insert(token.clone(), original);
                token
            })
            .into_owned();

        Tokenized {
            text: tokenized_text,
            map,
        }
    }

    /// Validate that a LLM response contains all tokens exactly once.
    ///
    /// Returns `Err(MissingPlaceholder)` if any token is absent.
    /// Returns `Err(DuplicatePlaceholder)` if any token appears more than once.
    pub fn validate(response: &str, map: &PlaceholderMap) -> Result<(), TokenizerError> {
        // Sort keys for deterministic error reporting in tests
        let mut tokens: Vec<&String> = map.keys().collect();
        tokens.sort();
        for token in tokens {
            let count = response.matches(token.as_str()).count();
            if count == 0 {
                return Err(TokenizerError::MissingPlaceholder {
                    uuid: token.clone(),
                    original: map[token].clone(),
                });
            }
            if count > 1 {
                return Err(TokenizerError::DuplicatePlaceholder {
                    uuid: token.clone(),
                });
            }
        }
        Ok(())
    }

    /// Restore the original placeholders in a tokenized LLM response.
    ///
    /// Calls `validate` first — returns an error if any token is missing or duplicated.
    pub fn restore(tokenized: &str, map: &PlaceholderMap) -> Result<String, TokenizerError> {
        Self::validate(tokenized, map)?;
        let mut result = tokenized.to_string();
        // Sort by token name so replacements are deterministic and non-overlapping
        let mut entries: Vec<(&String, &String)> = map.iter().collect();
        entries.sort_by_key(|(k, _)| k.as_str());
        for (token, original) in entries {
            result = result.replace(token.as_str(), original.as_str());
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------------------
// Tests (all 10 from docs/engines/mv-mz-placeholders.md)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // 1. Tokenisation basique \V[n]
    #[test]
    fn test_tokenize_variable() {
        let result = Tokenizer::tokenize(r"\V[12]", Engine::MvMz);
        assert_eq!(result.text, "⟦ph_0⟧");
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), r"\V[12]");
    }

    // 2. Tokenisation \PX[n] (MZ) sans capturer \P[n]
    #[test]
    fn test_tokenize_mz_px_not_confused_with_p() {
        // \PX[100] must be tokenized as a unit, \P[2] as a separate unit
        let result = Tokenizer::tokenize(r"\PX[100] et \P[2]", Engine::MzOnly);
        assert_eq!(result.map.len(), 2);
        // token 0 = \PX[100], token 1 = \P[2]
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), r"\PX[100]");
        assert_eq!(result.map.get("⟦ph_1⟧").unwrap(), r"\P[2]");
        assert!(result.text.contains("⟦ph_0⟧"));
        assert!(result.text.contains("⟦ph_1⟧"));
        // Neither raw placeholder should remain in the text
        assert!(!result.text.contains(r"\PX"));
        assert!(!result.text.contains(r"\P["));
    }

    // 3. Round-trip : tokenize → restore = texte original
    #[test]
    fn test_round_trip() {
        let original = r"Vous avez \V[12] pièces et \N[1] vous attend.";
        let tokenized = Tokenizer::tokenize(original, Engine::MvMz);
        let restored = Tokenizer::restore(&tokenized.text, &tokenized.map).unwrap();
        assert_eq!(restored, original);
    }

    // 4. Rejet si UUID manquant en sortie LLM
    #[test]
    fn test_reject_missing_placeholder() {
        let original = r"Vous avez \V[12] pièces.";
        let tokenized = Tokenizer::tokenize(original, Engine::MvMz);
        // LLM dropped the placeholder entirely
        let bad_response = "You have some coins.";
        let result = Tokenizer::validate(bad_response, &tokenized.map);
        assert!(matches!(
            result,
            Err(TokenizerError::MissingPlaceholder { .. })
        ));
    }

    // 5. Rejet si UUID dupliqué
    #[test]
    fn test_reject_duplicate_placeholder() {
        let original = r"\V[12]";
        let tokenized = Tokenizer::tokenize(original, Engine::MvMz);
        // LLM emitted the token twice
        let bad_response = "⟦ph_0⟧ et aussi ⟦ph_0⟧";
        let result = Tokenizer::validate(bad_response, &tokenized.map);
        assert!(matches!(
            result,
            Err(TokenizerError::DuplicatePlaceholder { .. })
        ));
    }

    // 6. Texte sans placeholder → inchangé
    #[test]
    fn test_text_without_placeholders() {
        let text = "Bonjour tout le monde !";
        let result = Tokenizer::tokenize(text, Engine::MvMz);
        assert_eq!(result.text, text);
        assert!(result.map.is_empty());
    }

    // 7. Placeholders multiples dans un seul segment
    #[test]
    fn test_multiple_placeholders() {
        let text = r"\C[3]\N[1] a reçu \V[5] dégâts !";
        let result = Tokenizer::tokenize(text, Engine::MvMz);
        assert_eq!(result.map.len(), 3);
        // No raw backslash-escape codes should remain
        assert!(!result.text.contains('\\'));
        // Round-trip must be lossless
        let restored = Tokenizer::restore(&result.text, &result.map).unwrap();
        assert_eq!(restored, text);
    }

    // 8. %1 dans Terms MZ vs 100% dans texte libre
    #[test]
    fn test_percent_substitution_mz_only() {
        // MzOnly mode: bare %1 and %2 ARE tokenized
        let term_text = r"%1は%2のダメージを受けた！";
        let result_mz = Tokenizer::tokenize(term_text, Engine::MzOnly);
        assert_eq!(result_mz.map.len(), 2, "MzOnly must tokenize %1 and %2");

        // MvMz mode: bare %1 is NOT tokenized (only [%1] form is)
        let result_mv = Tokenizer::tokenize(term_text, Engine::MvMz);
        assert_eq!(result_mv.map.len(), 0, "MvMz must NOT tokenize bare %1");

        // "100%の威力！" — % is followed by non-digit, must NOT be tokenized
        let free_text = "効果は100%だ！";
        let result_free = Tokenizer::tokenize(free_text, Engine::MzOnly);
        assert_eq!(
            result_free.map.len(),
            0,
            "100% (percent followed by non-digit) must not be tokenized"
        );
    }

    // 9. \\ (double backslash) préservé correctement
    #[test]
    fn test_double_backslash() {
        // In MV/MZ text (after JSON parsing), "\\" = 2 chars: backslash + backslash.
        // Groupe B matches: first \ followed by second \ (which is in the char class).
        let text = "\\\\"; // Rust string: 2 actual backslash characters
        let result = Tokenizer::tokenize(text, Engine::MvMz);
        assert_eq!(
            result.map.len(),
            1,
            "double backslash should produce exactly one token"
        );
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), "\\\\");

        // Round-trip must restore the original 2-backslash string
        let restored = Tokenizer::restore(&result.text, &result.map).unwrap();
        assert_eq!(restored, text);
    }

    // 10. Segment vide → pas d'erreur
    #[test]
    fn test_empty_string() {
        let result = Tokenizer::tokenize("", Engine::MvMz);
        assert_eq!(result.text, "");
        assert!(result.map.is_empty());

        // restore on empty tokenized string with empty map → OK
        let restored = Tokenizer::restore("", &result.map).unwrap();
        assert_eq!(restored, "");
    }

    // 11b. Groupe E — plugin codes \+switch[n], \-var[n]
    #[test]
    fn test_plugin_codes_tokenized() {
        let result = Tokenizer::tokenize(r"\+switch[269]", Engine::MvMz);
        assert_eq!(result.text, "⟦ph_0⟧");
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), r"\+switch[269]");

        // Round-trip
        let original = r"Go to \+switch[270] and talk to \N[1].";
        let tok = Tokenizer::tokenize(original, Engine::MvMz);
        assert_eq!(tok.map.len(), 2);
        let restored = Tokenizer::restore(&tok.text, &tok.map).unwrap();
        assert_eq!(restored, original);
    }

    // 12. Description multi-ligne — \n littéral doit être tokenisé
    #[test]
    fn test_multiline_description_round_trip() {
        let original = "動脈に直接打ち込む注射式アンプル\n使用者のHPを回復する";
        let tokenized = Tokenizer::tokenize(original, Engine::MvMz);
        assert_eq!(tokenized.map.len(), 1, "one newline → one token");
        assert!(
            !tokenized.text.contains('\n'),
            "tokenized text must not contain literal newline"
        );
        assert_eq!(tokenized.map.get("⟦ph_0⟧").unwrap(), "\n");
        let restored = Tokenizer::restore(&tokenized.text, &tokenized.map).unwrap();
        assert_eq!(restored, original);
    }

    // 13. Description multi-ligne + codes RPG Maker — round-trip complet
    #[test]
    fn test_multiline_with_rpg_codes_round_trip() {
        let original = "\\V[12]のダメージ！\n\\C[3]回復した。";
        let tokenized = Tokenizer::tokenize(original, Engine::MvMz);
        // \V[12] + \n + \C[3] = 3 tokens
        assert_eq!(tokenized.map.len(), 3);
        assert!(!tokenized.text.contains('\n'));
        let restored = Tokenizer::restore(&tokenized.text, &tokenized.map).unwrap();
        assert_eq!(restored, original);
    }

    // 14. \n[1] (code RPG Maker) ne doit PAS être confondu avec \n littéral
    #[test]
    fn test_rpg_newline_code_vs_literal_newline() {
        // \n[1] = code RPG Maker "nom du personnage 1" — tokenisé via Groupe A
        // \n (U+000A) = saut de ligne littéral — tokenisé via le nouveau pattern
        let original = "\\n[1]\nTexte suivant";
        let tokenized = Tokenizer::tokenize(original, Engine::MvMz);
        assert_eq!(
            tokenized.map.len(),
            2,
            "\\n[1] and literal \\n are two distinct tokens"
        );
        let restored = Tokenizer::restore(&tokenized.text, &tokenized.map).unwrap();
        assert_eq!(restored, original);
    }

    // 11. Codes lowercase (\n[n], \v[n], \c[n]) — variantes community plugins
    #[test]
    fn test_lowercase_codes_tokenized() {
        // \n[1] (lowercase n) must be tokenized like \N[1] (uppercase)
        let result = Tokenizer::tokenize(r"\n[1]", Engine::MvMz);
        assert_eq!(result.text, "⟦ph_0⟧");
        assert_eq!(result.map.get("⟦ph_0⟧").unwrap(), r"\n[1]");

        // Mixed: \n[1] + real text — token present but content remains
        let result2 = Tokenizer::tokenize(r"\n[1] 反応なし…", Engine::MvMz);
        assert_eq!(result2.map.len(), 1);
        assert!(result2.text.contains("反応なし…"));

        // Round-trip must preserve lowercase form
        let original = r"\c[3]\n[1] テスト";
        let tok = Tokenizer::tokenize(original, Engine::MvMz);
        let restored = Tokenizer::restore(&tok.text, &tok.map).unwrap();
        assert_eq!(restored, original);
    }
}
