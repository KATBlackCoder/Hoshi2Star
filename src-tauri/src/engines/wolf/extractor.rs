// Wolf RPG text extractor — F4-03 implementation.

// ---------------------------------------------------------------------------
// Error type
// ---------------------------------------------------------------------------

#[derive(Debug, thiserror::Error)]
pub enum ExtractorError {
    #[error("decryptor error: {0}")]
    Decryptor(#[from] super::decryptor::DecryptorError),
    #[error("map parser error: {0}")]
    MapParser(String),
    #[error("encoding error: {0}")]
    Encoding(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported Wolf RPG version: {0}.{1}")]
    UnsupportedVersion(u8, u8),
}

// ---------------------------------------------------------------------------
// Public output types
// ---------------------------------------------------------------------------

/// A single translatable text unit extracted from a Wolf RPG game.
///
/// `key` uniquely addresses this segment for re-injection by the injector (F4-04).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WolfSegment {
    /// Unique key, e.g. "MapData/Map001/events/0/pages/0/42"
    /// or "Database/Actors/0/name"
    pub key: String,
    /// Source text in UTF-8 (decoded from Shift-JIS if the file was v2).
    pub source_text: String,
    /// Segment kind — carries context for the injector and CAT UI.
    pub kind: WolfSegmentKind,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WolfSegmentKind {
    /// Dialogue / in-game displayed text (from .mps map files).
    MapMessage {
        map_name: String,
        event_idx: usize,
        page_idx: usize,
        cmd_idx: usize,
    },
    /// Database field (from .dat database files).
    DatabaseField {
        db_name: String,
        entry_idx: usize,
        field_name: String,
    },
}
