use crate::core::terminology::types::PartOfSpeech;

#[derive(Debug, Clone, Copy)]
pub struct EngineSegmentContext<'a> {
    pub source_text: &'a str,
    pub segment_kind: &'a str,
    pub file_name: &'a str,
    pub file_type: &'a str,
    pub json_key: &'a str,
    pub speaker: Option<&'a str>,
    pub context_json: Option<&'a str>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EngineTermSeed {
    pub source_text: String,
    pub semantic_type: String,
    pub part_of_speech: PartOfSpeech,
    pub confidence: f64,
}

pub trait EngineTerminologyAdapter: Send + Sync {
    fn version(&self) -> &'static str;
    fn seeds(&self, segment: &EngineSegmentContext<'_>) -> Vec<EngineTermSeed>;
    fn text_for_morphology(&self, segment: &EngineSegmentContext<'_>) -> String;
}

pub fn adapter_for(engine: &str) -> Option<&'static dyn EngineTerminologyAdapter> {
    match engine {
        "mv_mz" => Some(&crate::engines::mv_mz::terminology::MV_MZ_TERMINOLOGY_ADAPTER),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_engines_do_not_inherit_mv_mz_semantics() {
        assert!(adapter_for("mv_mz").is_some());
        assert!(adapter_for("wolf").is_none());
        assert!(adapter_for("vx_ace").is_none());
    }
}
