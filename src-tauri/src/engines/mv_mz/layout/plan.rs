//! Fitting a whole file: what the export writes, and what the scan reports.
//!
//! Every changed message or description passes the final fit check
//! ([`verify::check`]); one that fails keeps its translation as is.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::fit::{
    effective_text, fit_description, fit_message, help_box, message_box, FitContext, FitOutcome,
    FitSettings, Fitted, LineBreaks, MessageInput, MessageLine, Overflow,
};
use super::message::{descriptions_in, messages_in};
use super::simulate::BoxSpec;
use super::verify;

/// A segment's translation, keyed by its JSON pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    pub segment_id: String,
    pub text: String,
}

/// A per-message (or per-description) choice.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FitOverride {
    pub line_breaks: Option<LineBreaks>,
    pub overflow: Option<Overflow>,
    /// Export this text exactly as translated.
    #[serde(default)]
    pub disabled: bool,
}

/// An override and the original text it was made for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredOverride {
    pub choice: FitOverride,
    pub source_hash: String,
}

/// Hash of the original text a message or description is made of.
pub fn source_hash(texts: &[&str]) -> String {
    hex::encode(Sha256::digest(texts.join("\n").as_bytes()))
}

/// A fitted message or description.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FitItem {
    /// 101 command pointer, or the description field pointer.
    pub pointer: String,
    pub description: bool,
    pub face: bool,
    /// Pointers of the values (`lines[i]` ↔ `values[i]`).
    pub lines: Vec<String>,
    /// Segment ids of the lines (`None` for verbatim lines).
    pub segment_ids: Vec<Option<String>>,
    /// Text as translated (verbatim for lines without a segment).
    pub texts: Vec<String>,
    pub source_hash: String,
    #[serde(rename = "override")]
    pub choice: Option<FitOverride>,
    pub fitted: Fitted,
}

impl FitItem {
    /// `(pointer, new value)` the export writes.
    pub fn writes(&self) -> impl Iterator<Item = (&str, &str)> {
        self.lines
            .iter()
            .zip(&self.fitted.values)
            .filter_map(|(pointer, value)| value.as_deref().map(|v| (pointer.as_str(), v)))
    }
}

fn settings_for(base: &FitSettings, choice: Option<&FitOverride>) -> FitSettings {
    let mut settings = base.clone();
    if let Some(choice) = choice {
        if let Some(line_breaks) = choice.line_breaks {
            settings.line_breaks = line_breaks;
        }
        if let Some(overflow) = choice.overflow {
            settings.overflow = overflow;
        }
    }
    settings
}

fn valid_choice<'a>(
    overrides: &'a HashMap<String, StoredOverride>,
    pointer: &str,
    hash: &str,
) -> Option<&'a FitOverride> {
    overrides
        .get(pointer)
        .filter(|stored| stored.source_hash == hash)
        .map(|stored| &stored.choice)
}

/// Keep a fit only if its final values pass the fit check.
fn checked(
    fitted: Fitted,
    translated: &str,
    output: &str,
    spec: &BoxSpec,
    ctx: &FitContext,
) -> Fitted {
    if fitted.values.iter().all(Option::is_none) {
        return fitted;
    }
    let allow_overflow = matches!(fitted.outcome, FitOutcome::TooLong { .. });
    let problems = verify::check(
        translated,
        output,
        spec,
        ctx.metrics,
        ctx.inserts,
        allow_overflow,
    );
    if problems.is_empty() {
        return fitted;
    }
    Fitted {
        outcome: FitOutcome::Skipped {
            reason: format!("the fit check failed ({problems:?}); exported as translated"),
        },
        values: vec![None; fitted.values.len()],
        pages: fitted.pages,
    }
}

/// Fit every Show Text message of an event file.
pub fn fit_messages(
    file_name: &str,
    json: &Value,
    targets: &HashMap<String, Target>,
    settings: &FitSettings,
    overrides: &HashMap<String, StoredOverride>,
    ctx: &FitContext,
) -> Vec<FitItem> {
    let mut out = Vec::new();
    for message in messages_in(file_name, json) {
        let mut lines = Vec::with_capacity(message.lines.len());
        let mut pointers = Vec::with_capacity(message.lines.len());
        let mut segment_ids = Vec::with_capacity(message.lines.len());
        let mut untranslated = false;
        for (index, original) in &message.lines {
            let pointer = message.line_pointer(*index);
            match targets.get(&pointer) {
                Some(target) if target.text.is_empty() => {
                    untranslated = true;
                    lines.push(MessageLine {
                        text: original.clone(),
                        writable: false,
                    });
                    segment_ids.push(Some(target.segment_id.clone()));
                }
                Some(target) => {
                    lines.push(MessageLine {
                        text: target.text.clone(),
                        writable: true,
                    });
                    segment_ids.push(Some(target.segment_id.clone()));
                }
                None => {
                    lines.push(MessageLine {
                        text: original.clone(),
                        writable: false,
                    });
                    segment_ids.push(None);
                }
            }
            pointers.push(pointer);
        }
        if !lines.iter().any(|line| line.writable) && !untranslated {
            continue;
        }
        let originals: Vec<&str> = message
            .lines
            .iter()
            .map(|(_, text)| text.as_str())
            .collect();
        let hash = source_hash(&originals);
        let pointer = message.pointer();
        let choice = valid_choice(overrides, &pointer, &hash).cloned();
        let input = MessageInput {
            lines,
            face: message.face,
        };
        // What the output must still say: the translation, minus the stored
        // export prefixes the export chose to replace.
        let translated = input
            .lines
            .iter()
            .map(|l| effective_text(l, settings))
            .collect::<Vec<_>>()
            .join("\n");
        let fitted = if untranslated {
            skipped(&input, ctx, "not fully translated yet")
        } else if choice.as_ref().is_some_and(|c| c.disabled) {
            skipped(&input, ctx, "fitting turned off for this message")
        } else {
            let fitted = fit_message(&input, ctx, &settings_for(settings, choice.as_ref()));
            let output = input
                .lines
                .iter()
                .zip(&fitted.values)
                .map(|(line, value)| value.as_deref().unwrap_or(&line.text))
                .collect::<Vec<_>>()
                .join("\n");
            let spec = message_box(ctx, input.face);
            checked(fitted, &translated, &output, &spec, ctx)
        };
        out.push(FitItem {
            pointer,
            description: false,
            face: message.face,
            lines: pointers,
            segment_ids,
            texts: input.lines.iter().map(|l| l.text.clone()).collect(),
            source_hash: hash,
            choice,
            fitted,
        });
    }
    out
}

fn skipped(input: &MessageInput, ctx: &FitContext, reason: &str) -> Fitted {
    let text = input
        .lines
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    Fitted {
        outcome: FitOutcome::Skipped {
            reason: reason.to_string(),
        },
        values: vec![None; input.lines.len()],
        pages: super::simulate::simulate(
            &text,
            &message_box(ctx, input.face),
            ctx.metrics,
            ctx.inserts,
        ),
    }
}

/// Fit every description (or actor profile) of a database file.
pub fn fit_descriptions(
    file_name: &str,
    json: &Value,
    targets: &HashMap<String, Target>,
    settings: &FitSettings,
    overrides: &HashMap<String, StoredOverride>,
    ctx: &FitContext,
) -> Vec<FitItem> {
    let mut out = Vec::new();
    for description in descriptions_in(file_name, json) {
        let Some(target) = targets
            .get(&description.pointer)
            .filter(|t| !t.text.is_empty())
        else {
            continue;
        };
        let hash = source_hash(&[description.text.as_str()]);
        let choice = valid_choice(overrides, &description.pointer, &hash).cloned();
        let plain = description.item_like && ctx.plain_description_first_line;
        let fitted = if choice.as_ref().is_some_and(|c| c.disabled) {
            let mut fitted = fit_description(
                &target.text,
                plain,
                ctx,
                &FitSettings {
                    overflow: Overflow::Leave,
                    ..settings.clone()
                },
            );
            fitted.outcome = FitOutcome::Skipped {
                reason: "fitting turned off for this text".into(),
            };
            fitted.values = vec![None];
            fitted
        } else {
            let fitted = fit_description(
                &target.text,
                plain,
                ctx,
                &settings_for(settings, choice.as_ref()),
            );
            let output = fitted.values[0]
                .clone()
                .unwrap_or_else(|| target.text.clone());
            checked(fitted, &target.text, &output, &help_box(ctx), ctx)
        };
        out.push(FitItem {
            pointer: description.pointer.clone(),
            description: true,
            face: false,
            lines: vec![description.pointer],
            segment_ids: vec![Some(target.segment_id.clone())],
            texts: vec![target.text.clone()],
            source_hash: hash,
            choice,
            fitted,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engines::mv_mz::layout::font::Metrics;
    use crate::engines::mv_mz::layout::geometry::Geometry;
    use crate::engines::mv_mz::layout::text::Inserts;

    fn settings() -> FitSettings {
        FitSettings {
            line_breaks: LineBreaks::Reflow,
            overflow: Overflow::NewPage,
            min_font_size: 20,
            dialogue_font_size: None,
            strip_stored_font_prefix: false,
        }
    }

    fn map() -> Value {
        serde_json::json!({ "events": [null, { "pages": [{ "list": [
            { "code": 101, "parameters": ["", 0, 0, 2, ""] },
            { "code": 401, "parameters": ["長い一行目"] },
            { "code": 401, "parameters": ["……"] },
            { "code": 101, "parameters": ["", 0, 0, 2, ""] },
            { "code": 401, "parameters": ["未訳"] },
            { "code": 101, "parameters": ["", 0, 0, 2, ""] },
            { "code": 401, "parameters": ["短い"] }
        ]}]}]})
    }

    fn target(id: &str, text: &str) -> Target {
        Target {
            segment_id: id.into(),
            text: text.into(),
        }
    }

    fn run(overrides: &HashMap<String, StoredOverride>) -> Vec<FitItem> {
        let geometry = Geometry::from_system(&serde_json::json!({ "advanced": {} }));
        let inserts = Inserts::default();
        let ctx = FitContext {
            metrics: &Metrics::Estimated,
            geometry: &geometry,
            inserts: &inserts,
            no_space_join: false,
            plain_description_first_line: false,
        };
        let long = (0..20)
            .map(|i| format!("w{i:02}"))
            .collect::<Vec<_>>()
            .join(" ");
        let targets = HashMap::from([
            (
                "/events/1/pages/0/list/1/parameters/0".to_string(),
                target("s1", &long),
            ),
            (
                "/events/1/pages/0/list/4/parameters/0".to_string(),
                target("s2", ""),
            ),
            (
                "/events/1/pages/0/list/6/parameters/0".to_string(),
                target("s3", "Short."),
            ),
        ]);
        fit_messages(
            "Map001.json",
            &map(),
            &targets,
            &settings(),
            overrides,
            &ctx,
        )
    }

    #[test]
    fn fits_messages_and_skips_untranslated_ones() {
        let items = run(&HashMap::new());
        assert_eq!(items.len(), 3);
        assert_eq!(items[0].fitted.outcome, FitOutcome::Rewrapped);
        // The verbatim "……" line is never written.
        let writes: Vec<&str> = items[0].writes().map(|(p, _)| p).collect();
        assert_eq!(writes, vec!["/events/1/pages/0/list/1/parameters/0"]);
        assert_eq!(items[0].segment_ids, vec![Some("s1".into()), None]);
        assert!(matches!(
            items[1].fitted.outcome,
            FitOutcome::Skipped { .. }
        ));
        assert_eq!(items[1].writes().count(), 0);
        assert_eq!(items[2].fitted.outcome, FitOutcome::Fits);
    }

    #[test]
    fn overrides_apply_only_to_the_text_they_were_made_for() {
        let pointer = "/events/1/pages/0/list/0".to_string();
        let hash = source_hash(&["長い一行目", "……"]);
        let disabled = FitOverride {
            disabled: true,
            ..FitOverride::default()
        };
        let items = run(&HashMap::from([(
            pointer.clone(),
            StoredOverride {
                choice: disabled.clone(),
                source_hash: hash,
            },
        )]));
        assert!(matches!(
            items[0].fitted.outcome,
            FitOutcome::Skipped { .. }
        ));
        assert_eq!(items[0].writes().count(), 0);
        // A game update changed the text: the override is stale.
        let items = run(&HashMap::from([(
            pointer,
            StoredOverride {
                choice: disabled,
                source_hash: "old".into(),
            },
        )]));
        assert_eq!(items[0].fitted.outcome, FitOutcome::Rewrapped);
    }
}
