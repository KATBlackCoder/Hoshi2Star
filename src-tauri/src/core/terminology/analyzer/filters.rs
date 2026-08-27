use super::LinguisticToken;
use crate::core::terminology::types::PartOfSpeech;

pub fn is_terminology_candidate(token: &LinguisticToken) -> bool {
    let lemma = token.lemma.trim();
    if lemma.is_empty() || lemma.chars().all(|character| character.is_ascii_digit()) {
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
        if matches!(character, '\u{3040}'..='\u{30ff}') {
            return false;
        }
    }

    lemma.chars().any(|character| {
        matches!(
            character,
            '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}'
        ) || character.is_ascii_alphabetic()
    })
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
        assert!(is_terminology_candidate(&token("勇者", PartOfSpeech::Noun)));
        assert!(is_terminology_candidate(&token("剣", PartOfSpeech::Noun)));
        assert!(is_terminology_candidate(&token(
            "美しい",
            PartOfSpeech::Adjective
        )));
        assert!(!is_terminology_candidate(&token(
            "は",
            PartOfSpeech::Unknown
        )));
        assert!(!is_terminology_candidate(&token("100", PartOfSpeech::Noun)));
        assert!(!is_terminology_candidate(&token("を", PartOfSpeech::Noun)));
    }
}
