use super::types::PartOfSpeech;
use super::Result;

pub mod filters;
pub mod japanese;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinguisticToken {
    pub surface: String,
    pub lemma: String,
    pub reading: Option<String>,
    pub part_of_speech: PartOfSpeech,
    pub pos_detail: Option<String>,
    pub conjugation: Option<String>,
    pub byte_start: usize,
    pub byte_end: usize,
    pub is_unknown: bool,
}

pub trait MorphologicalAnalyzer: Send + Sync {
    fn version(&self) -> &str;
    fn analyze(&self, text: &str) -> Result<Vec<LinguisticToken>>;
}
