use crate::core::terminology::types::PartOfSpeech;
use crate::engines::terminology::{EngineSegmentContext, EngineTermSeed, EngineTerminologyAdapter};
use crate::llm::tokenizer::{Engine as TokenizerEngine, Tokenizer};

pub static MV_MZ_TERMINOLOGY_ADAPTER: MvMzTerminologyAdapter = MvMzTerminologyAdapter;

pub struct MvMzTerminologyAdapter;

impl EngineTerminologyAdapter for MvMzTerminologyAdapter {
    fn version(&self) -> &'static str {
        "mv_mz-terminology-v1"
    }

    fn seeds(&self, segment: &EngineSegmentContext<'_>) -> Vec<EngineTermSeed> {
        let Some((semantic_type, part_of_speech)) = classify(segment) else {
            return Vec::new();
        };
        let source_text = self.text_for_morphology(segment).trim().to_string();
        if source_text.is_empty() {
            return Vec::new();
        }
        vec![EngineTermSeed {
            source_text,
            semantic_type: semantic_type.to_string(),
            part_of_speech,
            confidence: 1.0,
        }]
    }

    fn text_for_morphology(&self, segment: &EngineSegmentContext<'_>) -> String {
        let tokenizer_engine = if segment.segment_kind == "system_term" {
            TokenizerEngine::MzOnly
        } else {
            TokenizerEngine::MvMz
        };
        let tokenized = Tokenizer::tokenize(segment.source_text, tokenizer_engine);
        tokenized
            .map
            .keys()
            .fold(tokenized.text, |safe, token| safe.replace(token, " "))
    }
}

fn classify(segment: &EngineSegmentContext<'_>) -> Option<(&'static str, PartOfSpeech)> {
    let proper_noun = PartOfSpeech::ProperNoun;
    let noun = PartOfSpeech::Noun;
    match segment.segment_kind {
        "actor_name" | "actor_nickname" => Some(("character", proper_noun)),
        "speaker" => Some(("speaker", proper_noun)),
        "class_name" => Some(("class", noun)),
        "item_name" => Some((item_semantic_type(segment), noun)),
        "skill_name" => Some(("skill", noun)),
        "enemy_name" => Some(("enemy", noun)),
        "state_name" => Some(("state", noun)),
        "map_name" => Some(("place", proper_noun)),
        "common_event_name" => Some(("event", noun)),
        "system_term" => Some(("system", noun)),
        "game_title" => Some(("title", proper_noun)),
        _ => None,
    }
}

fn item_semantic_type(segment: &EngineSegmentContext<'_>) -> &'static str {
    let file_name = segment.file_name.to_ascii_lowercase();
    let file_type = segment.file_type.to_ascii_lowercase();
    if file_name == "weapons.json" || file_type == "weapons" {
        "weapon"
    } else if file_name == "armors.json" || file_type == "armors" {
        "armor"
    } else {
        "item"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn context<'a>(kind: &'a str, file_name: &'a str, source: &'a str) -> EngineSegmentContext<'a> {
        EngineSegmentContext {
            source_text: source,
            segment_kind: kind,
            file_name,
            file_type: "",
            json_key: "/1/name",
            speaker: None,
            context_json: None,
        }
    }

    #[test]
    fn stable_segment_kinds_map_to_engine_owned_semantics() {
        for (kind, file, semantic_type, part_of_speech) in [
            (
                "actor_name",
                "Actors.json",
                "character",
                PartOfSpeech::ProperNoun,
            ),
            (
                "actor_nickname",
                "Actors.json",
                "character",
                PartOfSpeech::ProperNoun,
            ),
            (
                "speaker",
                "Map001.json",
                "speaker",
                PartOfSpeech::ProperNoun,
            ),
            ("class_name", "Classes.json", "class", PartOfSpeech::Noun),
            ("item_name", "Items.json", "item", PartOfSpeech::Noun),
            ("item_name", "Weapons.json", "weapon", PartOfSpeech::Noun),
            ("item_name", "Armors.json", "armor", PartOfSpeech::Noun),
            ("skill_name", "Skills.json", "skill", PartOfSpeech::Noun),
            ("enemy_name", "Enemies.json", "enemy", PartOfSpeech::Noun),
            ("state_name", "States.json", "state", PartOfSpeech::Noun),
            (
                "map_name",
                "MapInfos.json",
                "place",
                PartOfSpeech::ProperNoun,
            ),
            (
                "common_event_name",
                "CommonEvents.json",
                "event",
                PartOfSpeech::Noun,
            ),
            ("system_term", "System.json", "system", PartOfSpeech::Noun),
            (
                "game_title",
                "System.json",
                "title",
                PartOfSpeech::ProperNoun,
            ),
        ] {
            let seeds = MV_MZ_TERMINOLOGY_ADAPTER.seeds(&context(kind, file, "星の勇者"));
            assert_eq!(seeds.len(), 1, "missing seed for {kind}/{file}");
            assert_eq!(seeds[0].semantic_type, semantic_type);
            assert_eq!(seeds[0].part_of_speech, part_of_speech);
            assert_eq!(seeds[0].source_text, "星の勇者");
        }
    }

    #[test]
    fn narrative_fields_are_morphology_only() {
        for kind in [
            "dialogue",
            "scrolling_text",
            "choice",
            "actor_profile",
            "item_description",
            "skill_description",
            "skill_message",
            "state_message",
            "plugin_text",
        ] {
            assert!(MV_MZ_TERMINOLOGY_ADAPTER
                .seeds(&context(kind, "Map001.json", "勇者が走る"))
                .is_empty());
        }
    }

    #[test]
    fn morphology_text_reuses_placeholder_tokenizer() {
        let segment = context(
            "dialogue",
            "Map001.json",
            r"\C[3]勇者\C[0]は\V[1]ゴールドを持つ。",
        );
        let safe = MV_MZ_TERMINOLOGY_ADAPTER.text_for_morphology(&segment);
        assert!(safe.contains("勇者"));
        assert!(safe.contains("ゴールド"));
        assert!(!safe.contains(r"\C[3]"));
        assert!(!safe.contains(r"\V[1]"));
        assert!(!safe.contains("ph_"));
    }

    #[test]
    fn mv_and_mz_fixtures_share_the_same_stable_mapping() {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        for relative in [
            "tests/fixtures/mv_mz/mv/www/data/Actors.json",
            "tests/fixtures/mv_mz/mz/data/Actors.json",
        ] {
            let actors: serde_json::Value = serde_json::from_str(
                &std::fs::read_to_string(manifest_dir.join(relative)).unwrap(),
            )
            .unwrap();
            let name = actors
                .pointer("/1/name")
                .and_then(|value| value.as_str())
                .unwrap();
            let seeds =
                MV_MZ_TERMINOLOGY_ADAPTER.seeds(&context("actor_name", "Actors.json", name));
            assert_eq!(seeds[0].semantic_type, "character");
            assert_eq!(seeds[0].part_of_speech, PartOfSpeech::ProperNoun);
        }
    }
}
