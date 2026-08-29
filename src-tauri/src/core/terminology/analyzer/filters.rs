use super::LinguisticToken;
use crate::core::terminology::language::is_reusable_lexical_unit;
use crate::core::terminology::types::PartOfSpeech;

pub const TERMINOLOGY_FILTER_VERSION: &str = "language-pure-lexical-v5";

pub fn is_terminology_candidate(token: &LinguisticToken, source_language: &str) -> bool {
    let lemma = token.lemma.trim();
    if lemma.is_empty() || lemma.chars().all(|character| character.is_ascii_digit()) {
        return false;
    }
    if !is_reusable_lexical_unit(lemma, source_language) {
        return false;
    }
    if !matches!(
        token.part_of_speech,
        PartOfSpeech::Noun
            | PartOfSpeech::ProperNoun
            | PartOfSpeech::Verb
            | PartOfSpeech::Adjective
            | PartOfSpeech::Adverb
            | PartOfSpeech::Expression
    ) {
        return false;
    }

    let mut characters = lemma.chars();
    if let (Some(character), None) = (characters.next(), characters.next()) {
        if is_japanese_syllabary(character) {
            return false;
        }
    }

    true
}

pub fn matches_source_language(text: &str, source_language: &str) -> bool {
    is_reusable_lexical_unit(text, source_language)
}

fn is_japanese_syllabary(character: char) -> bool {
    matches!(
        character,
        '\u{3040}'..='\u{30ff}' | '\u{31f0}'..='\u{31ff}' | '\u{ff66}'..='\u{ff9f}'
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn token(lemma: &str, part_of_speech: PartOfSpeech) -> LinguisticToken {
        LinguisticToken {
            surface: lemma.to_string(),
            lemma: lemma.to_string(),
            reading: None,
            part_of_speech,
            pos_detail: None,
            conjugation: None,
            byte_start: 0,
            byte_end: lemma.len(),
            is_unknown: false,
        }
    }

    #[test]
    fn keeps_content_words_and_rejects_noise() {
        assert!(is_terminology_candidate(
            &token("勇者", PartOfSpeech::Noun),
            "ja"
        ));
        assert!(is_terminology_candidate(
            &token("剣", PartOfSpeech::Noun),
            "ja"
        ));
        assert!(is_terminology_candidate(
            &token("美しい", PartOfSpeech::Adjective),
            "ja"
        ));
        assert!(!is_terminology_candidate(
            &token("は", PartOfSpeech::Unknown),
            "ja"
        ));
        assert!(!is_terminology_candidate(
            &token("100", PartOfSpeech::Noun),
            "ja"
        ));
        assert!(!is_terminology_candidate(
            &token("を", PartOfSpeech::Noun),
            "ja"
        ));
    }

    #[test]
    fn japanese_source_rejects_ascii_and_mixed_terms() {
        for text in [
            "Bad",
            "Clear",
            "ED",
            "EXP",
            "End",
            "GP",
            "EXP獲得",
            "HP回復量",
        ] {
            assert!(!is_terminology_candidate(
                &token(text, PartOfSpeech::ProperNoun),
                "ja"
            ));
        }

        assert!(is_terminology_candidate(
            &token("獲得", PartOfSpeech::Noun),
            "ja-JP"
        ));
        assert!(is_terminology_candidate(
            &token("ﾎﾟｰｼｮﾝ", PartOfSpeech::Noun),
            "ja"
        ));
    }

    #[test]
    fn script_gate_is_specific_to_the_source_language() {
        assert!(is_terminology_candidate(
            &token("Bad", PartOfSpeech::Adjective),
            "en"
        ));
        assert!(is_terminology_candidate(
            &token("Épée", PartOfSpeech::Noun),
            "fr"
        ));
        assert!(!matches_source_language("EXP", "ja"));
        assert!(matches_source_language("EXP", "en"));
    }
}
