//! Show Text messages and descriptions of an MZ game, found in its original
//! (snapshot) JSON by structure — not by segment kind, which projects
//! created before kinds existed store as `unknown`.

use std::sync::LazyLock;

use regex::Regex;
use serde_json::Value;

use crate::engines::filter;

/// A 401/405 line text inside an event list.
static RE_EVENT_LINE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"/list/\d+/parameters/0$").expect("RE_EVENT_LINE must compile"));
static RE_DESCRIPTION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/\d+/description$").expect("RE_DESCRIPTION must compile"));
static RE_PROFILE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^/\d+/profile$").expect("RE_PROFILE must compile"));

/// Whether a segment is laid out by text fitting at export (so a line-width
/// warning on its raw translation does not apply). Dialogue is recognised
/// by kind, or by structure for projects whose segments predate kinds.
pub fn is_fitted_segment(
    file_name: &str,
    json_key: &str,
    segment_kind: &str,
    dialogue: bool,
    descriptions: bool,
) -> bool {
    let event_file = filter::is_map_file(file_name, ".json")
        || file_name == "CommonEvents.json"
        || file_name == "Troops.json";
    if dialogue
        && event_file
        && (segment_kind == "dialogue"
            || (segment_kind == "unknown" && RE_EVENT_LINE.is_match(json_key)))
    {
        return true;
    }
    descriptions
        && match file_name {
            "Items.json" | "Skills.json" | "Weapons.json" | "Armors.json" => {
                RE_DESCRIPTION.is_match(json_key)
            }
            "Actors.json" => RE_PROFILE.is_match(json_key),
            _ => false,
        }
}

/// A Show Text message: the 101 command and its consecutive 401 lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageRef {
    /// Pointer of the event list (`/events/3/pages/0/list`).
    pub list_pointer: String,
    /// Index of the 101 command in the list.
    pub command_index: usize,
    /// `(401 index, original text)`.
    pub lines: Vec<(usize, String)>,
    /// A portrait is shown.
    pub face: bool,
}

impl MessageRef {
    /// Pointer of the 101 command (stable: commands are never inserted).
    pub fn pointer(&self) -> String {
        format!("{}/{}", self.list_pointer, self.command_index)
    }

    /// Pointer of a 401 line's text.
    pub fn line_pointer(&self, index: usize) -> String {
        format!("{}/{index}/parameters/0", self.list_pointer)
    }
}

/// An item/skill/weapon/armor description or an actor profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DescriptionRef {
    pub pointer: String,
    pub text: String,
    /// Items, weapons and armors (shown by item popups such as TinyGetInfoWndMZ).
    pub item_like: bool,
}

/// Event command lists of a map, `CommonEvents.json` or `Troops.json`.
pub fn event_lists<'a>(file_name: &str, json: &'a Value) -> Vec<(String, &'a Vec<Value>)> {
    let mut out = Vec::new();
    let pages = |base: String, holder: &'a Value, out: &mut Vec<(String, &'a Vec<Value>)>| {
        for (page_index, page) in holder
            .get("pages")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            if let Some(list) = page.get("list").and_then(Value::as_array) {
                out.push((format!("{base}/pages/{page_index}/list"), list));
            }
        }
    };
    if filter::is_map_file(file_name, ".json") {
        for (event_index, event) in json
            .get("events")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .enumerate()
        {
            pages(format!("/events/{event_index}"), event, &mut out);
        }
    } else if file_name == "CommonEvents.json" {
        for (index, event) in json.as_array().into_iter().flatten().enumerate() {
            if let Some(list) = event.get("list").and_then(Value::as_array) {
                out.push((format!("/{index}/list"), list));
            }
        }
    } else if file_name == "Troops.json" {
        for (index, troop) in json.as_array().into_iter().flatten().enumerate() {
            pages(format!("/{index}"), troop, &mut out);
        }
    }
    out
}

fn code(command: &Value) -> Option<i64> {
    command.get("code").and_then(Value::as_i64)
}

/// Every Show Text message of a file.
pub fn messages_in(file_name: &str, json: &Value) -> Vec<MessageRef> {
    let mut out = Vec::new();
    for (list_pointer, list) in event_lists(file_name, json) {
        let mut index = 0;
        while index < list.len() {
            if code(&list[index]) != Some(101) {
                index += 1;
                continue;
            }
            let face = list[index]
                .get("parameters")
                .and_then(|p| p.get(0))
                .and_then(Value::as_str)
                .is_some_and(|name| !name.is_empty());
            let command_index = index;
            let mut lines = Vec::new();
            index += 1;
            while index < list.len() && code(&list[index]) == Some(401) {
                let text = list[index]
                    .get("parameters")
                    .and_then(|p| p.get(0))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string();
                lines.push((index, text));
                index += 1;
            }
            if !lines.is_empty() {
                out.push(MessageRef {
                    list_pointer: list_pointer.clone(),
                    command_index,
                    lines,
                    face,
                });
            }
        }
    }
    out
}

/// Descriptions (and actor profiles) of a database file.
pub fn descriptions_in(file_name: &str, json: &Value) -> Vec<DescriptionRef> {
    let (field, item_like) = match file_name {
        "Items.json" | "Weapons.json" | "Armors.json" => ("description", true),
        "Skills.json" => ("description", false),
        "Actors.json" => ("profile", false),
        _ => return Vec::new(),
    };
    json.as_array()
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, entry)| {
            let text = entry.get(field).and_then(Value::as_str)?;
            (!text.is_empty()).then(|| DescriptionRef {
                pointer: format!("/{index}/{field}"),
                text: text.to_string(),
                item_like,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_messages_in_maps_common_events_and_troops() {
        let map = serde_json::json!({ "events": [null, { "pages": [{ "list": [
            { "code": 101, "parameters": ["Actor1", 0, 0, 2, "Megumi"] },
            { "code": 401, "parameters": ["一行目"] },
            { "code": 401, "parameters": ["二行目"] },
            { "code": 102, "parameters": [["はい"]] },
            { "code": 101, "parameters": ["", 0, 0, 2, ""] },
            { "code": 401, "parameters": ["三"] },
            { "code": 0, "parameters": [] }
        ]}]}]});
        let messages = messages_in("Map001.json", &map);
        assert_eq!(messages.len(), 2);
        assert_eq!(messages[0].pointer(), "/events/1/pages/0/list/0");
        assert!(messages[0].face && !messages[1].face);
        assert_eq!(
            messages[0].lines,
            vec![(1, "一行目".into()), (2, "二行目".into())]
        );
        assert_eq!(
            messages[0].line_pointer(2),
            "/events/1/pages/0/list/2/parameters/0"
        );

        let common = serde_json::json!([null, { "list": [
            { "code": 101, "parameters": ["", 0, 0, 2] },
            { "code": 401, "parameters": ["x"] }
        ]}]);
        assert_eq!(
            messages_in("CommonEvents.json", &common)[0].pointer(),
            "/1/list/0"
        );
        let troops = serde_json::json!([null, { "pages": [{ "list": [
            { "code": 101, "parameters": ["", 0, 0, 2] },
            { "code": 401, "parameters": ["y"] }
        ]}]}]);
        assert_eq!(
            messages_in("Troops.json", &troops)[0].pointer(),
            "/1/pages/0/list/0"
        );
    }

    #[test]
    fn recognises_fitted_segments() {
        let line = "/events/1/pages/0/list/3/parameters/0";
        assert!(is_fitted_segment(
            "Map001.json",
            line,
            "dialogue",
            true,
            false
        ));
        assert!(is_fitted_segment(
            "Map001.json",
            line,
            "unknown",
            true,
            false
        ));
        assert!(!is_fitted_segment(
            "Map001.json",
            line,
            "choice",
            true,
            false
        ));
        assert!(!is_fitted_segment(
            "Map001.json",
            line,
            "dialogue",
            false,
            true
        ));
        assert!(is_fitted_segment(
            "Items.json",
            "/4/description",
            "unknown",
            false,
            true
        ));
        assert!(is_fitted_segment(
            "Actors.json",
            "/1/profile",
            "unknown",
            false,
            true
        ));
        assert!(!is_fitted_segment(
            "Actors.json",
            "/1/name",
            "unknown",
            true,
            true
        ));
    }

    #[test]
    fn finds_descriptions_and_profiles() {
        let items = serde_json::json!([null, { "description": "回復" }, { "description": "" }]);
        let found = descriptions_in("Items.json", &items);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].pointer, "/1/description");
        assert!(found[0].item_like);
        let actors = serde_json::json!([null, { "profile": "魔術師" }]);
        assert_eq!(
            descriptions_in("Actors.json", &actors)[0].pointer,
            "/1/profile"
        );
        assert!(descriptions_in("Map001.json", &items).is_empty());
    }
}
