use std::borrow::Cow;

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;

use super::{LinguisticToken, MorphologicalAnalyzer};
use crate::core::terminology::types::PartOfSpeech;
use crate::core::terminology::{Result, TerminologyError};

pub struct JapaneseAnalyzer {
    segmenter: Segmenter,
    version: String,
}

impl JapaneseAnalyzer {
    pub fn new_embedded_ipadic() -> Result<Self> {
        let dictionary = load_dictionary("embedded://ipadic")
            .map_err(|error| TerminologyError::Analyzer(error.to_string()))?;
        Ok(Self {
            segmenter: Segmenter::new(Mode::Normal, dictionary, None),
            version: format!("lindera-{}/ipadic", lindera::get_version()),
        })
    }
}

impl MorphologicalAnalyzer for JapaneseAnalyzer {
    fn version(&self) -> &str {
        &self.version
    }

    fn analyze(&self, text: &str) -> Result<Vec<LinguisticToken>> {
        let mut tokens = self
            .segmenter
            .segment(Cow::Borrowed(text))
            .map_err(|error| TerminologyError::Analyzer(error.to_string()))?;
        let mut result = Vec::with_capacity(tokens.len());
        for token in &mut tokens {
            let surface = token.surface.to_string();
            let byte_start = token.byte_start;
            let byte_end = token.byte_end;
            let details = token.details();
            let is_unknown = details.first().is_none_or(|detail| *detail == "UNK");
            let part_of_speech = map_part_of_speech(&details, &surface, is_unknown);
            let lemma = detail(&details, 6)
                .filter(|value| *value != "*")
                .unwrap_or(surface.as_str())
                .to_string();
            let reading = detail(&details, 7)
                .filter(|value| *value != "*")
                .map(ToOwned::to_owned);
            let conjugation = [detail(&details, 4), detail(&details, 5)]
                .into_iter()
                .flatten()
                .filter(|value| *value != "*")
                .collect::<Vec<_>>()
                .join(",");

            result.push(LinguisticToken {
                surface,
                lemma,
                reading,
                part_of_speech,
                pos_detail: (!details.is_empty()).then(|| details.join(",")),
                conjugation: (!conjugation.is_empty()).then_some(conjugation),
                byte_start,
                byte_end,
                is_unknown,
            });
        }
        Ok(result)
    }
}

fn detail<'a>(details: &'a [&str], index: usize) -> Option<&'a str> {
    details.get(index).copied()
}

fn map_part_of_speech(details: &[&str], surface: &str, is_unknown: bool) -> PartOfSpeech {
    if is_unknown {
        return if contains_japanese(surface) {
            PartOfSpeech::Noun
        } else {
            PartOfSpeech::Unknown
        };
    }

    match (detail(details, 0), detail(details, 1)) {
        (Some("名詞"), Some("固有名詞")) => PartOfSpeech::ProperNoun,
        (Some("名詞"), Some("形容動詞語幹")) => PartOfSpeech::Adjective,
        (Some("名詞"), _) => PartOfSpeech::Noun,
        (Some("動詞"), Some("非自立")) => PartOfSpeech::Unknown,
        (Some("動詞"), _) => PartOfSpeech::Verb,
        (Some("形容詞"), _) | (Some("連体詞"), _) => PartOfSpeech::Adjective,
        (Some("副詞"), _) => PartOfSpeech::Adverb,
        (Some("感動詞"), _) => PartOfSpeech::Expression,
        _ => PartOfSpeech::Unknown,
    }
}

fn contains_japanese(text: &str) -> bool {
    text.chars().any(|character| {
        matches!(
            character,
            '\u{3040}'..='\u{30ff}' | '\u{3400}'..='\u{4dbf}' | '\u{4e00}'..='\u{9fff}'
        )
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::core::terminology::analyzer::filters::is_terminology_candidate;
    use crate::llm::tokenizer::{Engine, Tokenizer};

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct FixtureCase {
        id: String,
        text: String,
        expected: Vec<ExpectedToken>,
        excluded: Vec<String>,
    }

    #[derive(Debug, Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct ExpectedToken {
        lemma: String,
        part_of_speech: PartOfSpeech,
    }

    fn protected_mv_mz_text(text: &str) -> String {
        let tokenized = Tokenizer::tokenize(text, Engine::MvMz);
        tokenized
            .map
            .keys()
            .fold(tokenized.text, |safe, token| safe.replace(token, " "))
    }

    #[test]
    fn fixture_exposes_expected_lemmas_and_excludes_engine_codes() {
        let analyzer = JapaneseAnalyzer::new_embedded_ipadic().unwrap();
        let fixture: Vec<FixtureCase> = serde_json::from_str(include_str!(
            "../../../../tests/fixtures/terminology/ja_tokens.json"
        ))
        .unwrap();

        for case in fixture {
            let safe_text = protected_mv_mz_text(&case.text);
            let candidates = analyzer
                .analyze(&safe_text)
                .unwrap()
                .into_iter()
                .filter(is_terminology_candidate)
                .collect::<Vec<_>>();

            for expected in case.expected {
                assert!(
                    candidates.iter().any(|token| {
                        token.lemma == expected.lemma
                            && token.part_of_speech == expected.part_of_speech
                    }),
                    "{} missing {:?}; got {:?}",
                    case.id,
                    expected,
                    candidates
                        .iter()
                        .map(|token| (&token.lemma, token.part_of_speech))
                        .collect::<Vec<_>>()
                );
            }
            for excluded in case.excluded {
                assert!(
                    candidates.iter().all(|token| token.lemma != excluded),
                    "{} unexpectedly retained {excluded}",
                    case.id
                );
            }
        }
    }

    #[test]
    fn analyzer_reports_stable_version_and_offsets() {
        let analyzer = JapaneseAnalyzer::new_embedded_ipadic().unwrap();
        assert!(analyzer.version().starts_with("lindera-5."));
        let tokens = analyzer.analyze("勇者が走った").unwrap();
        assert_eq!(
            &"勇者が走った"[tokens[0].byte_start..tokens[0].byte_end],
            "勇者"
        );
        assert!(tokens.iter().any(|token| token.lemma == "走る"));
    }
}
