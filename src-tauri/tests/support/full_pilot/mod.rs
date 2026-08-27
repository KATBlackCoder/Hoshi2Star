mod report;
mod terminology;
mod translation;
mod workspace;

use std::time::Duration;

use hoshi2star_lib::llm::provider::{
    LlmProvider, OpenAiCompatibleProvider, DEFAULT_OLLAMA_MODEL, DEFAULT_OLLAMA_URL,
};
use serde::{Deserialize, Serialize};

pub use workspace::PilotWorkspace;

#[derive(Debug, Clone)]
pub struct PilotConfig {
    pub source: std::path::PathBuf,
    pub root: std::path::PathBuf,
    pub ollama_url: String,
    pub model: String,
}

impl PilotConfig {
    fn from_env() -> Result<Self, String> {
        let source = std::env::var("H2S_STANDGIRL_PATH")
            .map(std::path::PathBuf::from)
            .map_err(|_| "H2S_STANDGIRL_PATH is required".to_string())?;
        let root = std::env::var("H2S_FULL_PILOT_ROOT")
            .map(std::path::PathBuf::from)
            .map_err(|_| "H2S_FULL_PILOT_ROOT is required".to_string())?;
        let root_text = root.to_string_lossy();
        if !root_text.starts_with("/tmp/hoshi2star-") {
            return Err("H2S_FULL_PILOT_ROOT must be an explicit /tmp/hoshi2star-* path".into());
        }
        if !source.is_dir() {
            return Err(format!(
                "StandGirl source does not exist: {}",
                source.display()
            ));
        }
        Ok(Self {
            source,
            root,
            ollama_url: std::env::var("H2S_OLLAMA_URL")
                .unwrap_or_else(|_| DEFAULT_OLLAMA_URL.to_string()),
            model: std::env::var("H2S_OLLAMA_MODEL")
                .unwrap_or_else(|_| DEFAULT_OLLAMA_MODEL.to_string()),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariantSummary {
    pub variant: String,
    pub total_segments: i64,
    pub translated: i64,
    pub needs_review: i64,
    pub untranslated: i64,
    pub qa_ok: i64,
    pub qa_errors: i64,
    pub qa_critical: i64,
    pub errors_by_type: std::collections::HashMap<String, usize>,
    pub terminology_issues: usize,
    pub inconsistent_repeated_sources: i64,
    pub provider_calls: usize,
    pub provider_attempts: u32,
    pub prompt_tokens: Option<u64>,
    pub completion_tokens: Option<u64>,
    pub total_tokens: Option<u64>,
    pub provider_duration_ms: u64,
    pub calls_with_hints: usize,
    pub terminology_hints: usize,
    pub export: String,
}

pub async fn run() -> Result<(), String> {
    let config = PilotConfig::from_env()?;
    std::fs::create_dir_all(&config.root).map_err(|error| error.to_string())?;

    let baseline = workspace::prepare(&config, "baseline").await?;
    let terminology_variant = workspace::prepare(&config, "terminology").await?;
    let source_fingerprint = baseline.source_fingerprint().await?;
    if source_fingerprint != terminology_variant.source_fingerprint().await? {
        return Err("baseline and terminology source fingerprints differ".into());
    }

    let term_provider = provider(&config);
    term_provider
        .health_check()
        .await
        .map_err(|error| error.to_string())?;
    terminology::prepare(&terminology_variant, &term_provider, &config.model).await?;

    let baseline_provider = provider(&config);
    baseline_provider
        .health_check()
        .await
        .map_err(|error| error.to_string())?;
    translation::run(&baseline, &baseline_provider).await?;

    let terminology_provider = provider(&config);
    terminology_provider
        .health_check()
        .await
        .map_err(|error| error.to_string())?;
    translation::run(&terminology_variant, &terminology_provider).await?;

    let baseline_summary = report::summarize(&baseline).await?;
    let terminology_summary = report::summarize(&terminology_variant).await?;
    report::write_comparison(
        &config,
        source_fingerprint,
        baseline_summary,
        terminology_summary,
    )?;
    Ok(())
}

pub async fn retry_critical() -> Result<(), String> {
    let config = PilotConfig::from_env()?;
    let terminology_variant = workspace::prepare(&config, "terminology").await?;
    let source_fingerprint = terminology_variant.source_fingerprint().await?;
    let retry_provider = provider(&config);
    retry_provider
        .health_check()
        .await
        .map_err(|error| error.to_string())?;
    let retry = translation::retry_needs_review(&terminology_variant, &retry_provider).await?;
    let after =
        report::summarize_named(&terminology_variant, "qa-details-after-retry.json").await?;
    report::write_retry_report(&config, source_fingerprint, retry, after)
}

fn provider(config: &PilotConfig) -> OpenAiCompatibleProvider {
    OpenAiCompatibleProvider::new_for_preset(
        "ollama",
        &config.ollama_url,
        &config.model,
        None,
        Duration::from_secs(180),
    )
}
