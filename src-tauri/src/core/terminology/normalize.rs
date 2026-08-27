use unicode_normalization::UnicodeNormalization;

use super::{Result, TerminologyError};

pub fn normalize_language(language: &str) -> Result<String> {
    let normalized = language.trim().to_lowercase();
    let valid = (2..=16).contains(&normalized.len())
        && normalized.split('-').all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|character| character.is_ascii_alphanumeric())
        });
    if !valid {
        return Err(TerminologyError::InvalidInput(format!(
            "invalid language code `{language}`"
        )));
    }
    Ok(normalized)
}

pub fn normalize_term(text: &str, source_language: &str) -> Result<String> {
    let collapsed = text
        .nfkc()
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if collapsed.is_empty() {
        return Err(TerminologyError::InvalidInput(
            "term text must not be empty".to_string(),
        ));
    }

    let primary_language = source_language.split('-').next().unwrap_or(source_language);
    if matches!(primary_language, "ja" | "zh" | "ko") {
        Ok(collapsed)
    } else {
        Ok(collapsed.to_lowercase())
    }
}

pub fn clean_optional_text(text: Option<&str>) -> Option<String> {
    text.map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn japanese_normalization_is_nfkc_but_keeps_script() {
        assert_eq!(
            normalize_term("  ＲＰＧ　勇者  ", "ja").unwrap(),
            "RPG 勇者"
        );
    }

    #[test]
    fn latin_normalization_collapses_space_and_case() {
        assert_eq!(
            normalize_term("  Dark   Knight ", "en").unwrap(),
            "dark knight"
        );
    }

    #[test]
    fn language_and_empty_values_are_rejected() {
        assert_eq!(normalize_language(" PT-BR ").unwrap(), "pt-br");
        assert!(normalize_language("fr_FR").is_err());
        assert!(normalize_term("　 ", "ja").is_err());
    }
}
