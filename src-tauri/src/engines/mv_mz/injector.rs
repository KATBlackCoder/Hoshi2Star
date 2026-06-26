//! RPG Maker MV/MZ — JSON injector
//!
//! Writes translated text back into a parsed JSON value using the same
//! JSON Pointer keys produced by `extractor`. Uses `serde_json::Value::pointer_mut`.

use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum InjectorError {
    #[error("key not found in JSON: {0}")]
    KeyNotFound(String),
    #[error("target at key is not a string: {0}")]
    NotAString(String),
}

/// Inject translations into raw JSON bytes and return the result in memory.
/// No disk I/O — used by the zip export path.
pub fn inject_to_bytes(raw_json: &str, translations: &[(&str, &str)]) -> Result<Vec<u8>, String> {
    let mut json: Value = serde_json::from_str(raw_json).map_err(|e| e.to_string())?;
    inject(&mut json, translations).map_err(|e| e.to_string())?;
    serde_json::to_string(&json)
        .map(|s| s.into_bytes())
        .map_err(|e| e.to_string())
}

/// Inject translated segments back into a parsed JSON value.
///
/// `translations` is a slice of `(json_pointer_key, translated_text)` pairs.
/// Returns an error on the first key that is missing or does not point to a string.
pub fn inject(json: &mut Value, translations: &[(&str, &str)]) -> Result<(), InjectorError> {
    for (key, text) in translations {
        match json.pointer_mut(key) {
            Some(target) if target.is_string() => {
                *target = Value::String(text.to_string());
            }
            Some(_) => return Err(InjectorError::NotAString(key.to_string())),
            None => return Err(InjectorError::KeyNotFound(key.to_string())),
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::mv_mz::extractor::{extract_items, extract_map};
    use serde_json::json;

    #[test]
    fn test_inject_dialogue() {
        let mut json = json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [
                        { "code": 401, "parameters": ["こんにちは！"] }
                    ]
                }]
            }]
        });

        inject(
            &mut json,
            &[("/events/1/pages/0/list/0/parameters/0", "Hello!")],
        )
        .unwrap();

        let text = json
            .pointer("/events/1/pages/0/list/0/parameters/0")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(text, "Hello!");
    }

    #[test]
    fn test_inject_choices() {
        let mut json = json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [
                        { "code": 102, "parameters": [["はい", "いいえ"], 0, 0, 0, 0] }
                    ]
                }]
            }]
        });

        inject(
            &mut json,
            &[
                ("/events/1/pages/0/list/0/parameters/0/0", "Yes"),
                ("/events/1/pages/0/list/0/parameters/0/1", "No"),
            ],
        )
        .unwrap();

        let yes = json
            .pointer("/events/1/pages/0/list/0/parameters/0/0")
            .and_then(|v| v.as_str())
            .unwrap();
        assert_eq!(yes, "Yes");
    }

    #[test]
    fn test_round_trip_map() {
        let original = json!({
            "events": [null, {
                "id": 1,
                "pages": [{
                    "list": [
                        { "code": 401, "parameters": ["こんにちは！"] },
                        { "code": 102, "parameters": [["はい", "いいえ"], 0, 0, 0, 0] }
                    ]
                }]
            }]
        });

        let segments = extract_map(&original);
        assert_eq!(segments.len(), 3);

        // Simulate translation: keep original text (round-trip identity check)
        let translations: Vec<(&str, &str)> = segments
            .iter()
            .map(|s| (s.key.as_str(), s.source.as_str()))
            .collect();

        let mut modified = original.clone();
        inject(&mut modified, &translations).unwrap();

        assert_eq!(
            original, modified,
            "JSON must be identical after round-trip"
        );
    }

    #[test]
    fn test_round_trip_items() {
        let original = json!([
            null,
            { "id": 1, "name": "ポーション", "description": "HPを50回復する。" }
        ]);

        let segments = extract_items(&original);
        let translations: Vec<(&str, &str)> = segments
            .iter()
            .map(|s| (s.key.as_str(), s.source.as_str()))
            .collect();

        let mut modified = original.clone();
        inject(&mut modified, &translations).unwrap();

        assert_eq!(
            original, modified,
            "JSON must be identical after round-trip"
        );
    }

    #[test]
    fn test_inject_error_key_not_found() {
        let mut json = json!({ "name": "テスト" });
        let result = inject(&mut json, &[("/nonexistent", "value")]);
        assert!(matches!(result, Err(InjectorError::KeyNotFound(_))));
    }

    #[test]
    fn test_inject_error_not_a_string() {
        let mut json = json!({ "count": 42 });
        let result = inject(&mut json, &[("/count", "should fail")]);
        assert!(matches!(result, Err(InjectorError::NotAString(_))));
    }
}
