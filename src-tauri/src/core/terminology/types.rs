use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use super::TerminologyError;

macro_rules! string_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        #[serde(rename_all = "snake_case")]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }
        }

        impl Display for $name {
            fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = TerminologyError;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                match value {
                    $($value => Ok(Self::$variant),)+
                    other => Err(TerminologyError::InvalidInput(format!(
                        "unknown {} value `{other}`",
                        stringify!($name)
                    ))),
                }
            }
        }
    };
}

string_enum!(PartOfSpeech {
    Noun => "noun",
    ProperNoun => "proper_noun",
    Verb => "verb",
    Adjective => "adjective",
    Adverb => "adverb",
    Expression => "expression",
    Unknown => "unknown",
});

#[allow(clippy::derivable_impls)]
impl Default for PartOfSpeech {
    fn default() -> Self {
        Self::Unknown
    }
}

string_enum!(EntryStatus {
    Active => "active",
    Ignored => "ignored",
    Archived => "archived",
});

#[allow(clippy::derivable_impls)]
impl Default for EntryStatus {
    fn default() -> Self {
        Self::Active
    }
}

string_enum!(TermOrigin {
    Engine => "engine",
    Lindera => "lindera",
    Manual => "manual",
    LegacyGlossary => "legacy_glossary",
    Import => "import",
});

string_enum!(Enforcement {
    Contextual => "contextual",
    Preferred => "preferred",
    Required => "required",
});

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyTranslationView {
    pub id: String,
    pub target_language: String,
    pub project_id: Option<String>,
    pub target_text: String,
    pub enforcement: Enforcement,
    pub confidence: f64,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    pub accepted_variants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyContextView {
    pub segment_id: String,
    pub surface_text: String,
    pub source_text: String,
    pub engine_kind: String,
    pub occurrence_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyEntryView {
    pub id: String,
    pub source_language: String,
    pub canonical_text: String,
    pub normalized_text: String,
    pub reading: Option<String>,
    pub part_of_speech: PartOfSpeech,
    pub semantic_type: String,
    pub sense_key: String,
    pub status: EntryStatus,
    pub origin: TermOrigin,
    pub confidence: f64,
    pub occurrence_count: i64,
    pub translation: Option<TerminologyTranslationView>,
    pub has_project_translation: bool,
    pub has_global_translation: bool,
    pub contexts: Vec<TerminologyContextView>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TerminologyQuery {
    pub source_language: String,
    pub target_language: String,
    pub project_id: Option<String>,
    pub search: Option<String>,
    pub part_of_speech: Option<PartOfSpeech>,
    pub semantic_type: Option<String>,
    pub status: Option<EntryStatus>,
    pub page: i64,
    pub page_size: i64,
}

impl Default for TerminologyQuery {
    fn default() -> Self {
        Self {
            source_language: "ja".to_string(),
            target_language: "en".to_string(),
            project_id: None,
            search: None,
            part_of_speech: None,
            semantic_type: None,
            status: Some(EntryStatus::Active),
            page: 0,
            page_size: 100,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaginatedTerminology {
    pub items: Vec<TerminologyEntryView>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateTermInput {
    pub source_language: String,
    pub canonical_text: String,
    pub reading: Option<String>,
    #[serde(default)]
    pub part_of_speech: PartOfSpeech,
    pub semantic_type: String,
    #[serde(default)]
    pub sense_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateTermInput {
    pub id: String,
    pub canonical_text: String,
    pub reading: Option<String>,
    pub part_of_speech: PartOfSpeech,
    pub semantic_type: String,
    pub sense_key: String,
    pub status: EntryStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpsertTranslationInput {
    pub entry_id: String,
    pub target_language: String,
    pub project_id: Option<String>,
    pub target_text: String,
    pub enforcement: Enforcement,
    pub confidence: f64,
    pub provider_id: Option<String>,
    pub model: Option<String>,
    #[serde(default)]
    pub accepted_variants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyStats {
    pub total_entries: i64,
    pub untranslated_entries: i64,
    pub project_translations: i64,
    pub global_translations: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TranslationScope {
    Project,
    Global,
    Both,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GlobalizeTranslationsInput {
    pub entry_ids: Vec<String>,
    pub target_language: String,
    pub project_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct GlobalizeTranslationsSummary {
    pub copied: u64,
    pub already_global: u64,
    pub conflicts: u64,
    pub skipped: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enums_use_stable_snake_case_contracts() {
        assert_eq!(
            serde_json::to_string(&PartOfSpeech::ProperNoun).unwrap(),
            "\"proper_noun\""
        );
        assert_eq!(
            "required".parse::<Enforcement>().unwrap(),
            Enforcement::Required
        );
        assert!("mandatory".parse::<Enforcement>().is_err());
    }

    #[test]
    fn query_defaults_are_bounded_and_language_explicit() {
        let query: TerminologyQuery = serde_json::from_str("{}").unwrap();
        assert_eq!(query.source_language, "ja");
        assert_eq!(query.target_language, "en");
        assert_eq!(query.page_size, 100);
        assert_eq!(query.status, Some(EntryStatus::Active));
    }
}
