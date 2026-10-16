//! Curated plugin profiles: which `plugins.js` parameters and plugin-command
//! arguments are player-visible text, verified against a specific plugin
//! source.
//!
//! A plugin parameter can hold display text, an enum value compared
//! literally, a note-tag key, a key binding or an editor memo — they all look
//! alike. Nothing is offered for translation unless a profile reviewed that
//! exact field in that exact plugin version. Profiles live as TOML under
//! `src-tauri/profiles/mv_mz/` and are embedded at compile time.

use std::collections::BTreeMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;
use sha2::{Digest, Sha256};

/// Embedded profile sources (file name, TOML).
const EMBEDDED: &[(&str, &str)] = &[
    (
        "nuun_enemy_book.toml",
        include_str!("../../../profiles/mv_mz/nuun_enemy_book.toml"),
    ),
    (
        "keke_variable_actor_command.toml",
        include_str!("../../../profiles/mv_mz/keke_variable_actor_command.toml"),
    ),
    (
        "sasuke_battle_voice.toml",
        include_str!("../../../profiles/mv_mz/sasuke_battle_voice.toml"),
    ),
    (
        "sasuke_tiny_get_info_wnd.toml",
        include_str!("../../../profiles/mv_mz/sasuke_tiny_get_info_wnd.toml"),
    ),
    (
        "change_equip_on_battle.toml",
        include_str!("../../../profiles/mv_mz/change_equip_on_battle.toml"),
    ),
    (
        "mpp_smooth_battle_log.toml",
        include_str!("../../../profiles/mv_mz/mpp_smooth_battle_log.toml"),
    ),
    (
        "potetodoragon_skill_cost.toml",
        include_str!("../../../profiles/mv_mz/potetodoragon_skill_cost.toml"),
    ),
    (
        "toriacontan_battle_log_output.toml",
        include_str!("../../../profiles/mv_mz/toriacontan_battle_log_output.toml"),
    ),
    (
        "toriacontan_damage_se_effective.toml",
        include_str!("../../../profiles/mv_mz/toriacontan_damage_se_effective.toml"),
    ),
    (
        "toriacontan_character_picture_manager.toml",
        include_str!("../../../profiles/mv_mz/toriacontan_character_picture_manager.toml"),
    ),
    (
        "toriacontan_dynamic_actor_graphic.toml",
        include_str!("../../../profiles/mv_mz/toriacontan_dynamic_actor_graphic.toml"),
    ),
    (
        "toriacontan_trigger_on_equip_and_state.toml",
        include_str!("../../../profiles/mv_mz/toriacontan_trigger_on_equip_and_state.toml"),
    ),
    (
        "toriacontan_message_window_hidden.toml",
        include_str!("../../../profiles/mv_mz/toriacontan_message_window_hidden.toml"),
    ),
    (
        "ll_standing_picture_battle.toml",
        include_str!("../../../profiles/mv_mz/ll_standing_picture_battle.toml"),
    ),
    (
        "nrp_seamless_encounter.toml",
        include_str!("../../../profiles/mv_mz/nrp_seamless_encounter.toml"),
    ),
    (
        "mpp_choice_ex.toml",
        include_str!("../../../profiles/mv_mz/mpp_choice_ex.toml"),
    ),
    (
        "plugin_common_base.toml",
        include_str!("../../../profiles/mv_mz/plugin_common_base.toml"),
    ),
    (
        "toriacontan_template_event.toml",
        include_str!("../../../profiles/mv_mz/toriacontan_template_event.toml"),
    ),
    (
        "nuun_base.toml",
        include_str!("../../../profiles/mv_mz/nuun_base.toml"),
    ),
    (
        "toriacontan_message_skip.toml",
        include_str!("../../../profiles/mv_mz/toriacontan_message_skip.toml"),
    ),
];

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginProfile {
    pub plugin: PluginMeta,
    #[serde(default)]
    pub display: Vec<DisplayRule>,
    #[serde(default)]
    pub fill_in: Vec<FillInRule>,
    #[serde(default)]
    pub blocked: Vec<BlockedRule>,
    #[serde(default)]
    pub memo: Vec<MemoRule>,
    #[serde(default)]
    pub command_arg: Vec<CommandArgRule>,
    /// How the plugin affects message/help text layout (text fitting).
    #[serde(default)]
    pub layout: Option<LayoutRule>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PluginMeta {
    /// Name as written in `plugins.js` (also `js/plugins/<name>.js`).
    pub name: String,
    pub title: String,
    /// `@version` values of the reviewed source.
    #[serde(default)]
    pub verified_versions: Vec<String>,
    /// SHA-256 of the reviewed source with CRLF normalized to LF.
    #[serde(default)]
    pub verified_sha256: Vec<String>,
    /// What was reviewed (shown to the user).
    pub reference: String,
    /// Parameters/arguments go through PluginCommonBase, which turns
    /// `true`/`false`, numbers and JSON text into other types.
    #[serde(default)]
    pub coerces_values: bool,
    /// Note-tag grammars of this plugin that Hoshi2Star implements
    /// (`keke_command`).
    #[serde(default)]
    pub note_tag_families: Vec<String>,
}

/// A parameter path whose value is drawn on screen.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DisplayRule {
    pub path: String,
    pub visible_in: String,
    /// Sentence-like text (battle log, popups) rather than a short label.
    #[serde(default)]
    pub message: bool,
    /// `String.format` template: `%1`, `%2`… must be kept.
    #[serde(default)]
    pub template: bool,
    /// Rendered with `drawTextEx` (escape codes work).
    #[serde(default)]
    pub escape_codes: bool,
}

/// An *empty* parameter whose on-screen default is hard-coded in the plugin
/// (chosen by a sibling field). Writing a label into the empty field replaces
/// that default.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FillInRule {
    pub path: String,
    /// Sibling field selecting the default (e.g. `DateSelect`).
    pub key_field: String,
    pub visible_in: String,
    /// `key_field` value → default text the plugin shows.
    pub defaults: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BlockedRule {
    pub path: String,
    pub reason: String,
}

/// Editor-only field (never read at runtime): neither offered nor treated as
/// an identifier site.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MemoRule {
    pub path: String,
}

/// An MZ plugin-command argument (code 357) that is display text.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CommandArgRule {
    pub command: String,
    pub arg: String,
    pub title: String,
    pub visible_in: String,
    #[serde(default)]
    pub message: bool,
    #[serde(default)]
    pub template: bool,
    #[serde(default)]
    pub escape_codes: bool,
}

/// What a reviewed plugin does to message and help-window text layout.
#[derive(Debug, Clone, Default, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LayoutRule {
    /// Its hooks on text functions do not change where text is drawn, how
    /// wide it is or where pages break.
    #[serde(default)]
    pub neutral: bool,
    /// Escape codes it adds that print a value (`S`, `SS`, `SV`…).
    #[serde(default)]
    pub inserted_codes: Vec<String>,
    /// Draws the first line of item/weapon/armor descriptions as plain text
    /// elsewhere (escape codes would show literally there).
    #[serde(default)]
    pub plain_description_first_line: bool,
    /// Shown to the user in the text fitting dialog.
    #[serde(default)]
    pub notes: Vec<String>,
}

/// A loaded, validated set of profiles.
#[derive(Debug, Clone, Default)]
pub struct ProfileSet {
    profiles: Vec<PluginProfile>,
}

static EMBEDDED_SET: LazyLock<ProfileSet> =
    LazyLock::new(|| match ProfileSet::parse_all(EMBEDDED) {
        Ok(set) => set,
        Err(error) => {
            // Unreachable in a tested build (see `embedded_profiles_are_valid`);
            // fail safe: no profile means nothing is offered.
            log::error!("embedded plugin profiles are invalid: {error}");
            ProfileSet::default()
        }
    });

impl ProfileSet {
    pub fn embedded() -> &'static ProfileSet {
        &EMBEDDED_SET
    }

    pub fn parse_all(sources: &[(&str, &str)]) -> Result<Self, String> {
        let mut profiles = Vec::with_capacity(sources.len());
        for (file, source) in sources {
            let profile: PluginProfile =
                toml::from_str(source).map_err(|error| format!("{file}: {error}"))?;
            profile
                .validate()
                .map_err(|error| format!("{file}: {error}"))?;
            if profiles
                .iter()
                .any(|existing: &PluginProfile| existing.plugin.name == profile.plugin.name)
            {
                return Err(format!(
                    "{file}: duplicate profile for {}",
                    profile.plugin.name
                ));
            }
            profiles.push(profile);
        }
        Ok(Self { profiles })
    }

    pub fn find(&self, plugin_name: &str) -> Option<&PluginProfile> {
        self.profiles
            .iter()
            .find(|profile| profile.plugin.name == plugin_name)
    }

    pub fn len(&self) -> usize {
        self.profiles.len()
    }

    pub fn is_empty(&self) -> bool {
        self.profiles.is_empty()
    }
}

impl PluginProfile {
    fn validate(&self) -> Result<(), String> {
        if self.plugin.verified_versions.is_empty() && self.plugin.verified_sha256.is_empty() {
            return Err("a profile must pin at least one version or sha256".into());
        }
        for family in &self.plugin.note_tag_families {
            if family != crate::engines::mv_mz::keke::FAMILY {
                return Err(format!("unknown note-tag family {family}"));
            }
        }
        if let Some(layout) = &self.layout {
            for code in &layout.inserted_codes {
                if code.is_empty() || !code.chars().all(|c| c.is_ascii_uppercase()) {
                    return Err(format!("invalid inserted escape code {code}"));
                }
            }
        }
        for hash in &self.plugin.verified_sha256 {
            if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err(format!("invalid sha256 {hash}"));
            }
        }
        let paths = self
            .display
            .iter()
            .map(|rule| &rule.path)
            .chain(self.fill_in.iter().map(|rule| &rule.path))
            .chain(self.blocked.iter().map(|rule| &rule.path))
            .chain(self.memo.iter().map(|rule| &rule.path));
        for path in paths {
            PathPattern::parse(path)?;
        }
        Ok(())
    }

    /// Whether the installed plugin source is one this profile reviewed.
    pub fn verify(&self, source: &str) -> Verification {
        let version = header_version(source);
        if let Some(version) = &version {
            if self.plugin.verified_versions.iter().any(|v| v == version) {
                return Verification::Verified;
            }
        }
        if self
            .plugin
            .verified_sha256
            .iter()
            .any(|h| *h == normalized_sha256(source))
        {
            return Verification::Verified;
        }
        Verification::Unverified {
            found_version: version,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verification {
    Verified,
    Unverified { found_version: Option<String> },
}

static RE_VERSION: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?m)^\s*\*\s*@version\s+(\S+)").expect("RE_VERSION must compile")
});

/// `@version` declared in the plugin's documentation header, if any.
pub fn header_version(source: &str) -> Option<String> {
    RE_VERSION.captures(source).map(|caps| caps[1].to_string())
}

/// SHA-256 of `source` with CRLF normalized to LF (line endings vary between
/// distributions of the same file).
pub fn normalized_sha256(source: &str) -> String {
    hex::encode(Sha256::digest(source.replace("\r\n", "\n").as_bytes()))
}

/// Parsed rule path: `Param`, `List[].Field`, `PageList*[].paramName`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathPattern {
    tokens: Vec<PatternToken>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PatternToken {
    Exact(String),
    /// Parameter-name prefix (`PageList*`), first token only.
    Prefix(String),
    AnyIndex,
}

impl PathPattern {
    pub fn parse(pattern: &str) -> Result<Self, String> {
        if pattern.trim().is_empty() {
            return Err("empty path".into());
        }
        let mut tokens = Vec::new();
        for (index, part) in pattern.split('.').enumerate() {
            let (name, indices) = match part.find("[]") {
                Some(at) => (&part[..at], &part[at..]),
                None => (part, ""),
            };
            if !indices.chars().all(|c| c == '[' || c == ']') || indices.len() % 2 != 0 {
                return Err(format!("invalid index syntax in `{pattern}`"));
            }
            if name.is_empty() {
                return Err(format!("empty segment in `{pattern}`"));
            }
            let bare = name.strip_suffix('*').unwrap_or(name);
            if bare.contains(['[', ']', '*']) {
                return Err(format!(
                    "invalid characters in segment `{name}` of `{pattern}`"
                ));
            }
            match name.strip_suffix('*') {
                Some(prefix) if index == 0 && !prefix.is_empty() => {
                    tokens.push(PatternToken::Prefix(prefix.to_string()))
                }
                Some(_) => {
                    return Err(format!(
                        "`*` is only allowed at the end of the first segment in `{pattern}`"
                    ))
                }
                None => tokens.push(PatternToken::Exact(name.to_string())),
            }
            for _ in 0..indices.len() / 2 {
                tokens.push(PatternToken::AnyIndex);
            }
        }
        Ok(Self { tokens })
    }

    pub fn matches(&self, path: &[String]) -> bool {
        self.tokens.len() == path.len()
            && self
                .tokens
                .iter()
                .zip(path)
                .all(|(token, part)| match token {
                    PatternToken::Exact(name) => name == part,
                    PatternToken::Prefix(prefix) => part.starts_with(prefix.as_str()),
                    PatternToken::AnyIndex => {
                        !part.is_empty() && part.chars().all(|c| c.is_ascii_digit())
                    }
                })
    }
}

/// How a profile classifies one parameter leaf.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeafRole<'a> {
    Display(&'a DisplayRule),
    FillIn(&'a FillInRule),
    Blocked(&'a BlockedRule),
    Memo,
    Unreviewed,
}

impl PluginProfile {
    pub fn classify_leaf(&self, path: &[String]) -> LeafRole<'_> {
        let matches = |pattern: &str| PathPattern::parse(pattern).is_ok_and(|p| p.matches(path));
        if let Some(rule) = self.blocked.iter().find(|rule| matches(&rule.path)) {
            return LeafRole::Blocked(rule);
        }
        if self.memo.iter().any(|rule| matches(&rule.path)) {
            return LeafRole::Memo;
        }
        if let Some(rule) = self.display.iter().find(|rule| matches(&rule.path)) {
            return LeafRole::Display(rule);
        }
        if let Some(rule) = self.fill_in.iter().find(|rule| matches(&rule.path)) {
            return LeafRole::FillIn(rule);
        }
        LeafRole::Unreviewed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(parts: &[&str]) -> Vec<String> {
        parts.iter().map(|p| p.to_string()).collect()
    }

    #[test]
    fn embedded_profiles_are_valid() {
        let set =
            ProfileSet::parse_all(EMBEDDED).expect("embedded profiles must parse and validate");
        assert_eq!(set.len(), EMBEDDED.len());
        assert!(set.find("nuun/NUUN_EnemyBook").is_some());
        // Shipped profiles pin the exact reviewed source: a `@version` header
        // alone also matches edited copies of the plugin.
        for profile in &set.profiles {
            assert!(
                !profile.plugin.verified_sha256.is_empty(),
                "{} is not pinned by sha256",
                profile.plugin.name
            );
        }
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let source = r#"
            [plugin]
            name = "X"
            title = "X"
            verified_versions = ["1.0"]
            reference = "r"
            typo_field = true
        "#;
        assert!(ProfileSet::parse_all(&[("x.toml", source)]).is_err());
    }

    #[test]
    fn profiles_must_pin_a_version_or_hash() {
        let source = "[plugin]\nname = \"X\"\ntitle = \"X\"\nreference = \"r\"\n";
        let error = ProfileSet::parse_all(&[("x.toml", source)]).unwrap_err();
        assert!(error.contains("pin"));
    }

    #[test]
    fn path_patterns_match_indices_and_prefixes() {
        let pattern = PathPattern::parse("PageList*[].paramName").unwrap();
        assert!(pattern.matches(&path(&["PageList1", "11", "paramName"])));
        assert!(pattern.matches(&path(&["PageList12", "0", "paramName"])));
        assert!(!pattern.matches(&path(&["PageSetting", "0", "paramName"])));
        assert!(!pattern.matches(&path(&["PageList1", "x", "paramName"])));
        assert!(!pattern.matches(&path(&["PageList1", "1"])));
        let plain = PathPattern::parse("Battle Voice Name at Option").unwrap();
        assert!(plain.matches(&path(&["Battle Voice Name at Option"])));
        let nested = PathPattern::parse("damageSeList[].condition.elementId").unwrap();
        assert!(nested.matches(&path(&["damageSeList", "3", "condition", "elementId"])));
        for bad in ["", "a.", "a*b.c", "x.y*", "a[x]"] {
            assert!(PathPattern::parse(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn verification_accepts_versions_or_normalized_hashes_only() {
        let source = "/*:\r\n * @plugindesc test\r\n * @version 2.23.4\r\n */\r\nvar x = 1;\r\n";
        let lf = source.replace("\r\n", "\n");
        let profile: PluginProfile = toml::from_str(&format!(
            "[plugin]\nname = \"A\"\ntitle = \"A\"\nreference = \"r\"\nverified_versions = [\"9.9\"]\nverified_sha256 = [\"{}\"]\n",
            normalized_sha256(&lf)
        ))
        .unwrap();
        assert_eq!(header_version(source).as_deref(), Some("2.23.4"));
        // Same content with CRLF endings hashes identically.
        assert_eq!(profile.verify(source), Verification::Verified);
        assert_eq!(
            profile.verify("var x = 2;"),
            Verification::Unverified {
                found_version: None
            }
        );
        let versioned: PluginProfile = toml::from_str(
            "[plugin]\nname = \"A\"\ntitle = \"A\"\nreference = \"r\"\nverified_versions = [\"2.23.4\"]\n",
        )
        .unwrap();
        assert_eq!(versioned.verify(source), Verification::Verified);
        assert_eq!(
            versioned.verify(" * @version 2.24.0\n"),
            Verification::Unverified {
                found_version: Some("2.24.0".into())
            }
        );
    }

    #[test]
    fn blocked_and_memo_rules_win_over_display() {
        let profile: PluginProfile = toml::from_str(
            r#"
            [plugin]
            name = "A"
            title = "A"
            reference = "r"
            verified_versions = ["1"]
            [[display]]
            path = "List[].name"
            visible_in = "x"
            [[blocked]]
            path = "List[].name"
            reason = "key"
            [[memo]]
            path = "Memo"
            "#,
        )
        .unwrap();
        assert!(matches!(
            profile.classify_leaf(&path(&["List", "0", "name"])),
            LeafRole::Blocked(_)
        ));
        assert_eq!(profile.classify_leaf(&path(&["Memo"])), LeafRole::Memo);
        assert_eq!(
            profile.classify_leaf(&path(&["Other"])),
            LeafRole::Unreviewed
        );
    }
}
