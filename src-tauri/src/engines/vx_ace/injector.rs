//! RPG Maker VX Ace — .rvdata2 injector
//!
//! Writes translated text back into a parsed `serde_json::Value` using the same
//! JSON Pointer keys produced by `extractor`, then serialises back to Ruby Marshal
//! bytes via `marshal-rs`.
//!
//! ## Workflow
//! ```text
//! raw bytes (.rvdata2)
//!   → load_utf8(bytes, None) → marshal_rs::Value → .into() → serde_json::Value
//!   → inject(value, translations)
//!   → .into() → marshal_rs::Value → dump(val, None) → raw bytes
//! ```
//!
//! The `inject()` function is identical to the MV/MZ injector — both work on
//! `serde_json::Value` with `pointer_mut`. The difference is `inject_and_serialize`
//! which handles the Marshal serialisation layer.

use marshal_rs::{dump, load_utf8};
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum InjectorError {
    #[error("key not found in Marshal value: {0}")]
    KeyNotFound(String),
    #[error("target at key is not a string: {0}")]
    NotAString(String),
    #[error("failed to parse .rvdata2: {0}")]
    ParseError(String),
}

/// Inject translated segments into a parsed `serde_json::Value`.
///
/// `translations` is a slice of `(json_pointer_key, translated_text)` pairs.
/// Returns an error on the first key that is missing or does not point to a string.
pub fn inject(value: &mut Value, translations: &[(&str, &str)]) -> Result<(), InjectorError> {
    for (key, text) in translations {
        match value.pointer_mut(key) {
            Some(target) if target.is_string() => {
                *target = Value::String(text.to_string());
            }
            Some(_) => return Err(InjectorError::NotAString(key.to_string())),
            None => return Err(InjectorError::KeyNotFound(key.to_string())),
        }
    }
    Ok(())
}

/// Serialise a `serde_json::Value` back to Ruby Marshal bytes.
pub fn serialize(value: Value) -> Vec<u8> {
    let mv: marshal_rs::Value = value.into();
    dump(mv, None)
}

/// Load, inject, and serialise in one step.
///
/// Convenience function used by `export_project` in the commands layer.
/// Returns the updated `.rvdata2` bytes ready to write back to disk.
pub fn inject_and_serialize(
    bytes: &[u8],
    translations: &[(&str, &str)],
) -> Result<Vec<u8>, InjectorError> {
    let mv: marshal_rs::Value =
        load_utf8(bytes, None).map_err(|e| InjectorError::ParseError(e.to_string()))?;
    let mut value: Value = mv.into();
    inject(&mut value, translations)?;
    Ok(serialize(value))
}

// ---------------------------------------------------------------------------
// Tests — Step 7
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::vx_ace::extractor::{extract_actors, extract_map};
    use marshal_rs::load_utf8;
    use serde_json::json;

    fn to_json(bytes: &[u8]) -> Value {
        let mv: marshal_rs::Value = load_utf8(bytes, None).unwrap();
        mv.into()
    }

    // --- inject() ---

    #[test]
    fn test_inject_actor_name() {
        let mut value = json!([null, { "name": "主人公", "nickname": "勇者" }]);
        inject(&mut value, &[("/1/name", "Hero")]).unwrap();
        assert_eq!(
            value.pointer("/1/name").and_then(Value::as_str),
            Some("Hero")
        );
    }

    #[test]
    fn test_inject_error_key_not_found() {
        let mut value = json!({ "name": "テスト" });
        let result = inject(&mut value, &[("/nonexistent", "value")]);
        assert!(matches!(result, Err(InjectorError::KeyNotFound(_))));
    }

    #[test]
    fn test_inject_error_not_a_string() {
        let mut value = json!({ "count": 42 });
        let result = inject(&mut value, &[("/count", "should fail")]);
        assert!(matches!(result, Err(InjectorError::NotAString(_))));
    }

    // --- round-trip ---

    #[test]
    fn test_round_trip_actors() {
        let original = json!([
            null,
            { "id": 1, "name": "主人公", "nickname": "勇者", "description": "冒険者" }
        ]);
        let segments = extract_actors(&original);

        // identity translation (source as target)
        let translations: Vec<(&str, &str)> = segments
            .iter()
            .map(|s| (s.key.as_str(), s.source.as_str()))
            .collect();

        let mut modified = original.clone();
        inject(&mut modified, &translations).unwrap();
        assert_eq!(
            original, modified,
            "Value must be identical after round-trip"
        );
    }

    #[test]
    fn test_round_trip_map() {
        let original = json!({
            "events": {
                "1": {
                    "id": 1,
                    "pages": [{
                        "list": [
                            { "code": 401, "parameters": ["こんにちは！"] },
                            { "code": 102, "parameters": [["はい", "いいえ"], 0] }
                        ]
                    }]
                }
            }
        });
        let segments = extract_map(&original);
        assert_eq!(segments.len(), 3);

        let translations: Vec<(&str, &str)> = segments
            .iter()
            .map(|s| (s.key.as_str(), s.source.as_str()))
            .collect();

        let mut modified = original.clone();
        inject(&mut modified, &translations).unwrap();
        assert_eq!(
            original, modified,
            "Map Value must be identical after round-trip"
        );
    }

    // --- serialize + Value idempotence ---

    #[test]
    fn test_serialize_produces_valid_marshal() {
        // Correct test: load_utf8(dump(load_utf8(bytes))) == load_utf8(bytes)
        // NOT: dump(load_utf8(bytes)) == bytes  (bytes differ due to encoding)
        let original = json!([null, { "name": "テスト", "description": "説明" }]);
        let bytes = serialize(original);
        let reloaded = to_json(&bytes);
        let bytes2 = serialize(reloaded.clone());
        let reloaded2 = to_json(&bytes2);
        assert_eq!(
            reloaded, reloaded2,
            "Value idempotent after double marshal round-trip"
        );
    }

    // --- inject_and_serialize ---

    #[test]
    fn test_inject_and_serialize_round_trip() {
        let original = json!([null, { "id": 1, "name": "主人公", "nickname": "勇者" }]);
        let bytes = serialize(original.clone());

        let result = inject_and_serialize(&bytes, &[("/1/name", "Hero"), ("/1/nickname", "Brave")]);
        assert!(result.is_ok());

        let updated = to_json(&result.unwrap());
        assert_eq!(
            updated.pointer("/1/name").and_then(Value::as_str),
            Some("Hero")
        );
        assert_eq!(
            updated.pointer("/1/nickname").and_then(Value::as_str),
            Some("Brave")
        );
        // Unchanged field preserved
        assert_eq!(updated.pointer("/0"), Some(&serde_json::Value::Null));
    }
}
