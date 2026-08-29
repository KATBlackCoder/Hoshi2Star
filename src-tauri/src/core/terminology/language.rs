use unicode_normalization::UnicodeNormalization;

fn primary_language(language: &str) -> &str {
    language.trim().split(['-', '_']).next().unwrap_or(language)
}

fn is_japanese_letter(character: char) -> bool {
    matches!(
        character,
        '\u{3041}'..='\u{3096}'
            | '\u{30a1}'..='\u{30fa}'
            | '\u{31f0}'..='\u{31ff}'
            | '\u{3400}'..='\u{4dbf}'
            | '\u{4e00}'..='\u{9fff}'
            | '\u{f900}'..='\u{faff}'
            | 'ー'
            | '々'
            | '〆'
            | 'ゝ'
            | 'ゞ'
            | 'ヽ'
            | 'ヾ'
    )
}

fn is_latin_letter(character: char) -> bool {
    character.is_alphabetic()
        && !matches!(
            character,
            '\u{3040}'..='\u{30ff}'
                | '\u{31f0}'..='\u{31ff}'
                | '\u{3400}'..='\u{9fff}'
                | '\u{f900}'..='\u{faff}'
                | '\u{0400}'..='\u{052f}'
        )
}

pub fn source_lexical_spans(text: &str, source_language: &str) -> Vec<String> {
    let normalized = text.nfkc().collect::<String>();
    match primary_language(source_language) {
        language if language.eq_ignore_ascii_case("ja") => {
            collect_spans(&normalized, is_japanese_letter, |_| false)
        }
        language if language.eq_ignore_ascii_case("en") || language.eq_ignore_ascii_case("fr") => {
            collect_spans(&normalized, is_latin_letter, |character| {
                matches!(character, '\'' | '’' | '-')
            })
        }
        _ => normalized
            .split_whitespace()
            .map(str::trim)
            .filter(|part| !part.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
    }
}

fn collect_spans(
    text: &str,
    is_letter: impl Fn(char) -> bool,
    is_connector: impl Fn(char) -> bool,
) -> Vec<String> {
    let characters = text.chars().collect::<Vec<_>>();
    let mut spans = Vec::new();
    let mut current = String::new();
    for (index, character) in characters.iter().copied().enumerate() {
        let connector_is_internal = is_connector(character)
            && index > 0
            && index + 1 < characters.len()
            && is_letter(characters[index - 1])
            && is_letter(characters[index + 1]);
        if is_letter(character) || connector_is_internal {
            current.push(character);
        } else if !current.is_empty() {
            spans.push(std::mem::take(&mut current));
        }
    }
    if !current.is_empty() {
        spans.push(current);
    }
    spans
}

pub fn is_pure_for_source(text: &str, source_language: &str) -> bool {
    let normalized = text.nfkc().collect::<String>();
    let trimmed = normalized.trim();
    if trimmed.is_empty() {
        return false;
    }
    let spans = source_lexical_spans(trimmed, source_language);
    spans.len() == 1 && spans[0] == trimmed
}

pub fn is_reusable_lexical_unit(text: &str, source_language: &str) -> bool {
    if !is_pure_for_source(text, source_language) {
        return false;
    }
    let length = text.chars().count();
    if length == 0 || length > 24 {
        return false;
    }
    if primary_language(source_language).eq_ignore_ascii_case("ja") {
        let all_hiragana = text
            .chars()
            .all(|character| matches!(character, '\u{3041}'..='\u{3096}'));
        if all_hiragana && length > 10 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn japanese_entries_are_script_pure() {
        for accepted in ["敵", "エネミー", "おじ共", "回復量", "時々", "一ヶ月"] {
            assert!(is_pure_for_source(accepted, "ja"), "{accepted}");
        }
        for rejected in [
            "Enemy",
            "Bad",
            "EXP",
            "GP",
            "EXP獲得",
            "HP回復量",
            "エネミーEnemy",
            "-----エネミー",
        ] {
            assert!(!is_pure_for_source(rejected, "ja"), "{rejected}");
        }
    }

    #[test]
    fn japanese_spans_drop_foreign_runs_and_decoration() {
        assert_eq!(source_lexical_spans("EXP獲得", "ja"), vec!["獲得"]);
        assert_eq!(source_lexical_spans("HP回復量", "ja"), vec!["回復量"]);
        assert_eq!(
            source_lexical_spans("エネミーEnemy", "ja"),
            vec!["エネミー"]
        );
        assert_eq!(
            source_lexical_spans("-----エネミー", "ja"),
            vec!["エネミー"]
        );
        assert!(source_lexical_spans("Bad EXP GP", "ja").is_empty());
    }

    #[test]
    fn long_hiragana_fragments_are_not_library_terms() {
        assert!(!is_reusable_lexical_unit(
            "ぐにつけあがってくるんだからね",
            "ja"
        ));
        assert!(is_reusable_lexical_unit("エネミー", "ja"));
    }

    #[test]
    fn latin_ownership_uses_declared_language() {
        assert!(is_pure_for_source("enemy", "en"));
        assert!(is_pure_for_source("ennemi", "fr"));
        assert!(is_pure_for_source("l’homme", "fr"));
        assert!(!is_pure_for_source("敵", "en"));
        assert!(!is_pure_for_source("enemy敵", "en"));
    }
}
