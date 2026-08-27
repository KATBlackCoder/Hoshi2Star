use std::fs::{self, OpenOptions};
use std::io::Write;

use hoshi2star_lib::llm::pipeline;
use hoshi2star_lib::llm::provider::{
    LlmProvider, PromptContextPolicy, ProviderCallMetrics, TranslationContext,
};
use serde::Serialize;

use super::PilotWorkspace;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct Progress<'a> {
    variant: &'a str,
    completed_segments: i64,
    total_segments: i64,
    current_file: &'a str,
}

pub async fn run<P: LlmProvider>(workspace: &PilotWorkspace, provider: &P) -> Result<(), String> {
    let state = workspace.state();
    let files = sqlx::query_as::<_, (String, String)>(
        "SELECT id, file_name FROM source_files WHERE project_id = ? ORDER BY file_name",
    )
    .bind(&workspace.project_id)
    .fetch_all(&state.db)
    .await
    .map_err(|error| error.to_string())?;
    let total_segments: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM segments segment JOIN source_files file \
         ON file.id = segment.source_file_id WHERE file.project_id = ?",
    )
    .bind(&workspace.project_id)
    .fetch_one(&state.db)
    .await
    .map_err(|error| error.to_string())?;

    for (file_id, file_name) in files {
        let pairs = sqlx::query_as::<_, (String, String)>(
            "SELECT id, source_text FROM segments WHERE source_file_id = ? \
             AND (status = 'untranslated' OR target_text = '') ORDER BY rowid",
        )
        .bind(&file_id)
        .fetch_all(&state.db)
        .await
        .map_err(|error| error.to_string())?;
        if !pairs.is_empty() {
            let file_total = pairs.len();
            pipeline::run_inner(
                pairs,
                provider,
                TranslationContext {
                    source_lang: "ja".into(),
                    target_lang: "en".into(),
                    terminology_hints: Vec::new(),
                    engine: "mv_mz".into(),
                    batch_size: 20,
                    batch_delay_ms: 0,
                    prompt_context_policy: PromptContextPolicy::EngineOwned,
                    segment_contexts: Vec::new(),
                },
                &state.db,
                None,
                None,
                None,
                |done, total| {
                    eprintln!("[{}] {file_name}: {done}/{total}", workspace.variant);
                },
            )
            .await
            .map_err(|error| format!("{} / {file_name}: {error}", workspace.variant))?;
            append_metrics(&workspace.metrics_path(), &provider.drain_metrics())?;
            eprintln!(
                "[{}] completed {file_name} ({file_total} segments)",
                workspace.variant
            );
        }

        let completed_segments: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM segments segment JOIN source_files file \
             ON file.id = segment.source_file_id WHERE file.project_id = ? \
             AND segment.status != 'untranslated'",
        )
        .bind(&workspace.project_id)
        .fetch_one(&state.db)
        .await
        .map_err(|error| error.to_string())?;
        let progress = Progress {
            variant: &workspace.variant,
            completed_segments,
            total_segments,
            current_file: &file_name,
        };
        fs::write(
            workspace.root.join("progress.json"),
            serde_json::to_vec_pretty(&progress).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    }
    Ok(())
}

pub fn append_metrics(
    path: &std::path::Path,
    metrics: &[ProviderCallMetrics],
) -> Result<(), String> {
    if metrics.is_empty() {
        return Ok(());
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|error| error.to_string())?;
    for metric in metrics {
        serde_json::to_writer(&mut file, metric).map_err(|error| error.to_string())?;
        file.write_all(b"\n").map_err(|error| error.to_string())?;
    }
    Ok(())
}
