//! RPG Maker MV/MZ — JSON extractor
//!
//! Reads `data/*.json` files from an MV/MZ game and extracts all
//! translatable text segments with their JSON Pointer keys (RFC 6901).
//!
//! JSON Pointer keys allow the injector to write translations back
//! with `serde_json::Value::pointer_mut()`.

use crate::llm::tokenizer::{Engine as TokEngine, Tokenizer};
use serde_json::Value;

/// Semantic kind of a translatable text unit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SegmentKind {
    /// Dialogue text line (event code 401 — Show Text continuation)
    Dialogue,
    /// Speaker name in MZ Show Text header (event code 101, params[4])
    Speaker,
    /// Choice option text (event code 102, params[0][n])
    Choice,
    ActorName,
    ActorNickname,
    ActorProfile,
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
    /// JSON Pointer (RFC 6901) within the source file.
    /// e.g. `"/events/1/pages/0/list/5/parameters/0"`
    pub key: String,
    /// Original source text.
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
// Public extraction functions (one per JSON file type)
// ---------------------------------------------------------------------------

/// Extract from a Map JSON file (`data/MapXXX.json`).
///
/// Handles event codes: 101 (MZ speaker name), 401 (dialogue), 102 (choices).
pub fn extract_map(json: &Value) -> Vec<ExtractedSegment> {
    let mut segments = Vec::new();
    let events = match json.get("events").and_then(Value::as_array) {
        Some(e) => e,
        None => return segments,
    };
    for (ei, event) in events.iter().enumerate() {
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
                &format!("/events/{ei}/pages/{pi}/list"),
                &mut segments,
            );
        }
    }
    segments
}

/// Extract from `data/CommonEvents.json`.
///
/// Structure: `[null, {id, name, list: [...commands...], ...}, ...]`
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
            if !name.is_empty() {
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

/// Extract from `data/Troops.json`.
///
/// Structure: `[null, {id, name, pages: [{list: [...]}], ...}, ...]`
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

/// Extract from `data/Actors.json`.
pub fn extract_actors(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(
        json,
        &[
            ("name", SegmentKind::ActorName),
            ("nickname", SegmentKind::ActorNickname),
            ("profile", SegmentKind::ActorProfile),
        ],
    )
}

/// Extract from `data/Classes.json`.
pub fn extract_classes(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(json, &[("name", SegmentKind::ClassName)])
}

/// Extract from `data/Items.json`.
pub fn extract_items(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(
        json,
        &[
            ("name", SegmentKind::ItemName),
            ("description", SegmentKind::ItemDescription),
        ],
    )
}

/// Extract from `data/Weapons.json` (same structure as Items).
pub fn extract_weapons(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(
        json,
        &[
            ("name", SegmentKind::ItemName),
            ("description", SegmentKind::ItemDescription),
        ],
    )
}

/// Extract from `data/Armors.json` (same structure as Items).
pub fn extract_armors(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(
        json,
        &[
            ("name", SegmentKind::ItemName),
            ("description", SegmentKind::ItemDescription),
        ],
    )
}

/// Extract from `data/Skills.json` — includes message1/message2 (in-battle use).
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
                if !text.trim().is_empty() {
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
                if !text.trim().is_empty() {
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

/// Extract from `data/Enemies.json`.
pub fn extract_enemies(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(json, &[("name", SegmentKind::EnemyName)])
}

/// Extract from `data/States.json` — includes message1–4.
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
            if !text.is_empty() {
                segments.push(ExtractedSegment::new(
                    format!("/{i}/name"),
                    text,
                    SegmentKind::StateName,
                ));
            }
        }
        for msg_field in &["message1", "message2", "message3", "message4"] {
            if let Some(text) = entry.get(msg_field).and_then(Value::as_str) {
                if !text.trim().is_empty() {
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

/// Extract from `data/MapInfos.json` — map names.
pub fn extract_map_infos(json: &Value) -> Vec<ExtractedSegment> {
    extract_simple_array(json, &[("name", SegmentKind::MapName)])
}

/// Extract from `data/System.json` — game title, currency, terms.
pub fn extract_system(json: &Value) -> Vec<ExtractedSegment> {
    let mut segments = Vec::new();

    if let Some(title) = json.get("gameTitle").and_then(Value::as_str) {
        if !title.trim().is_empty() {
            segments.push(ExtractedSegment::new(
                "/gameTitle",
                title,
                SegmentKind::GameTitle,
            ));
        }
    }

    if let Some(currency) = json.get("currencyUnit").and_then(Value::as_str) {
        if !currency.trim().is_empty() {
            segments.push(ExtractedSegment::new(
                "/currencyUnit",
                currency,
                SegmentKind::SystemTerm,
            ));
        }
    }

    // terms > basic (array)
    if let Some(basic) = json.pointer("/terms/basic").and_then(Value::as_array) {
        for (i, term) in basic.iter().enumerate() {
            if let Some(text) = term.as_str() {
                if !text.trim().is_empty() {
                    segments.push(ExtractedSegment::new(
                        format!("/terms/basic/{i}"),
                        text,
                        SegmentKind::SystemTerm,
                    ));
                }
            }
        }
    }

    // terms > commands (array)
    if let Some(commands) = json.pointer("/terms/commands").and_then(Value::as_array) {
        for (i, term) in commands.iter().enumerate() {
            if let Some(text) = term.as_str() {
                if !text.trim().is_empty() {
                    segments.push(ExtractedSegment::new(
                        format!("/terms/commands/{i}"),
                        text,
                        SegmentKind::SystemTerm,
                    ));
                }
            }
        }
    }

    // terms > params (array)
    if let Some(params) = json.pointer("/terms/params").and_then(Value::as_array) {
        for (i, term) in params.iter().enumerate() {
            if let Some(text) = term.as_str() {
                if !text.trim().is_empty() {
                    segments.push(ExtractedSegment::new(
                        format!("/terms/params/{i}"),
                        text,
                        SegmentKind::SystemTerm,
                    ));
                }
            }
        }
    }

    // terms > messages (object with string values)
    if let Some(messages) = json.pointer("/terms/messages").and_then(Value::as_object) {
        let mut msg_keys: Vec<&String> = messages.keys().collect();
        msg_keys.sort(); // deterministic ordering for tests
        for key in msg_keys {
            if let Some(text) = messages[key].as_str() {
                if !text.trim().is_empty() {
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

/// Extract from simple `[null, {field: text, ...}, ...]` arrays.
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
                if !text.trim().is_empty() {
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

/// Returns `true` if `text` contains only RPG Maker escape codes and whitespace —
/// nothing meaningful to translate once placeholders are stripped.
fn is_placeholder_only(text: &str) -> bool {
    let tok = Tokenizer::tokenize(text, TokEngine::MvMz);
    if tok.map.is_empty() {
        return false; // No placeholders at all — whatever text is there is real content
    }
    // Remove every ⟦ph_N⟧ token; check if anything translatable remains
    let bare = tok
        .map
        .keys()
        .fold(tok.text.clone(), |s, k| s.replace(k.as_str(), ""));
    bare.trim().is_empty()
}

/// Extract dialogue and choices from an event command list.
///
/// `list_path` is the JSON Pointer prefix for this list,
/// e.g. `"/events/1/pages/0/list"`.
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
            // Show Text header — in MZ, params[4] = speaker name
            101 => {
                if let Some(name) = params.get(4).and_then(Value::as_str) {
                    if !name.trim().is_empty() && !is_placeholder_only(name) {
                        segments.push(ExtractedSegment::new(
                            format!("{list_path}/{li}/parameters/4"),
                            name,
                            SegmentKind::Speaker,
                        ));
                    }
                }
            }
            // Show Text continuation — one line of dialogue
            401 => {
                if let Some(text) = params.first().and_then(Value::as_str) {
                    if !text.trim().is_empty() && !is_placeholder_only(text) {
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
                            if !text.trim().is_empty() && !is_placeholder_only(text) {
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
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_map_dialogue() {
        let json = json!({
            "events": [
                null,
                {
                    "id": 1,
                    "pages": [{
                        "list": [
                            { "code": 401, "parameters": ["こんにちは！"] },
                            { "code": 401, "parameters": ["元気ですか？"] }
                        ]
                    }]
                }
            ]
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].source, "こんにちは！");
        assert_eq!(segs[0].key, "/events/1/pages/0/list/0/parameters/0");
        assert_eq!(segs[0].kind, SegmentKind::Dialogue);
        assert_eq!(segs[1].source, "元気ですか？");
    }

    #[test]
    fn test_extract_map_mz_speaker() {
        let json = json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [
                        { "code": 101, "parameters": ["", 0, 0, 2, "勇者"] },
                        { "code": 401, "parameters": ["出発します！"] }
                    ]
                }]
            }]
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].source, "勇者");
        assert_eq!(segs[0].kind, SegmentKind::Speaker);
        assert_eq!(segs[1].source, "出発します！");
        assert_eq!(segs[1].kind, SegmentKind::Dialogue);
    }

    #[test]
    fn test_extract_map_mv_no_speaker() {
        // MV format: code 101 has only 4 params (no speaker name)
        let json = json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [
                        { "code": 101, "parameters": ["", 0, 0, 2] },
                        { "code": 401, "parameters": ["MV テキスト"] }
                    ]
                }]
            }]
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].kind, SegmentKind::Dialogue);
    }

    #[test]
    fn test_extract_map_choices() {
        let json = json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [
                        { "code": 102, "parameters": [["はい", "いいえ"], 0, 0, 0, 0] }
                    ]
                }]
            }]
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].source, "はい");
        assert_eq!(segs[0].kind, SegmentKind::Choice);
        assert_eq!(segs[0].key, "/events/1/pages/0/list/0/parameters/0/0");
        assert_eq!(segs[1].source, "いいえ");
        assert_eq!(segs[1].key, "/events/1/pages/0/list/0/parameters/0/1");
    }

    #[test]
    fn test_extract_map_skips_empty_text() {
        let json = json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [
                        { "code": 401, "parameters": [""] },
                        { "code": 401, "parameters": ["テキスト"] }
                    ]
                }]
            }]
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, "テキスト");
    }

    #[test]
    fn test_extract_actors() {
        let json = json!([
            null,
            { "id": 1, "name": "主人公", "nickname": "勇者", "profile": "冒険者の少年。" }
        ]);
        let segs = extract_actors(&json);
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0].source, "主人公");
        assert_eq!(segs[0].kind, SegmentKind::ActorName);
        assert_eq!(segs[0].key, "/1/name");
        assert_eq!(segs[1].source, "勇者");
        assert_eq!(segs[1].kind, SegmentKind::ActorNickname);
        assert_eq!(segs[2].source, "冒険者の少年。");
        assert_eq!(segs[2].kind, SegmentKind::ActorProfile);
    }

    #[test]
    fn test_extract_items() {
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
        // name + description + message1 (message2 empty → skipped)
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[2].source, "%1は炎魔法を唱えた！");
        assert_eq!(segs[2].kind, SegmentKind::SkillMessage);
    }

    #[test]
    fn test_extract_states_with_messages() {
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
        // name + message1 + message2 + message3 (message4 empty → skipped)
        assert_eq!(segs.len(), 4);
        assert_eq!(segs[0].kind, SegmentKind::StateName);
        assert_eq!(segs[1].kind, SegmentKind::StateMessage);
        assert_eq!(segs[1].source, "%1は毒を受けた！");
    }

    #[test]
    fn test_extract_system() {
        let json = json!({
            "gameTitle": "勇者の物語",
            "currencyUnit": "G",
            "terms": {
                "basic": ["最大HP", "最大MP"],
                "commands": ["攻撃", "防御"],
                "params": ["HP", "MP"],
                "messages": {
                    "actorDamage": "%1は%2のダメージを受けた！"
                }
            }
        });
        let segs = extract_system(&json);
        // gameTitle + currencyUnit + 2 basic + 2 commands + 2 params + 1 message = 9
        assert_eq!(segs.len(), 9);
        assert_eq!(segs[0].source, "勇者の物語");
        assert_eq!(segs[0].kind, SegmentKind::GameTitle);
        assert_eq!(segs[1].source, "G");
        assert_eq!(segs[1].kind, SegmentKind::SystemTerm);
    }

    #[test]
    fn test_extract_common_events() {
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
        // event name + dialogue
        assert_eq!(segs.len(), 2);
        assert_eq!(segs[0].source, "自動起動");
        assert_eq!(segs[0].kind, SegmentKind::CommonEventName);
        assert_eq!(segs[1].source, "共通イベントのセリフ");
        assert_eq!(segs[1].kind, SegmentKind::Dialogue);
    }

    #[test]
    fn test_extract_map_infos() {
        let json = json!([null, { "id": 1, "name": "序章の森" }]);
        let segs = extract_map_infos(&json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, "序章の森");
        assert_eq!(segs[0].kind, SegmentKind::MapName);
    }

    #[test]
    fn test_extract_troops_dialogue() {
        let json = json!([
            null,
            {
                "id": 1,
                "name": "スライム×3",
                "pages": [{
                    "list": [
                        { "code": 401, "parameters": ["スライムの鳴き声！"] }
                    ]
                }]
            }
        ]);
        let segs = extract_troops(&json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, "スライムの鳴き声！");
        assert_eq!(segs[0].key, "/1/pages/0/list/0/parameters/0");
    }

    #[test]
    fn test_extract_skips_whitespace_only() {
        // Empty strings AND whitespace-only strings must both be filtered
        let json = json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [
                        { "code": 401, "parameters": [""] },
                        { "code": 401, "parameters": ["   "] },
                        { "code": 401, "parameters": ["\t\n"] },
                        { "code": 401, "parameters": ["有効なテキスト"] }
                    ]
                }]
            }]
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, "有効なテキスト");
    }

    #[test]
    fn test_extract_simple_array_skips_whitespace() {
        // whitespace-only name filtered, empty profile filtered, valid nickname kept
        let json = json!([
            null,
            { "id": 1, "name": "  ", "nickname": "勇者", "profile": "" }
        ]);
        let segs = extract_actors(&json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, "勇者");
        assert_eq!(segs[0].kind, SegmentKind::ActorNickname);
    }

    // --- Placeholder-only filter tests ---

    #[test]
    fn test_extract_skips_placeholder_only_segment() {
        // "\n[1]" (lowercase — real community-plugin variant) → skipped
        // "\N[4]" (uppercase — official spec) → also skipped
        let json = json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [
                        { "code": 401, "parameters": [r"\n[1]"] },
                        { "code": 401, "parameters": [r"\N[4]"] },
                        { "code": 401, "parameters": ["反応なし…"] }
                    ]
                }]
            }]
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, "反応なし…");
    }

    #[test]
    fn test_extract_keeps_placeholder_with_text() {
        // "\n[1] 反応なし…" (lowercase) — real content after token → kept
        let json = json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [
                        { "code": 401, "parameters": [r"\n[1] 反応なし…"] }
                    ]
                }]
            }]
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, r"\n[1] 反応なし…");
    }

    #[test]
    fn test_extract_skips_multiple_placeholders_no_text() {
        // "\c[2]\n[4]" (lowercase) — multiple codes, zero text → skipped
        let json = json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [
                        { "code": 401, "parameters": [r"\c[2]\n[4]"] },
                        { "code": 401, "parameters": ["捕えた！"] }
                    ]
                }]
            }]
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, "捕えた！");
    }

    #[test]
    fn test_extract_keeps_text_with_intercalated_placeholder() {
        // "捕えた！\V[5]水深１５m" tokenizes to "捕えた！⟦ph_0⟧水深１５m" → real text → kept
        let json = json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [
                        { "code": 401, "parameters": [r"捕えた！\V[5]水深１５m"] }
                    ]
                }]
            }]
        });
        let segs = extract_map(&json);
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].source, r"捕えた！\V[5]水深１５m");
    }
}
