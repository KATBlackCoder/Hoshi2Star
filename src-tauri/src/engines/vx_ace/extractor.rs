//! RPG Maker VX Ace — .rvdata2 extractor
//!
//! Reads `.rvdata2` files from a VX Ace game (non-packaged, Temps 1) and
//! extracts all translatable text segments with their JSON Pointer keys.
//!
//! Uses `marshal-rs` to deserialise Ruby Marshal format into `serde_json::Value`
//! via the conversion `marshal_rs::Value → serde_json::Value`.
//!
//! ## Key differences vs MV/MZ extractor
//! - Binary input (`&[u8]`) not string — Shift-JIS handled by `load_utf8`
//! - `events` in Map*.rvdata2 is a Ruby Hash (JSON Object, string keys) not Array
//! - `MapInfos.rvdata2` is a Ruby Hash with Integer keys (serialised as string "1", "2", ...)
//! - `System.rvdata2` uses snake_case fields (`game_title`, `currency_unit`)
//! - Code 101 in VX Ace has NO speaker name (unlike MZ params[4])
//! - Placeholder filtering via `engines::filter` — VX Ace shares the MvMz tokenizer

use crate::{engines::filter, llm::tokenizer::Engine as TokEngine};
use marshal_rs::load_utf8;
use serde_json::Value;

/// Semantic kind of a translatable text unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentKind {
    Dialogue,
    Choice,
    ActorName,
    ActorNickname,
    ActorDescription,
    ClassName,
    ItemName,
    ItemDescription,
    SkillName,
    SkillDescription,
    /// In-battle skill use message (message1 / message2)
    SkillMessage,
    EnemyName,
    StateName,
    /// State apply/persist/remove message (message1-4)
    StateMessage,
    MapName,
    CommonEventName,
    /// Currency unit, battle commands, param names, terms messages
    SystemTerm,
    GameTitle,
}

/// A single extracted translatable text unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractedSegment {
    /// JSON Pointer (RFC 6901) within the deserialised Value.
    pub key: String,
    /// Original source text (UTF-8, converted from Shift-JIS by load_utf8).
    pub source: String,
    /// Semantic kind for CAT context.
    pub kind: SegmentKind,
}

impl ExtractedSegment {
    fn new(key: impl Into<String>, source: impl Into<String>, kind: SegmentKind) -> Self {
        Self {
            key: key.into(),
            source: source.into(),
            kind,
        }
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Extract all translatable segments from a raw `.rvdata2` file.
///
/// `file_name` is used to dispatch to the correct extraction function.
/// `bytes` is the raw binary content of the `.rvdata2` file.
///
/// Returns an empty Vec if the file type is unrecognised or parsing fails.
pub fn extract_from_bytes(file_name: &str, bytes: &[u8]) -> Vec<ExtractedSegment> {
    let json: Value = match load_utf8(bytes, None) {
        Ok(mv) => mv.into(),
        Err(_) => return Vec::new(),
    };
    dispatch_extract(file_name, &json)
}

// ---------------------------------------------------------------------------
// Public extraction functions (one per file type)
// ---------------------------------------------------------------------------

/// Extract from `Data/Actors.rvdata2`.
///
/// Structure: `[null, {name, nickname, description, ...}, ...]`
pub fn extract_actors(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(
        json,
        &[
            ("name", SegmentKind::ActorName),
            ("nickname", SegmentKind::ActorNickname),
            ("description", SegmentKind::ActorDescription),
        ],
    )
}

/// Extract from `Data/Classes.rvdata2`.
pub fn extract_classes(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(json, &[("name", SegmentKind::ClassName)])
}

/// Extract from `Data/Items.rvdata2`.
pub fn extract_items(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(
        json,
        &[
            ("name", SegmentKind::ItemName),
            ("description", SegmentKind::ItemDescription),
        ],
    )
}

/// Extract from `Data/Weapons.rvdata2` (same structure as Items).
pub fn extract_weapons(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(
        json,
        &[
            ("name", SegmentKind::ItemName),
            ("description", SegmentKind::ItemDescription),
        ],
    )
}

/// Extract from `Data/Armors.rvdata2` (same structure as Items).
pub fn extract_armors(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(
        json,
        &[
            ("name", SegmentKind::ItemName),
            ("description", SegmentKind::ItemDescription),
        ],
    )
}

/// Extract from `Data/Skills.rvdata2` — includes message1/message2.
pub fn extract_skills(json: &Value) -> Vec<ExtractedSegment> {
    let mut segments = Vec::new();
    let arr = match json.as_array() {
        Some(a) => a,
        None => return segments,
    };
    for (i, entry) in arr.iter().enumerate() {
        if entry.is_null() {
            continue;
        }
        for (field, kind) in &[
            ("name", SegmentKind::SkillName),
            ("description", SegmentKind::SkillDescription),
        ] {
            if let Some(text) = entry.get(field).and_then(Value::as_str) {
                if filter::needs_translation(text, TokEngine::MvMz) {
                    segments.push(ExtractedSegment::new(
                        format!("/{i}/{field}"),
                        text,
                        kind.clone(),
                    ));
                }
            }
        }
        for msg_field in &["message1", "message2"] {
            if let Some(text) = entry.get(msg_field).and_then(Value::as_str) {
                if filter::needs_translation(text, TokEngine::MvMz) {
                    segments.push(ExtractedSegment::new(
                        format!("/{i}/{msg_field}"),
                        text,
                        SegmentKind::SkillMessage,
                    ));
                }
            }
        }
    }
    segments
}

/// Extract from `Data/States.rvdata2` — includes message1–4.
pub fn extract_states(json: &Value) -> Vec<ExtractedSegment> {
    let mut segments = Vec::new();
    let arr = match json.as_array() {
        Some(a) => a,
        None => return segments,
    };
    for (i, entry) in arr.iter().enumerate() {
        if entry.is_null() {
            continue;
        }
        if let Some(text) = entry.get("name").and_then(Value::as_str) {
            if filter::needs_translation(text, TokEngine::MvMz) {
                segments.push(ExtractedSegment::new(
                    format!("/{i}/name"),
                    text,
                    SegmentKind::StateName,
                ));
            }
        }
        for msg_field in &["message1", "message2", "message3", "message4"] {
            if let Some(text) = entry.get(msg_field).and_then(Value::as_str) {
                if filter::needs_translation(text, TokEngine::MvMz) {
                    segments.push(ExtractedSegment::new(
                        format!("/{i}/{msg_field}"),
                        text,
                        SegmentKind::StateMessage,
                    ));
                }
            }
        }
    }
    segments
}

/// Extract from `Data/Enemies.rvdata2`.
pub fn extract_enemies(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(json, &[("name", SegmentKind::EnemyName)])
}

/// Extract from `Data/Troops.rvdata2`.
///
/// Structure: `[null, {name, pages: [{list: [...commands...]}], ...}, ...]`
pub fn extract_troops(json: &Value) -> Vec<ExtractedSegment> {
    let mut segments = Vec::new();
    let troops = match json.as_array() {
        Some(t) => t,
        None => return segments,
    };
    for (ti, troop) in troops.iter().enumerate() {
        if troop.is_null() {
            continue;
        }
        let pages = match troop.get("pages").and_then(Value::as_array) {
            Some(p) => p,
            None => continue,
        };
        for (pi, page) in pages.iter().enumerate() {
            let list = match page.get("list").and_then(Value::as_array) {
                Some(l) => l,
                None => continue,
            };
            extract_event_list(list, &format!("/{ti}/pages/{pi}/list"), &mut segments);
        }
    }
    segments
}

/// Extract from `Data/MapInfos.rvdata2`.
///
/// VX Ace structure: Ruby Hash `{Integer → RPG::MapInfo}` serialised as JSON Object
/// with string keys `"1"`, `"2"`, etc. Keys are sorted numerically for determinism.
pub fn extract_map_infos(json: &Value) -> Vec<ExtractedSegment> {
    let mut segments = Vec::new();
    let obj = match json.as_object() {
        Some(o) => o,
        None => return segments,
    };
    let mut keys: Vec<&String> = obj.keys().collect();
    keys.sort_by_key(|k| k.parse::<u32>().unwrap_or(u32::MAX));
    for key in keys {
        if let Some(name) = obj[key].get("name").and_then(Value::as_str) {
            if filter::needs_translation(name, TokEngine::MvMz) {
                segments.push(ExtractedSegment::new(
                    format!("/{key}/name"),
                    name,
                    SegmentKind::MapName,
                ));
            }
        }
    }
    segments
}

/// Extract from `Data/CommonEvents.rvdata2`.
///
/// Structure: `[null, {name, list: [...commands...], ...}, ...]`
pub fn extract_common_events(json: &Value) -> Vec<ExtractedSegment> {
    let mut segments = Vec::new();
    let events = match json.as_array() {
        Some(e) => e,
        None => return segments,
    };
    for (ei, event) in events.iter().enumerate() {
        if event.is_null() {
            continue;
        }
        if let Some(name) = event.get("name").and_then(Value::as_str) {
            if filter::needs_translation(name, TokEngine::MvMz) {
                segments.push(ExtractedSegment::new(
                    format!("/{ei}/name"),
                    name,
                    SegmentKind::CommonEventName,
                ));
            }
        }
        let list = match event.get("list").and_then(Value::as_array) {
            Some(l) => l,
            None => continue,
        };
        extract_event_list(list, &format!("/{ei}/list"), &mut segments);
    }
    segments
}

/// Extract from `Data/Map*.rvdata2`.
///
/// VX Ace structure: Object with `events` as Ruby Hash `{Integer → RPG::Event}`.
/// Unlike MV/MZ where `events` is an Array, here it is a JSON Object with string keys.
/// Event command codes: 401 (dialogue), 102 (choices). Code 101 has NO speaker name.
pub fn extract_map(json: &Value) -> Vec<ExtractedSegment> {
    let mut segments = Vec::new();
    let events_obj = match json.get("events").and_then(Value::as_object) {
        Some(e) => e,
        None => return segments,
    };
    let mut keys: Vec<&String> = events_obj.keys().collect();
    keys.sort_by_key(|k| k.parse::<u32>().unwrap_or(u32::MAX));
    for key in keys {
        let event = &events_obj[key];
        let pages = match event.get("pages").and_then(Value::as_array) {
            Some(p) => p,
            None => continue,
        };
        for (pi, page) in pages.iter().enumerate() {
            let list = match page.get("list").and_then(Value::as_array) {
                Some(l) => l,
                None => continue,
            };
            extract_event_list(
                list,
                &format!("/events/{key}/pages/{pi}/list"),
                &mut segments,
            );
        }
    }
    segments
}

/// Extract from `Data/System.rvdata2`.
///
/// VX Ace uses snake_case Ruby field names: `game_title`, `currency_unit`.
/// `terms` structure mirrors MV/MZ: basic[], commands[], params[], messages{}.
pub fn extract_system(json: &Value) -> Vec<ExtractedSegment> {
    let mut segments = Vec::new();

    if let Some(title) = json.get("game_title").and_then(Value::as_str) {
        if filter::needs_translation(title, TokEngine::MvMz) {
            segments.push(ExtractedSegment::new(
                "/game_title",
                title,
                SegmentKind::GameTitle,
            ));
        }
    }

    if let Some(currency) = json.get("currency_unit").and_then(Value::as_str) {
        if filter::needs_translation(currency, TokEngine::MvMz) {
            segments.push(ExtractedSegment::new(
                "/currency_unit",
                currency,
                SegmentKind::SystemTerm,
            ));
        }
    }

    if let Some(basic) = json.pointer("/terms/basic").and_then(Value::as_array) {
        for (i, term) in basic.iter().enumerate() {
            if let Some(text) = term.as_str() {
                if filter::needs_translation(text, TokEngine::MvMz) {
                    segments.push(ExtractedSegment::new(
                        format!("/terms/basic/{i}"),
                        text,
                        SegmentKind::SystemTerm,
                    ));
                }
            }
        }
    }

    if let Some(commands) = json.pointer("/terms/commands").and_then(Value::as_array) {
        for (i, term) in commands.iter().enumerate() {
            if let Some(text) = term.as_str() {
                if filter::needs_translation(text, TokEngine::MvMz) {
                    segments.push(ExtractedSegment::new(
                        format!("/terms/commands/{i}"),
                        text,
                        SegmentKind::SystemTerm,
                    ));
                }
            }
        }
    }

    if let Some(params) = json.pointer("/terms/params").and_then(Value::as_array) {
        for (i, term) in params.iter().enumerate() {
            if let Some(text) = term.as_str() {
                if filter::needs_translation(text, TokEngine::MvMz) {
                    segments.push(ExtractedSegment::new(
                        format!("/terms/params/{i}"),
                        text,
                        SegmentKind::SystemTerm,
                    ));
                }
            }
        }
    }

    if let Some(messages) = json.pointer("/terms/messages").and_then(Value::as_object) {
        let mut msg_keys: Vec<&String> = messages.keys().collect();
        msg_keys.sort();
        for key in msg_keys {
            if let Some(text) = messages[key].as_str() {
                if filter::needs_translation(text, TokEngine::MvMz) {
                    segments.push(ExtractedSegment::new(
                        format!("/terms/messages/{key}"),
                        text,
                        SegmentKind::SystemTerm,
                    ));
                }
            }
        }
    }

    segments
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Dispatch to the correct extract function based on VX Ace file name.
fn dispatch_extract(file_name: &str, json: &Value) -> Vec<ExtractedSegment> {
    if file_name.starts_with("Map")
        && file_name != "MapInfos.rvdata2"
        && file_name
            .trim_start_matches("Map")
            .trim_end_matches(".rvdata2")
            .parse::<u32>()
            .is_ok()
    {
        return extract_map(json);
    }

    match file_name {
        "Actors.rvdata2" => extract_actors(json),
        "Armors.rvdata2" => extract_armors(json),
        "Classes.rvdata2" => extract_classes(json),
        "CommonEvents.rvdata2" => extract_common_events(json),
        "Enemies.rvdata2" => extract_enemies(json),
        "Items.rvdata2" => extract_items(json),
        "MapInfos.rvdata2" => extract_map_infos(json),
        "Skills.rvdata2" => extract_skills(json),
        "States.rvdata2" => extract_states(json),
        "System.rvdata2" => extract_system(json),
        "Troops.rvdata2" => extract_troops(json),
        "Weapons.rvdata2" => extract_weapons(json),
        _ => Vec::new(),
    }
}

/// Extract from a simple `[null, {field: text, ...}, ...]` array.
fn extract_simple_array(json: &Value, fields: &[(&str, SegmentKind)]) -> Vec<ExtractedSegment> {
    let mut segments = Vec::new();
    let arr = match json.as_array() {
        Some(a) => a,
        None => return segments,
    };
    for (i, entry) in arr.iter().enumerate() {
        if entry.is_null() {
            continue;
        }
        for (field, kind) in fields {
            if let Some(text) = entry.get(field).and_then(Value::as_str) {
                if filter::needs_translation(text, TokEngine::MvMz) {
                    segments.push(ExtractedSegment::new(
                        format!("/{i}/{field}"),
                        text,
                        kind.clone(),
                    ));
                }
            }
        }
    }
    segments
}

/// Extract dialogue and choices from an event command list.
///
/// VX Ace event codes:
/// - 401: Show Text continuation (dialogue line) — same as MV/MZ
/// - 102: Show Choices — same as MV/MZ
/// - 101: Show Text header — NO speaker name in VX Ace (unlike MZ params[4])
fn extract_event_list(list: &[Value], list_path: &str, segments: &mut Vec<ExtractedSegment>) {
    for (li, cmd) in list.iter().enumerate() {
        let code = match cmd.get("code").and_then(Value::as_i64) {
            Some(c) => c,
            None => continue,
        };
        let params = match cmd.get("parameters").and_then(Value::as_array) {
            Some(p) => p,
            None => continue,
        };

        match code {
            // Show Text continuation — one line of dialogue
            401 => {
                if let Some(text) = params.first().and_then(Value::as_str) {
                    if filter::needs_translation(text, TokEngine::MvMz) {
                        segments.push(ExtractedSegment::new(
                            format!("{list_path}/{li}/parameters/0"),
                            text,
                            SegmentKind::Dialogue,
                        ));
                    }
                }
            }
            // Show Choices — params[0] is an array of choice strings
            102 => {
                if let Some(choices) = params.first().and_then(Value::as_array) {
                    for (ci, choice) in choices.iter().enumerate() {
                        if let Some(text) = choice.as_str() {
                            if filter::needs_translation(text, TokEngine::MvMz) {
                                segments.push(ExtractedSegment::new(
                                    format!("{list_path}/{li}/parameters/0/{ci}"),
                                    text,
                                    SegmentKind::Choice,
                                ));
                            }
                        }
                    }
                }
            }
            // 101 = Show Text header — no speaker name in VX Ace, skip
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Tests — Step 5
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // --- Actors ---

    #[test]
    fn test_extract_actors_basic() {
        let json = json!([
            null,
            { "id": 1, "name": "主人公", "nickname": "勇者", "description": "冒険者の少年。" },
            { "id": 2, "name": "仲間", "nickname": "魔法使い", "description": "魔法が得意。" }
        ]);
        let segs = extract_actors(&json);
        assert_eq!(segs.len(), 6); // 3 fields × 2 actors
        assert_eq!(segs[0].source, "主人公");
        assert_eq!(segs[0].key, "/1/name");
        assert_eq!(segs[0].kind, SegmentKind::ActorName);
        assert_eq!(segs[1].source, "勇者");
        assert_eq!(segs[1].kind, SegmentKind::ActorNickname);
        assert_eq!(segs[2].source, "冒険者の少年。");
        assert_eq!(segs[2].kind, SegmentKind::ActorDescription);
        assert_eq!(segs[3].source, "仲間");
        assert_eq!(segs[3].key, "/2/name");
    }

    #[test]
    fn test_extract_actors_skips_null() {
        let json = json!([null, { "id": 1, "name": "勇者", "nickname": "", "description": "" }]);
        let segs = extract_actors(&json);
        assert_eq!(segs.len(), 1); // only non-empty name
        assert_eq!(segs[0].source, "勇者");
    }

    // --- Items ---

    #[test]
    fn test_extract_items_name_and_description() {
        let json = json!([
            null,
            { "id": 1, "name": "ポーション", "description": "HPを50回復する。" }
        ]);
        let segs = extract_items(&json);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].source, "ポーション");
        assert_eq!(segs[0].kind, SegmentKind::ItemName);
        assert_eq!(segs[1].source, "HPを50回復する。");
        assert_eq!(segs[1].kind, SegmentKind::ItemDescription);
    }

    // --- Skills ---

    #[test]
    fn test_extract_skills_with_messages() {
        let json = json!([
            null,
            {
                "id": 1,
                "name": "ファイア",
                "description": "炎の魔法。",
                "message1": "%1は炎魔法を唱えた！",
                "message2": ""
            }
        ]);
        let segs = extract_skills(&json);
        // name + description + message1 (message2 vide → ignoré)
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[2].source, "%1は炎魔法を唱えた！");
        assert_eq!(segs[2].kind, SegmentKind::SkillMessage);
    }

    // --- States ---

    #[test]
    fn test_extract_states_four_messages() {
        let json = json!([
            null,
            {
                "id": 1,
                "name": "毒",
                "message1": "%1は毒を受けた！",
                "message2": "%1は毒にかかっている。",
                "message3": "%1は毒が治った。",
                "message4": ""
            }
        ]);
        let segs = extract_states(&json);
        // name + message1 + message2 + message3 (message4 vide → ignoré)
        assert_eq!(segs.len(), 4);
        assert_eq!(segs[0].kind, SegmentKind::StateName);
        assert_eq!(segs[1].kind, SegmentKind::StateMessage);
        assert_eq!(segs[1].source, "%1は毒を受けた！");
    }

    // --- MapInfos (Hash avec clés string) ---

    #[test]
    fn test_extract_map_infos_hash_keys() {
        let json = json!({
            "1": { "name": "序章の森", "parent_id": 0, "order": 1 },
            "2": { "name": "村", "parent_id": 0, "order": 2 }
        });
        let segs = extract_map_infos(&json);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].source, "序章の森");
        assert_eq!(segs[0].key, "/1/name");
        assert_eq!(segs[0].kind, SegmentKind::MapName);
        assert_eq!(segs[1].source, "村");
        assert_eq!(segs[1].key, "/2/name");
    }

    // --- CommonEvents ---

    #[test]
    fn test_extract_common_events_name_and_dialogue() {
        let json = json!([
            null,
            {
                "id": 1,
                "name": "自動起動",
                "list": [
                    { "code": 401, "parameters": ["共通イベントのセリフ"] }
                ]
            }
        ]);
        let segs = extract_common_events(&json);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].source, "自動起動");
        assert_eq!(segs[0].kind, SegmentKind::CommonEventName);
        assert_eq!(segs[1].source, "共通イベントのセリフ");
        assert_eq!(segs[1].kind, SegmentKind::Dialogue);
    }

    // --- Map (events = Hash avec clé string) ---

    #[test]
    fn test_extract_map_dialogue_code_401() {
        let json = json!({
            "events": {
                "1": {
                    "id": 1,
                    "pages": [{
                        "list": [
                            { "code": 401, "parameters": ["こんにちは！"] },
                            { "code": 401, "parameters": ["元気ですか？"] }
                        ]
                    }]
                }
            }
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].source, "こんにちは！");
        assert_eq!(segs[0].key, "/events/1/pages/0/list/0/parameters/0");
        assert_eq!(segs[0].kind, SegmentKind::Dialogue);
        assert_eq!(segs[1].source, "元気ですか？");
    }

    #[test]
    fn test_extract_map_choices_code_102() {
        let json = json!({
            "events": {
                "1": {
                    "id": 1,
                    "pages": [{
                        "list": [
                            { "code": 102, "parameters": [["はい", "いいえ"], 0] }
                        ]
                    }]
                }
            }
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].source, "はい");
        assert_eq!(segs[0].key, "/events/1/pages/0/list/0/parameters/0/0");
        assert_eq!(segs[0].kind, SegmentKind::Choice);
        assert_eq!(segs[1].source, "いいえ");
        assert_eq!(segs[1].key, "/events/1/pages/0/list/0/parameters/0/1");
    }

    #[test]
    fn test_extract_map_no_speaker_in_code_101() {
        // VX Ace: code 101 = Show Text header, no speaker name (unlike MZ params[4])
        let json = json!({
            "events": {
                "1": {
                    "id": 1,
                    "pages": [{
                        "list": [
                            { "code": 101, "parameters": ["face_file", 0, 0, 2] },
                            { "code": 401, "parameters": ["VXAceのセリフ"] }
                        ]
                    }]
                }
            }
        });
        let segs = extract_map(&json);
        // Code 101 must not produce a segment — only code 401
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, "VXAceのセリフ");
        assert_eq!(segs[0].kind, SegmentKind::Dialogue);
    }

    // --- System (snake_case) ---

    #[test]
    fn test_extract_system_game_title_snake_case() {
        let json = json!({
            "game_title": "勇者の物語",
            "currency_unit": "G",
            "terms": {
                "basic": ["最大HP", "最大MP"],
                "commands": ["攻撃", "防御"],
                "params": ["HP", "MP"],
                "messages": {
                    "actor_damage": "%1は%2のダメージを受けた！"
                }
            }
        });
        let segs = extract_system(&json);
        // game_title + currency_unit + 2 basic + 2 commands + 2 params + 1 message = 9
        assert_eq!(segs.len(), 9);
        assert_eq!(segs[0].source, "勇者の物語");
        assert_eq!(segs[0].key, "/game_title");
        assert_eq!(segs[0].kind, SegmentKind::GameTitle);
        assert_eq!(segs[1].source, "G");
        assert_eq!(segs[1].kind, SegmentKind::SystemTerm);
    }

    // --- Empty / whitespace filters ---

    #[test]
    fn test_extract_skips_empty_strings() {
        let json = json!([
            null,
            { "id": 1, "name": "", "description": "有効な説明" }
        ]);
        let segs = extract_items(&json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, "有効な説明");
    }

    #[test]
    fn test_extract_skips_whitespace_only() {
        let json = json!({
            "events": {
                "1": {
                    "id": 1,
                    "pages": [{
                        "list": [
                            { "code": 401, "parameters": [""] },
                            { "code": 401, "parameters": ["   "] },
                            { "code": 401, "parameters": ["有効なテキスト"] }
                        ]
                    }]
                }
            }
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, "有効なテキスト");
    }
}
