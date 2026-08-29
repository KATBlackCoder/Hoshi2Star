//! QA engine — checks each segment before it is saved to DB.
//!
//! ## Checks (in priority order)
//! 1. **Placeholders** — every escape code present in the source (`\V[n]`, `\C[n]`, …)
//!    must also appear in the target.  Uses the tokenizer to enumerate them.
//! 2. **Line width** — MV/MZ message boxes have ~720 px of usable width.
//!    Full-width characters (CJK, hiragana, katakana) count as 2 half-width units;
//!    ASCII and other half-width characters count as 1 unit.
//!    The configurable limit is ~55.38 units (720 px / 13 px per half-width char).
//!    Lines beyond `LineWidthConfig::max_lines` are ignored.
//!    Empty lines are skipped.
//! 3. **BOM** — a UTF-8 BOM (`\u{FEFF}`) at the start of the target causes
//!    mojibake in the game engine.
//!
//! ## Score
//! 100 if 0 errors.  Per error:
//! - `MissingPlaceholder`  → −25
//! - `LineTooLong`         → −10
//! - `BomDetected`         → −15
//! - `TerminologyMismatch` → severity-dependent (0/15/35)
//!
//! Minimum score: 0.

use crate::core::terminology::{
    resolver::QaTerminologyRule,
    types::{Enforcement, PartOfSpeech},
};
use crate::llm::tokenizer::{Engine as TokEngine, Tokenizer};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

/// Layout parameters for RPG Maker MV/MZ message boxes.
///
/// Default values are calibrated for the standard MV/MZ 816 px window
/// with ~48 px margins → ~720 px usable, using the default game font
/// (DejaVu Sans Mono-like metrics: 26 px full-width, 13 px half-width).
///
/// In F3, `LineWidthConfig` will be exposed as a per-project setting.
#[derive(Debug, Clone)]
pub struct LineWidthConfig {
    /// Usable box width in pixels (default: 720).
    pub box_width_px: f32,
    /// Width of one full-width character in pixels (default: 26.0).
    pub fullwidth_char_px: f32,
    /// Width of one half-width character in pixels (default: 13.0).
    pub halfwidth_char_px: f32,
    /// Maximum number of lines to check per segment (default: 4).
    pub max_lines: usize,
}

impl Default for LineWidthConfig {
    fn default() -> Self {
        Self {
            box_width_px: 720.0,
            fullwidth_char_px: 26.0,
            halfwidth_char_px: 13.0,
            max_lines: 4,
        }
    }
}

impl LineWidthConfig {
    /// Maximum line width in half-width units (derived: `box_width_px / halfwidth_char_px`).
    ///
    /// With default values: 720 / 13 ≈ 55.38 units.
    pub fn max_halfwidth_units(&self) -> f32 {
        self.box_width_px / self.halfwidth_char_px
    }

    /// Conservative defaults for Wolf RPG message boxes.
    ///
    /// Wolf v2 default window: ~520 px usable, 26 px full-width, 13 px half-width.
    /// ⚠️ This is an estimate — the actual box width is configurable in the Wolf editor.
    pub fn wolf_default() -> Self {
        Self {
            box_width_px: 520.0,
            fullwidth_char_px: 26.0,
            halfwidth_char_px: 13.0,
            max_lines: 4,
        }
    }
}

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum QaError {
    EmptyTranslation,
    UnchangedSource,
    SourceScriptRemaining {
        source_language: String,
    },
    SuspiciousExpansion {
        source_chars: usize,
        target_chars: usize,
    },
    ContextLeak {
        neighbor_text: String,
    },
    InconsistentRepeatedSource {
        variants: usize,
    },
    MissingPlaceholder {
        placeholder: String,
    },
    LineTooLong {
        /// 1-based line number.
        line: usize,
        /// Measured width in half-width units (full-width = 2, half-width = 1).
        units: f32,
        /// Configured maximum width in half-width units.
        max_units: f32,
        /// Raw character count of the line.
        char_count: usize,
    },
    BomDetected,
    /// An occurrence-scoped terminology rule whose accepted target is absent.
    TerminologyMismatch {
        entry_id: String,
        source_term: String,
        expected_targets: Vec<String>,
        severity: QaSeverity,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QaSeverity {
    Critical,
    Warning,
    Info,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QaResult {
    pub score: u8,
    pub errors: Vec<QaError>,
}

impl QaResult {
    pub fn has_critical_errors(&self) -> bool {
        self.errors.iter().any(QaError::is_critical)
    }

    pub fn add_error(&mut self, error: QaError) {
        self.errors.push(error);
        let penalty: i32 = self.errors.iter().map(QaError::penalty).sum();
        self.score = (100 - penalty).max(0) as u8;
    }
}

/// Additional facts available to the automatic translation pipeline and full
/// project audit. The basic editor QA remains available through [`check`].
pub struct QaSemanticContext<'a> {
    pub source_language: &'a str,
    pub target_language: &'a str,
    pub segment_kind: &'a str,
    pub neighbor_sources: &'a [String],
    pub neighbor_targets: &'a [String],
}

// ---------------------------------------------------------------------------
// Width measurement
// ---------------------------------------------------------------------------

/// Returns `true` if `c` is a full-width character (CJK, kana, full-width ASCII/symbols).
fn is_fullwidth(c: char) -> bool {
    matches!(
        c,
        // CJK Unified Ideographs
        '\u{4E00}'..='\u{9FFF}'
        // Hiragana
        | '\u{3040}'..='\u{309F}'
        // Katakana
        | '\u{30A0}'..='\u{30FF}'
        // Halfwidth/Fullwidth Forms block — FULL-width sub-ranges only
        // (East Asian Width = F). FF61–FF9F (half-width katakana/punctuation),
        // FFA0–FFDF (half-width jamo) and FFE8–FFEE are half-width: 1 unit.
        | '\u{FF00}'..='\u{FF60}'
        | '\u{FFE0}'..='\u{FFE6}'
        // CJK Compatibility Ideographs
        | '\u{F900}'..='\u{FAFF}'
        // CJK Extension A
        | '\u{3400}'..='\u{4DBF}'
        // CJK Symbols and Punctuation
        | '\u{3000}'..='\u{303F}'
    )
}

/// Measures the display width of `line` in half-width units.
///
/// Full-width characters count as 2 units; all other characters count as 1 unit.
pub fn measure_line_units(line: &str) -> f32 {
    line.chars()
        .map(|c| if is_fullwidth(c) { 2.0_f32 } else { 1.0_f32 })
        .sum()
}

// ---------------------------------------------------------------------------
// Internal checks
// ---------------------------------------------------------------------------

fn check_terminology(target: &str, rules: &[QaTerminologyRule]) -> Vec<QaError> {
    let target_lower = target.to_lowercase();
    rules
        .iter()
        .filter_map(|rule| {
            let mut accepted = vec![rule.target.clone()];
            accepted.extend(rule.accepted_targets.iter().cloned());
            if accepted
                .iter()
                .any(|candidate| target_lower.contains(&candidate.to_lowercase()))
            {
                return None;
            }
            let severity = terminology_severity(rule);
            Some(QaError::TerminologyMismatch {
                entry_id: rule.entry_id.clone(),
                source_term: rule.source.clone(),
                expected_targets: accepted,
                severity,
            })
        })
        .collect()
}

fn terminology_severity(rule: &QaTerminologyRule) -> QaSeverity {
    if rule.enforcement == Enforcement::Contextual {
        return QaSeverity::Info;
    }
    let stable_entity = matches!(
        rule.part_of_speech,
        PartOfSpeech::Noun | PartOfSpeech::ProperNoun
    ) && matches!(
        rule.semantic_type.as_str(),
        "character"
            | "speaker"
            | "actor"
            | "class"
            | "item"
            | "weapon"
            | "armor"
            | "skill"
            | "enemy"
            | "state"
            | "map"
            | "location"
            | "currency"
            | "system_term"
            | "game_title"
            | "object"
    );
    if rule.enforcement == Enforcement::Required && stable_entity {
        QaSeverity::Critical
    } else {
        QaSeverity::Warning
    }
}

fn legacy_rules(source: &str, terms: &[(String, String)]) -> Vec<QaTerminologyRule> {
    terms
        .iter()
        .filter_map(|(source_term, target_term)| {
            if !source.contains(source_term) {
                None
            } else {
                Some(QaTerminologyRule {
                    entry_id: String::new(),
                    source: source_term.clone(),
                    target: target_term.clone(),
                    semantic_type: "general".to_string(),
                    part_of_speech: PartOfSpeech::Unknown,
                    enforcement: Enforcement::Preferred,
                    accepted_targets: Vec::new(),
                })
            }
        })
        .collect()
}

fn check_line_length(text: &str, config: &LineWidthConfig) -> Vec<QaError> {
    let max_units = config.max_halfwidth_units();
    text.lines()
        .take(config.max_lines)
        .enumerate()
        .filter_map(|(i, line)| {
            if line.trim().is_empty() {
                return None;
            }
            let units = measure_line_units(line);
            if units > max_units {
                Some(QaError::LineTooLong {
                    line: i + 1,
                    units,
                    max_units,
                    char_count: line.chars().count(),
                })
            } else {
                None
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Run all QA checks on a (source, target) pair.
///
/// `source` is the original Japanese text; `target` is the translation.
/// `legacy_terms` is retained only for the historical pack compatibility
/// layer. Runtime translation and reports use occurrence-scoped rules.
/// `engine` selects placeholder patterns and line-width config:
///   `"wolf"` → Wolf RPG placeholders + 520 px box
///   any other value → MV/MZ placeholders + 720 px box
/// Returns a `QaResult` containing the score and the list of errors found.
pub fn check(
    source: &str,
    target: &str,
    legacy_terms: &[(String, String)],
    engine: &str,
) -> QaResult {
    let terminology_rules = legacy_rules(source, legacy_terms);
    check_with_rules(source, target, &terminology_rules, engine)
}

fn check_with_rules(
    source: &str,
    target: &str,
    terminology_rules: &[QaTerminologyRule],
    engine: &str,
) -> QaResult {
    let mut errors: Vec<QaError> = Vec::new();

    let tok_engine = TokEngine::from_project_engine(engine);
    let line_config = if engine == "wolf" {
        LineWidthConfig::wolf_default()
    } else {
        LineWidthConfig::default()
    };

    // 1. BOM check (cheapest — do first)
    if target.starts_with('\u{FEFF}') {
        errors.push(QaError::BomDetected);
    }

    // 2. Placeholder check
    //    Tokenise the source to enumerate unique original placeholders,
    //    then verify each one is present in the (raw) target text.
    let tokenized = Tokenizer::tokenize(source, tok_engine);
    let mut seen: HashSet<&str> = HashSet::new();
    for original in tokenized.map.values() {
        if seen.insert(original.as_str()) && !target.contains(original.as_str()) {
            errors.push(QaError::MissingPlaceholder {
                placeholder: original.clone(),
            });
        }
    }

    // 3. Line width check
    errors.extend(check_line_length(target, &line_config));

    // 4. Occurrence-scoped terminology consistency check
    if !terminology_rules.is_empty() {
        errors.extend(check_terminology(target, terminology_rules));
    }

    // Score calculation
    let penalty: i32 = errors.iter().map(QaError::penalty).sum();

    let score = (100 - penalty).max(0) as u8;

    QaResult { score, errors }
}

/// Run structural QA plus conservative semantic checks suitable for automatic
/// persistence. These checks deliberately favour `needs_review` over silently
/// accepting a contaminated model output.
pub fn check_with_context(
    source: &str,
    target: &str,
    terminology_rules: &[QaTerminologyRule],
    engine: &str,
    context: &QaSemanticContext<'_>,
) -> QaResult {
    let mut result = check_with_rules(source, target, terminology_rules, engine);
    let source_trimmed = source.trim();
    let target_trimmed = target.trim();

    if target_trimmed.is_empty() {
        result.errors.push(QaError::EmptyTranslation);
    } else {
        if normalized_semantic_text(source_trimmed) == normalized_semantic_text(target_trimmed)
            && unchanged_text_requires_translation(source_trimmed, context.source_language)
        {
            result.errors.push(QaError::UnchangedSource);
        }

        if context.source_language != context.target_language
            && contains_source_script(target_trimmed, context.source_language)
        {
            result.errors.push(QaError::SourceScriptRemaining {
                source_language: context.source_language.to_string(),
            });
        }

        let source_chars = source_trimmed.chars().count();
        let target_chars = target_trimmed.chars().count();
        let speaker_expansion = context.segment_kind == "speaker"
            && target_chars > 40
            && target_chars > source_chars.saturating_mul(4);
        let short_text_expansion = source_chars <= 20
            && target_chars > 96
            && target_chars > source_chars.saturating_mul(8);
        if speaker_expansion || short_text_expansion {
            result.errors.push(QaError::SuspiciousExpansion {
                source_chars,
                target_chars,
            });
        }

        let normalized_target = normalized_semantic_text(target_trimmed);
        let source_leak = context.neighbor_sources.iter().find(|neighbor| {
            let normalized_neighbor = normalized_semantic_text(neighbor);
            normalized_neighbor.chars().count() >= 4
                && normalized_target.contains(&normalized_neighbor)
        });
        let translated_leak = (target_chars > source_chars.saturating_mul(4)).then(|| {
            context
                .neighbor_targets
                .iter()
                .find(|neighbor| shares_word_sequence(target_trimmed, neighbor, 4))
        });
        if let Some(neighbor) = source_leak.or_else(|| translated_leak.flatten()) {
            result.errors.push(QaError::ContextLeak {
                neighbor_text: neighbor.clone(),
            });
        }
    }

    let penalty: i32 = result.errors.iter().map(QaError::penalty).sum();
    result.score = (100 - penalty).max(0) as u8;
    result
}

fn normalized_semantic_text(text: &str) -> String {
    text.split_whitespace()
        .flat_map(str::chars)
        .flat_map(char::to_lowercase)
        .collect()
}

fn shares_word_sequence(left: &str, right: &str, minimum_words: usize) -> bool {
    let words = |text: &str| {
        text.split(|character: char| !character.is_alphanumeric())
            .filter(|word| !word.is_empty())
            .map(str::to_lowercase)
            .collect::<Vec<_>>()
    };
    let left_words = words(left);
    let right_words = words(right);
    if left_words.len() < minimum_words || right_words.len() < minimum_words {
        return false;
    }
    left_words.windows(minimum_words).any(|left_window| {
        right_words
            .windows(minimum_words)
            .any(|right_window| left_window == right_window)
    })
}

fn contains_source_script(text: &str, source_language: &str) -> bool {
    text.chars().any(|character| match source_language {
        "ja" => matches!(
            character,
            '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}'
        ),
        "zh" => matches!(character, '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}'),
        "ko" => matches!(character, '\u{1100}'..='\u{11ff}' | '\u{3130}'..='\u{318f}' | '\u{ac00}'..='\u{d7af}'),
        "ru" | "uk" | "bg" => matches!(character, '\u{0400}'..='\u{04ff}'),
        _ => false,
    })
}

/// Acronyms, engine tokens and placeholders such as `EXP`, `GP`, `OP` or `%2`
/// are often intentionally identical in a Japanese-to-French localisation.
/// For languages with a distinctive source script, unchanged text is only a
/// translation failure when that script is actually present. Latin-source
/// languages retain the conservative unchanged-text warning.
fn unchanged_text_requires_translation(text: &str, source_language: &str) -> bool {
    match source_language {
        "ja" | "zh" | "ko" | "ru" | "uk" | "bg" => contains_source_script(text, source_language),
        _ => text.chars().any(char::is_alphabetic),
    }
}

// ---------------------------------------------------------------------------
// Label helpers (used by report.rs for HTML output)
// ---------------------------------------------------------------------------

impl QaError {
    /// Score penalty (points subtracted from 100) for this error kind.
    ///
    /// Single source of truth for the per-kind weighting — the score calculation
    /// and any future consumer read it here rather than re-spelling the match.
    pub fn penalty(&self) -> i32 {
        match self {
            QaError::EmptyTranslation => 100,
            QaError::UnchangedSource => 60,
            QaError::SourceScriptRemaining { .. } => 40,
            QaError::SuspiciousExpansion { .. } => 50,
            QaError::ContextLeak { .. } => 60,
            QaError::InconsistentRepeatedSource { .. } => 50,
            QaError::MissingPlaceholder { .. } => 25,
            QaError::LineTooLong { .. } => 10,
            QaError::BomDetected => 15,
            QaError::TerminologyMismatch { severity, .. } => match severity {
                QaSeverity::Critical => 35,
                QaSeverity::Warning => 15,
                QaSeverity::Info => 0,
            },
        }
    }

    /// Stable machine key for this error kind (CSS class / HTML filter value).
    pub fn type_key(&self) -> &'static str {
        match self {
            QaError::EmptyTranslation => "empty_translation",
            QaError::UnchangedSource => "unchanged_source",
            QaError::SourceScriptRemaining { .. } => "source_script_remaining",
            QaError::SuspiciousExpansion { .. } => "suspicious_expansion",
            QaError::ContextLeak { .. } => "context_leak",
            QaError::InconsistentRepeatedSource { .. } => "inconsistent_repeated_source",
            QaError::MissingPlaceholder { .. } => "missing_placeholder",
            QaError::LineTooLong { .. } => "line_too_long",
            QaError::BomDetected => "bom_detected",
            QaError::TerminologyMismatch { .. } => "terminology_mismatch",
        }
    }

    /// Critical errors must not be silently exported as completed work.
    pub fn is_critical(&self) -> bool {
        matches!(
            self,
            QaError::EmptyTranslation
                | QaError::UnchangedSource
                | QaError::SourceScriptRemaining { .. }
                | QaError::SuspiciousExpansion { .. }
                | QaError::ContextLeak { .. }
                | QaError::InconsistentRepeatedSource { .. }
                | QaError::MissingPlaceholder { .. }
                | QaError::BomDetected
                | QaError::TerminologyMismatch {
                    severity: QaSeverity::Critical,
                    ..
                }
        )
    }

    /// Human-readable label for this error in the requested language.
    ///
    /// `lang` is `"fr"` for French; any other value falls back to English.
    /// The returned string is plain text — callers must apply HTML escaping if
    /// embedding in markup.
    pub fn label(&self, lang: &str) -> String {
        if lang == "fr" {
            self.label_fr()
        } else {
            self.label_en()
        }
    }

    fn label_en(&self) -> String {
        match self {
            QaError::EmptyTranslation => "Empty translation".to_string(),
            QaError::UnchangedSource => "Translation is identical to the source".to_string(),
            QaError::SourceScriptRemaining { source_language } => {
                format!("Source-language script remains ({source_language})")
            }
            QaError::SuspiciousExpansion {
                source_chars,
                target_chars,
            } => format!(
                "Suspicious expansion ({source_chars} source chars → {target_chars} target chars)"
            ),
            QaError::ContextLeak { neighbor_text } => {
                format!("Neighbouring context copied into output: \"{neighbor_text}\"")
            }
            QaError::InconsistentRepeatedSource { variants } => {
                format!("Repeated source has {variants} different translations")
            }
            QaError::MissingPlaceholder { placeholder } => {
                format!("Missing placeholder: {placeholder}")
            }
            QaError::LineTooLong {
                line,
                units,
                max_units,
                char_count,
            } => {
                format!(
                    "Line {line} too wide ({units:.1} / {max_units:.1} units — {char_count} chars)"
                )
            }
            QaError::BomDetected => "UTF-8 BOM detected at start".to_string(),
            QaError::TerminologyMismatch {
                source_term,
                expected_targets,
                severity,
                ..
            } => {
                format!(
                    "Terminology {severity:?}: \"{source_term}\" → expected one of \"{}\"",
                    expected_targets.join("\", \"")
                )
            }
        }
    }

    fn label_fr(&self) -> String {
        match self {
            QaError::EmptyTranslation => "Traduction vide".to_string(),
            QaError::UnchangedSource => "La traduction est identique à la source".to_string(),
            QaError::SourceScriptRemaining { source_language } => {
                format!("Écriture de la langue source encore présente ({source_language})")
            }
            QaError::SuspiciousExpansion {
                source_chars,
                target_chars,
            } => format!(
                "Expansion suspecte ({source_chars} caract. source → {target_chars} caract. cible)"
            ),
            QaError::ContextLeak { neighbor_text } => {
                format!("Contexte voisin recopié dans la sortie : \"{neighbor_text}\"")
            }
            QaError::InconsistentRepeatedSource { variants } => {
                format!("La même source possède {variants} traductions différentes")
            }
            QaError::MissingPlaceholder { placeholder } => {
                format!("Placeholder manquant : {placeholder}")
            }
            QaError::LineTooLong {
                line,
                units,
                max_units,
                char_count,
            } => {
                format!(
                    "Ligne {line} trop longue ({units:.1} / {max_units:.1} unités — {char_count} caract.)"
                )
            }
            QaError::BomDetected => "BOM UTF-8 détecté en début de cible".to_string(),
            QaError::TerminologyMismatch {
                source_term,
                expected_targets,
                severity,
                ..
            } => {
                format!(
                    "Terminologie {severity:?} : \"{source_term}\" → une forme attendue parmi \"{}\"",
                    expected_targets.join("\", \"")
                )
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // --- is_fullwidth ---

    #[test]
    fn test_is_fullwidth_kanji() {
        assert!(is_fullwidth('日'));
        assert!(is_fullwidth('本'));
        assert!(is_fullwidth('語'));
    }

    #[test]
    fn test_is_fullwidth_hiragana() {
        assert!(is_fullwidth('あ'));
        assert!(is_fullwidth('い'));
        assert!(is_fullwidth('う'));
    }

    #[test]
    fn test_is_fullwidth_katakana() {
        assert!(is_fullwidth('ア'));
        assert!(is_fullwidth('カ'));
    }

    #[test]
    fn test_is_fullwidth_ascii_halfwidth() {
        assert!(!is_fullwidth('A'));
        assert!(!is_fullwidth('1'));
        assert!(!is_fullwidth(' '));
        assert!(!is_fullwidth('!'));
    }

    // --- measure_line_units ---

    #[test]
    fn test_measure_line_units_ascii() {
        assert_eq!(measure_line_units("ABC"), 3.0);
    }

    #[test]
    fn test_measure_line_units_japanese() {
        // 3 kanji × 2 = 6
        assert_eq!(measure_line_units("日本語"), 6.0);
    }

    #[test]
    fn test_measure_line_units_mixed() {
        // 'A'=1 + 'B'=1 + '日'=2 = 4
        assert_eq!(measure_line_units("AB日"), 4.0);
    }

    #[test]
    fn test_measure_line_units_halfwidth_katakana() {
        // U+FF61..=U+FF9F are HALF-width forms (East Asian Width = H):
        // 1 unit each, despite living in the FF00 block.
        assert_eq!(measure_line_units("ｱｲｳ"), 3.0);
        // Half-width punctuation (｡ U+FF61, ･ U+FF65) too.
        assert_eq!(measure_line_units("｡･"), 2.0);
        // Full-width forms of the same block stay 2 units (Ａ U+FF21, ￥ U+FFE5).
        assert_eq!(measure_line_units("Ａ￥"), 4.0);
    }

    // --- check_line_length ---

    #[test]
    fn test_check_line_length_empty_lines_ignored() {
        let errors = check_line_length("\n   \n", &LineWidthConfig::default());
        assert!(errors.is_empty());
    }

    #[test]
    fn test_check_line_length_jp_long_triggers_error() {
        // 28 kanji × 2 = 56.0 units > 55.38 max
        let line = "日".repeat(28);
        let errors = check_line_length(&line, &LineWidthConfig::default());
        assert_eq!(errors.len(), 1);
        match &errors[0] {
            QaError::LineTooLong {
                line: n,
                units,
                max_units,
                char_count,
            } => {
                assert_eq!(*n, 1);
                assert_eq!(*char_count, 28);
                assert!(
                    *units > *max_units,
                    "units={units} should exceed max_units={max_units}"
                );
            }
            _ => panic!("expected LineTooLong"),
        }
    }

    #[test]
    fn test_check_line_length_en_within_limit() {
        // 55 ASCII chars = 55.0 units < 55.38 max → no error
        let line = "A".repeat(55);
        let errors = check_line_length(&line, &LineWidthConfig::default());
        assert!(errors.is_empty());
    }

    // --- check() integration ---

    #[test]
    fn test_clean_segment_score_100() {
        let result = check(r"\V[12] pièces", r"\V[12] coins", &[], "mv_mz");
        assert_eq!(result.score, 100);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_missing_placeholder() {
        let result = check(r"\V[12] pièces", "coins", &[], "mv_mz");
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            QaError::MissingPlaceholder { placeholder } if placeholder == r"\V[12]"
        ));
        assert_eq!(result.score, 75);
    }

    #[test]
    fn test_multiple_missing_placeholders() {
        let result = check(r"\V[12] et \N[1]", "coins", &[], "mv_mz");
        assert_eq!(result.errors.len(), 2);
        assert_eq!(result.score, 50); // 100 - 25 - 25
    }

    #[test]
    fn test_line_too_long() {
        // 56 ASCII chars = 56.0 units > 55.38 max
        let long_line = "A".repeat(56);
        let result = check("hello", &long_line, &[], "mv_mz");
        assert_eq!(result.errors.len(), 1);
        match &result.errors[0] {
            QaError::LineTooLong {
                line,
                units,
                max_units,
                char_count,
            } => {
                assert_eq!(*line, 1);
                assert_eq!(*char_count, 56);
                assert!(*units > *max_units);
            }
            _ => panic!("expected LineTooLong"),
        }
        assert_eq!(result.score, 90); // 100 - 10
    }

    #[test]
    fn test_bom_detected() {
        let result = check("hello", "\u{FEFF}hello", &[], "mv_mz");
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(&result.errors[0], QaError::BomDetected));
        assert_eq!(result.score, 85); // 100 - 15
    }

    #[test]
    fn test_cumulative_score() {
        // BOM + missing placeholder + long line = -15 -25 -10 = 50
        // '\u{FEFF}' (1 unit) + 56 × 'A' (56 units) = 57.0 > 55.38 → LineTooLong
        let long_line = format!("\u{FEFF}{}", "A".repeat(56));
        let result = check(r"\V[12]", &long_line, &[], "mv_mz");
        assert_eq!(result.score, 50);
        assert_eq!(result.errors.len(), 3);
    }

    #[test]
    fn test_score_floor_zero() {
        // 4 missing placeholders = -100 → floor to 0
        let result = check(
            r"\V[1]\V[2]\V[3]\V[4]",
            "no placeholders here",
            &[],
            "mv_mz",
        );
        assert_eq!(result.score, 0);
    }

    #[test]
    fn test_no_source_placeholders_passes() {
        let result = check("こんにちは", "Hello", &[], "mv_mz");
        assert_eq!(result.score, 100);
        assert!(result.errors.is_empty());
    }

    #[test]
    fn test_lines_beyond_max_ignored() {
        // 5 lines of 56 'A' — only first 4 are checked (max_lines = 4)
        let lines: Vec<String> = (0..5).map(|_| "A".repeat(56)).collect();
        let text = lines.join("\n");
        let result = check("hello", &text, &[], "mv_mz");
        let long_errors: Vec<_> = result
            .errors
            .iter()
            .filter(|e| matches!(e, QaError::LineTooLong { .. }))
            .collect();
        assert_eq!(long_errors.len(), 4);
    }

    // --- terminology compatibility wrapper ---

    fn terms(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(s, t)| (s.to_string(), t.to_string()))
            .collect()
    }

    #[test]
    fn test_terminology_mismatch_detected() {
        let t = terms(&[("魔法使い", "Mage")]);
        let result = check("魔法使い が現れた！", "A sorcerer appeared!", &t, "mv_mz");
        assert_eq!(result.errors.len(), 1);
        assert!(matches!(
            &result.errors[0],
            QaError::TerminologyMismatch { source_term, expected_targets, severity, .. }
                if source_term == "魔法使い"
                    && expected_targets == &["Mage"]
                    && *severity == QaSeverity::Warning
        ));
        assert_eq!(result.score, 85); // 100 - 15
    }

    #[test]
    fn test_terminology_term_present_no_error() {
        // "Mage" IS in target (case-insensitive) → no error
        let t = terms(&[("魔法使い", "Mage")]);
        let result = check("魔法使い が現れた！", "A mage appeared!", &t, "mv_mz");
        assert!(result.errors.is_empty());
        assert_eq!(result.score, 100);
    }

    #[test]
    fn test_terminology_no_source_term_no_error() {
        // "魔法使い" not in source → compatibility rule is skipped
        let t = terms(&[("魔法使い", "Mage")]);
        let result = check("戦士 が現れた！", "A warrior appeared!", &t, "mv_mz");
        assert!(result.errors.is_empty());
        assert_eq!(result.score, 100);
    }

    #[test]
    fn test_terminology_empty_terms_no_error() {
        let result = check("魔法使い", "something else", &[], "mv_mz");
        assert!(result.errors.is_empty());
        assert_eq!(result.score, 100);
    }

    #[test]
    fn test_terminology_multiple_mismatches_floor_zero() {
        // 7 mismatches = -105 → floor to 0
        let t = terms(&[
            ("term1", "T1"),
            ("term2", "T2"),
            ("term3", "T3"),
            ("term4", "T4"),
            ("term5", "T5"),
            ("term6", "T6"),
            ("term7", "T7"),
        ]);
        let source = "term1 term2 term3 term4 term5 term6 term7";
        let result = check(source, "wrong translation", &t, "mv_mz");
        assert_eq!(result.score, 0);
    }

    fn rich_rule(enforcement: Enforcement, part_of_speech: PartOfSpeech) -> QaTerminologyRule {
        QaTerminologyRule {
            entry_id: "entry-1".to_string(),
            source: "勇者".to_string(),
            target: "Hero".to_string(),
            semantic_type: "character".to_string(),
            part_of_speech,
            enforcement,
            accepted_targets: vec!["The Hero".to_string()],
        }
    }

    #[test]
    fn required_entity_is_critical_and_accepts_variants() {
        let rule = rich_rule(Enforcement::Required, PartOfSpeech::ProperNoun);
        let mismatch = check_with_rules("勇者", "Champion", std::slice::from_ref(&rule), "mv_mz");
        assert!(matches!(
            mismatch.errors.as_slice(),
            [QaError::TerminologyMismatch {
                severity: QaSeverity::Critical,
                ..
            }]
        ));
        assert!(mismatch.has_critical_errors());
        assert!(
            check_with_rules("勇者", "The Hero arrives", &[rule], "mv_mz")
                .errors
                .is_empty()
        );
    }

    #[test]
    fn contextual_and_inflected_terms_never_block() {
        let contextual = rich_rule(Enforcement::Contextual, PartOfSpeech::ProperNoun);
        let verb = rich_rule(Enforcement::Required, PartOfSpeech::Verb);
        let contextual_result = check_with_rules("勇者", "Champion", &[contextual], "mv_mz");
        let verb_result = check_with_rules("勇者", "Champion", &[verb], "mv_mz");
        assert!(matches!(
            contextual_result.errors.as_slice(),
            [QaError::TerminologyMismatch {
                severity: QaSeverity::Info,
                ..
            }]
        ));
        assert!(!contextual_result.has_critical_errors());
        assert!(matches!(
            verb_result.errors.as_slice(),
            [QaError::TerminologyMismatch {
                severity: QaSeverity::Warning,
                ..
            }]
        ));
        assert!(!verb_result.has_critical_errors());
    }

    fn semantic_context<'a>(kind: &'a str, neighbors: &'a [String]) -> QaSemanticContext<'a> {
        QaSemanticContext {
            source_language: "ja",
            target_language: "fr",
            segment_kind: kind,
            neighbor_sources: neighbors,
            neighbor_targets: &[],
        }
    }

    #[test]
    fn semantic_qa_rejects_empty_unchanged_and_source_script() {
        let empty =
            check_with_context("勇者", " ", &[], "mv_mz", &semantic_context("speaker", &[]));
        assert!(matches!(
            empty.errors.as_slice(),
            [QaError::EmptyTranslation]
        ));
        assert!(empty.has_critical_errors());

        let unchanged = check_with_context(
            "勇者",
            "勇者",
            &[],
            "mv_mz",
            &semantic_context("speaker", &[]),
        );
        assert!(unchanged
            .errors
            .iter()
            .any(|error| matches!(error, QaError::UnchangedSource)));
        assert!(unchanged.errors.iter().any(|error| matches!(
            error,
            QaError::SourceScriptRemaining { source_language } if source_language == "ja"
        )));
    }

    #[test]
    fn semantic_qa_allows_language_neutral_tokens_to_remain_unchanged() {
        for text in ["EXP", "GP", "OP", "%2", "aaa"] {
            let result = check_with_context(
                text,
                text,
                &[],
                "mv_mz",
                &semantic_context("system_term", &[]),
            );
            assert!(result
                .errors
                .iter()
                .all(|error| !matches!(error, QaError::UnchangedSource)));
        }
    }

    #[test]
    fn semantic_qa_rejects_speaker_expansion_and_neighbor_copy() {
        let expanded = check_with_context(
            "男",
            &"Un très long paragraphe de dialogue inventé par le modèle. ".repeat(2),
            &[],
            "mv_mz",
            &semantic_context("speaker", &[]),
        );
        assert!(expanded
            .errors
            .iter()
            .any(|error| matches!(error, QaError::SuspiciousExpansion { .. })));

        let neighbors = vec!["ここから逃げてください".to_string()];
        let leaked = check_with_context(
            "はい",
            "ここから逃げてください",
            &[],
            "mv_mz",
            &semantic_context("dialogue", &neighbors),
        );
        assert!(leaked
            .errors
            .iter()
            .any(|error| matches!(error, QaError::ContextLeak { .. })));

        let translated_neighbors = vec!["We should go and continue the mission.".to_string()];
        let translated_leak = check_with_context(
            "行こう",
            "Let's go and continue the mission.",
            &[],
            "mv_mz",
            &QaSemanticContext {
                source_language: "ja",
                target_language: "en",
                segment_kind: "dialogue",
                neighbor_sources: &[],
                neighbor_targets: &translated_neighbors,
            },
        );
        assert!(translated_leak
            .errors
            .iter()
            .any(|error| matches!(error, QaError::ContextLeak { .. })));
    }
}
