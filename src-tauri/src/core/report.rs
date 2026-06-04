//! QA report generation — collect per-segment QA details and render to HTML.
//!
//! Errors are recalculated at export time (not stored in DB) so the report
//! is always fresh. Glossary terms are not applied (indicative report only).

use crate::core::qa::{self, QaError};
use crate::utils::text::escape_xml;
use serde::{Deserialize, Serialize};
use sqlx::SqlitePool;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QaSegmentDetail {
    pub segment_id: String,
    pub file_name: String,
    pub segment_number: u32,
    pub source_text: String,
    pub target_text: String,
    pub qa_score: u8,
    pub errors: Vec<QaError>,
}

// ---------------------------------------------------------------------------
// DB collection
// ---------------------------------------------------------------------------

/// Fetch all translated segments for `project_id`, recalculate QA errors,
/// and return only those with `score < 100`.
///
/// Uses `ROW_NUMBER()` (SQLite ≥ 3.25 — bundled libsqlite3-sys 0.30.1 → 3.46.x).
pub async fn collect_qa_details(
    pool: &SqlitePool,
    project_id: &str,
) -> Result<Vec<QaSegmentDetail>, sqlx::Error> {
    #[derive(sqlx::FromRow)]
    struct Row {
        segment_id: String,
        file_name: String,
        seg_num: i64,
        source_text: String,
        target_text: String,
    }

    let rows = sqlx::query_as::<_, Row>(
        "SELECT
            s.id AS segment_id,
            sf.file_name,
            ROW_NUMBER() OVER (PARTITION BY s.source_file_id ORDER BY s.rowid) AS seg_num,
            s.source_text,
            s.target_text
         FROM segments s
         JOIN source_files sf ON s.source_file_id = sf.id
         WHERE sf.project_id = ?
           AND s.status IN ('translated', 'reviewed', 'needs_review')
           AND s.target_text != ''
         ORDER BY sf.file_name, s.rowid",
    )
    .bind(project_id)
    .fetch_all(pool)
    .await?;

    let mut details = Vec::new();
    for row in rows {
        let result = qa::check(&row.source_text, &row.target_text, &[]);
        if result.score < 100 {
            details.push(QaSegmentDetail {
                segment_id: row.segment_id,
                file_name: row.file_name,
                segment_number: row.seg_num as u32,
                source_text: row.source_text,
                target_text: row.target_text,
                qa_score: result.score,
                errors: result.errors,
            });
        }
    }
    Ok(details)
}

// ---------------------------------------------------------------------------
// HTML generation
// ---------------------------------------------------------------------------

fn error_type_key(err: &QaError) -> &'static str {
    match err {
        QaError::MissingPlaceholder { .. } => "missing_placeholder",
        QaError::LineTooLong { .. } => "line_too_long",
        QaError::BomDetected => "bom_detected",
        QaError::GlossaryMismatch { .. } => "glossary_mismatch",
    }
}

struct Labels {
    title: &'static str,
    generated: &'static str,
    segments_with_errors: &'static str,
    checked: &'static str,
    missing_ph: &'static str,
    line_long: &'static str,
    bom: &'static str,
    glossary: &'static str,
    filter_file: &'static str,
    filter_all_files: &'static str,
    filter_score: &'static str,
    filter_all_errors: &'static str,
    filter_lt90: &'static str,
    filter_lt70: &'static str,
    col_file: &'static str,
    col_num: &'static str,
    col_source: &'static str,
    col_target: &'static str,
    col_score: &'static str,
    col_errors: &'static str,
    all_pass: &'static str,
    no_rows: &'static str,
    err_missing_ph: &'static str,
    err_line_long: &'static str,
    err_bom: &'static str,
    err_glossary: &'static str,
}

fn labels(lang: &str) -> Labels {
    if lang == "fr" {
        Labels {
            title: "Rapport QA",
            generated: "Généré le",
            segments_with_errors: "segments avec erreurs",
            checked: "vérifiés",
            missing_ph: "Placeholder manquant",
            line_long: "Ligne trop longue",
            bom: "BOM UTF-8",
            glossary: "Terme glossaire non respecté",
            filter_file: "Fichier :",
            filter_all_files: "Tous les fichiers",
            filter_score: "Score :",
            filter_all_errors: "Toutes les erreurs",
            filter_lt90: "Score &lt; 90",
            filter_lt70: "Score &lt; 70 (critique)",
            col_file: "Fichier",
            col_num: "#",
            col_source: "Source",
            col_target: "Cible",
            col_score: "Score",
            col_errors: "Erreurs",
            all_pass: "✅ Tous les segments passent le QA — aucune erreur détectée.",
            no_rows: "Aucune ligne ne correspond aux filtres actifs.",
            err_missing_ph: "Placeholder manquant",
            err_line_long: "Ligne trop longue",
            err_bom: "BOM UTF-8",
            err_glossary: "Glossaire",
        }
    } else {
        Labels {
            title: "QA Report",
            generated: "Generated",
            segments_with_errors: "segments with errors",
            checked: "checked",
            missing_ph: "Missing placeholder",
            line_long: "Line too long",
            bom: "BOM detected",
            glossary: "Glossary mismatch",
            filter_file: "File:",
            filter_all_files: "All files",
            filter_score: "Score:",
            filter_all_errors: "All errors",
            filter_lt90: "Score &lt; 90",
            filter_lt70: "Score &lt; 70 (critical)",
            col_file: "File",
            col_num: "#",
            col_source: "Source",
            col_target: "Target",
            col_score: "Score",
            col_errors: "Errors",
            all_pass: "✅ All segments pass QA — no errors found.",
            no_rows: "No rows match the current filters.",
            err_missing_ph: "Missing placeholder",
            err_line_long: "Line too long",
            err_bom: "BOM detected",
            err_glossary: "Glossary mismatch",
        }
    }
}

/// Format current wall-clock time as "YYYY-MM-DD HH:MM:SS UTC" without external crates.
fn unix_timestamp_to_date_str() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let sec = secs % 60;
    let min = (secs / 60) % 60;
    let hour = (secs / 3600) % 24;
    let days = secs / 86400;
    // Civil date from days since epoch (Gregorian)
    let z = days + 719_468;
    let era = z / 146_097;
    let doe = z % 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let mon = if mp < 10 { mp + 3 } else { mp - 9 };
    let yr = if mon <= 2 { y + 1 } else { y };
    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        yr, mon, d, hour, min, sec
    )
}

/// Generate a standalone HTML QA report.
///
/// `lang` is "en" or "fr". The output is a self-contained file with inline CSS
/// and JS — no external resources, no server required.
pub fn generate_qa_html(project_title: &str, details: &[QaSegmentDetail], lang: &str) -> String {
    use std::collections::HashSet;
    use std::fmt::Write as _;

    let lbl = labels(lang);
    let now = unix_timestamp_to_date_str();

    // Statistics
    let total_with_errors = details.len();
    let total_checked = details.len(); // only segments with errors are passed in

    let mut count_missing_ph: usize = 0;
    let mut count_line_long: usize = 0;
    let mut count_bom: usize = 0;
    let mut count_glossary: usize = 0;

    for d in details {
        for e in &d.errors {
            match e {
                QaError::MissingPlaceholder { .. } => count_missing_ph += 1,
                QaError::LineTooLong { .. } => count_line_long += 1,
                QaError::BomDetected => count_bom += 1,
                QaError::GlossaryMismatch { .. } => count_glossary += 1,
            }
        }
    }

    // Collect unique file names for the file filter dropdown
    let mut files_seen: Vec<&str> = Vec::new();
    let mut seen_set: HashSet<&str> = HashSet::new();
    for d in details {
        if seen_set.insert(d.file_name.as_str()) {
            files_seen.push(d.file_name.as_str());
        }
    }

    let mut out = String::new();
    let _ = write!(
        out,
        r#"<!DOCTYPE html>
<html lang="{lang}">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>{lbl_title} — {title_escaped}</title>
<style>
*{{box-sizing:border-box;margin:0;padding:0}}
body{{background:#1a1a1a;color:#e0e0e0;font-family:system-ui,sans-serif;font-size:13px;line-height:1.5;padding:24px}}
h1{{font-size:18px;font-weight:600;margin-bottom:4px}}
.meta{{color:#888;font-size:12px;margin-bottom:16px}}
.stats{{display:flex;gap:16px;flex-wrap:wrap;margin-bottom:20px;background:#222;border:1px solid #333;border-radius:6px;padding:12px 16px}}
.stat{{display:flex;flex-direction:column}}
.stat-label{{font-size:11px;color:#888;text-transform:uppercase;letter-spacing:.05em}}
.stat-value{{font-size:20px;font-weight:700}}
.stat-value.green{{color:#22c55e}}.stat-value.orange{{color:#f59e0b}}.stat-value.red{{color:#ef4444}}.stat-value.blue{{color:#60a5fa}}
.filters{{display:flex;gap:12px;flex-wrap:wrap;align-items:center;margin-bottom:16px;background:#222;border:1px solid #333;border-radius:6px;padding:10px 14px}}
.filters label{{display:flex;align-items:center;gap:6px;font-size:12px;color:#ccc;cursor:pointer}}
.filters select{{background:#2a2a2a;color:#e0e0e0;border:1px solid #444;border-radius:4px;padding:4px 8px;font-size:12px;cursor:pointer}}
.filters input[type=checkbox]{{accent-color:#60a5fa}}
.filter-group{{display:flex;align-items:center;gap:8px}}
.filter-sep{{width:1px;height:20px;background:#444;margin:0 4px}}
table{{width:100%;border-collapse:collapse;font-size:12px}}
thead th{{background:#2a2a2a;color:#aaa;text-align:left;padding:8px 10px;border-bottom:2px solid #333;font-weight:600;text-transform:uppercase;letter-spacing:.04em;font-size:11px}}
tbody tr{{border-bottom:1px solid #2a2a2a}}
tbody tr:nth-child(even){{background:#1e1e1e}}
tbody tr.score-ok{{}}
tbody tr.score-warn{{border-left:3px solid #f59e0b}}
tbody tr.score-crit{{border-left:3px solid #ef4444}}
td{{padding:7px 10px;vertical-align:top}}
.score-badge{{display:inline-block;padding:1px 6px;border-radius:3px;font-weight:700;font-size:11px}}
.score-badge.ok{{background:#22c55e22;color:#22c55e}}
.score-badge.warn{{background:#f59e0b22;color:#f59e0b}}
.score-badge.crit{{background:#ef444422;color:#ef4444}}
.err-list{{display:flex;flex-direction:column;gap:3px}}
.err-item{{display:flex;align-items:flex-start;gap:4px;font-size:11px;color:#ccc}}
.err-dot{{width:6px;height:6px;border-radius:50%;flex-shrink:0;margin-top:4px}}
.err-dot.missing_placeholder{{background:#ef4444}}
.err-dot.line_too_long{{background:#f59e0b}}
.err-dot.bom_detected{{background:#f59e0b}}
.err-dot.glossary_mismatch{{background:#a78bfa}}
.source-text,.target-text{{max-width:280px;word-break:break-word;white-space:pre-wrap;font-family:monospace;font-size:11px;color:#ddd}}
.source-text{{color:#94a3b8}}
.no-data{{text-align:center;padding:40px;color:#666;font-size:14px}}
.file-col{{color:#60a5fa;font-size:11px;max-width:120px;overflow:hidden;text-overflow:ellipsis;white-space:nowrap}}
#no-rows-msg{{display:none}}
</style>
</head>
<body>
<h1>{lbl_title} — {title_escaped}</h1>
<p class="meta">{lbl_generated}: {now} &nbsp;·&nbsp; {total_with_errors} {lbl_segments_with_errors} / {total_checked} {lbl_checked}</p>
"#,
        lang = escape_xml(lang),
        lbl_title = lbl.title,
        title_escaped = escape_xml(project_title),
        lbl_generated = lbl.generated,
        now = now,
        total_with_errors = total_with_errors,
        lbl_segments_with_errors = lbl.segments_with_errors,
        total_checked = total_checked,
        lbl_checked = lbl.checked,
    );

    // Stats block
    let _ = write!(
        out,
        r#"<div class="stats">
<div class="stat"><span class="stat-label">{lbl_missing_ph}</span><span class="stat-value red">{count_missing_ph}</span></div>
<div class="stat"><span class="stat-label">{lbl_line_long}</span><span class="stat-value orange">{count_line_long}</span></div>
<div class="stat"><span class="stat-label">{lbl_bom}</span><span class="stat-value orange">{count_bom}</span></div>
<div class="stat"><span class="stat-label">{lbl_glossary}</span><span class="stat-value blue">{count_glossary}</span></div>
</div>
"#,
        lbl_missing_ph = lbl.missing_ph,
        count_missing_ph = count_missing_ph,
        lbl_line_long = lbl.line_long,
        count_line_long = count_line_long,
        lbl_bom = lbl.bom,
        count_bom = count_bom,
        lbl_glossary = lbl.glossary,
        count_glossary = count_glossary,
    );

    if details.is_empty() {
        let _ = write!(out, r#"<p class="no-data">{}</p>"#, lbl.all_pass);
        let _ = write!(out, "\n</body>\n</html>");
        return out;
    }

    // Filters
    let _ = write!(
        out,
        r#"<div class="filters">
<div class="filter-group">
<span style="color:#888;font-size:12px">{lbl_filter_file}</span>
<select id="fileFilter">
<option value="all">{lbl_all_files}</option>
"#,
        lbl_filter_file = lbl.filter_file,
        lbl_all_files = lbl.filter_all_files,
    );
    for f in &files_seen {
        let _ = writeln!(
            out,
            "<option value=\"{escaped}\">{escaped}</option>",
            escaped = escape_xml(f)
        );
    }
    let _ = write!(
        out,
        r#"</select>
</div>
<div class="filter-sep"></div>
<div class="filter-group">
<span style="color:#888;font-size:12px">{lbl_filter_score}</span>
<select id="scoreFilter">
<option value="all">{lbl_filter_all_errors}</option>
<option value="lt90">{lbl_filter_lt90}</option>
<option value="lt70">{lbl_filter_lt70}</option>
</select>
</div>
<div class="filter-sep"></div>
<div class="filter-group" style="gap:12px">
<label><input type="checkbox" class="err-filter" value="missing_placeholder" checked> {err_missing_ph}</label>
<label><input type="checkbox" class="err-filter" value="line_too_long" checked> {err_line_long}</label>
<label><input type="checkbox" class="err-filter" value="bom_detected" checked> {err_bom}</label>
<label><input type="checkbox" class="err-filter" value="glossary_mismatch" checked> {err_glossary}</label>
</div>
</div>
"#,
        lbl_filter_score = lbl.filter_score,
        lbl_filter_all_errors = lbl.filter_all_errors,
        lbl_filter_lt90 = lbl.filter_lt90,
        lbl_filter_lt70 = lbl.filter_lt70,
        err_missing_ph = lbl.err_missing_ph,
        err_line_long = lbl.err_line_long,
        err_bom = lbl.err_bom,
        err_glossary = lbl.err_glossary,
    );

    // Table
    let _ = write!(
        out,
        r#"<p id="no-rows-msg" style="color:#666;padding:20px 0">{no_rows}</p>
<table id="qaTable">
<thead><tr>
<th>{col_file}</th><th>{col_num}</th><th>{col_source}</th>
<th>{col_target}</th><th>{col_score}</th><th>{col_errors}</th>
</tr></thead>
<tbody>
"#,
        no_rows = lbl.no_rows,
        col_file = lbl.col_file,
        col_num = lbl.col_num,
        col_source = lbl.col_source,
        col_target = lbl.col_target,
        col_score = lbl.col_score,
        col_errors = lbl.col_errors,
    );

    let use_fr = lang == "fr";
    for d in details {
        let score_class = if d.qa_score < 70 {
            "score-crit"
        } else {
            "score-warn"
        };
        let badge_class = if d.qa_score < 70 { "crit" } else { "warn" };

        // data-errors = comma-separated list of unique error type keys
        let mut error_types: Vec<&str> = d.errors.iter().map(error_type_key).collect();
        error_types.dedup();
        let data_errors = error_types.join(",");

        let _ = writeln!(
            out,
            "<tr class=\"{score_class}\" data-file=\"{file_escaped}\" data-score=\"{score}\" data-errors=\"{data_errors}\">",
            score_class = score_class,
            file_escaped = escape_xml(&d.file_name),
            score = d.qa_score,
            data_errors = escape_xml(&data_errors),
        );

        // File col
        let _ = writeln!(
            out,
            "<td class=\"file-col\" title=\"{file_escaped}\">{file_escaped}</td>",
            file_escaped = escape_xml(&d.file_name),
        );

        // Segment number
        let _ = writeln!(out, "<td>{}</td>", d.segment_number);

        // Source text
        let _ = writeln!(
            out,
            "<td><span class=\"source-text\">{}</span></td>",
            escape_xml(&d.source_text)
        );

        // Target text
        let _ = writeln!(
            out,
            "<td><span class=\"target-text\">{}</span></td>",
            escape_xml(&d.target_text)
        );

        // Score badge
        let _ = writeln!(
            out,
            "<td><span class=\"score-badge {badge_class}\">{score}</span></td>",
            badge_class = badge_class,
            score = d.qa_score,
        );

        // Errors list
        out.push_str("<td><div class=\"err-list\">\n");
        for err in &d.errors {
            let key = error_type_key(err);
            let label = escape_xml(&err.label(if use_fr { "fr" } else { "en" }));
            let _ = writeln!(
                out,
                "<div class=\"err-item\"><span class=\"err-dot {key}\"></span><span>{label}</span></div>",
                key = key,
                label = label,
            );
        }
        out.push_str("</div></td>\n</tr>\n");
    }

    out.push_str("</tbody>\n</table>\n");

    // Inline JS
    let _ = write!(
        out,
        r#"<script>
(function(){{
  function applyFilters(){{
    var file=document.getElementById('fileFilter').value;
    var score=document.getElementById('scoreFilter').value;
    var checked=[].slice.call(document.querySelectorAll('.err-filter:checked')).map(function(cb){{return cb.value;}});
    var rows=[].slice.call(document.querySelectorAll('#qaTable tbody tr'));
    var visible=0;
    rows.forEach(function(row){{
      var rowFile=row.getAttribute('data-file');
      var rowScore=parseInt(row.getAttribute('data-score'),10);
      var rowErrors=row.getAttribute('data-errors').split(',');
      var fileOk=(file==='all'||rowFile===file);
      var scoreOk=(score==='all'||(score==='lt90'&&rowScore<90)||(score==='lt70'&&rowScore<70));
      var errOk=checked.length===0||rowErrors.some(function(e){{return checked.indexOf(e)>=0;}});
      var show=fileOk&&scoreOk&&errOk;
      row.style.display=show?'':'none';
      if(show)visible++;
    }});
    document.getElementById('no-rows-msg').style.display=visible===0?'block':'none';
  }}
  document.getElementById('fileFilter').addEventListener('change',applyFilters);
  document.getElementById('scoreFilter').addEventListener('change',applyFilters);
  [].slice.call(document.querySelectorAll('.err-filter')).forEach(function(cb){{
    cb.addEventListener('change',applyFilters);
  }});
}})();
</script>
</body>
</html>
"#
    );

    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::qa::QaError;

    fn make_detail(
        file: &str,
        num: u32,
        source: &str,
        target: &str,
        score: u8,
        errors: Vec<QaError>,
    ) -> QaSegmentDetail {
        QaSegmentDetail {
            segment_id: "id-1".to_string(),
            file_name: file.to_string(),
            segment_number: num,
            source_text: source.to_string(),
            target_text: target.to_string(),
            qa_score: score,
            errors,
        }
    }

    #[test]
    fn test_escape_xml_special_chars() {
        assert_eq!(escape_xml("a & <b>"), "a &amp; &lt;b&gt;");
        assert_eq!(escape_xml("\"quoted\""), "&quot;quoted&quot;");
        assert_eq!(escape_xml("こんにちは"), "こんにちは");
    }

    #[test]
    fn test_generate_qa_html_structure() {
        let details = vec![
            make_detail(
                "Map001.json",
                1,
                r"\V[12] ゴールド",
                "coins",
                75,
                vec![QaError::MissingPlaceholder {
                    placeholder: r"\V[12]".to_string(),
                }],
            ),
            make_detail(
                "Map002.json",
                3,
                "こんにちは",
                &"A".repeat(56),
                90,
                vec![QaError::LineTooLong {
                    line: 1,
                    units: 56.0,
                    max_units: 55.38,
                    char_count: 56,
                }],
            ),
        ];

        let html = generate_qa_html("MyGame", &details, "en");

        assert!(html.contains("<html"), "missing <html");
        assert!(html.contains("<table"), "missing <table");
        assert!(html.contains("MyGame"), "missing project title");
        assert!(html.contains("Map001.json"), "missing file name");
        assert!(html.contains(r"\V[12]"), "missing placeholder text");
        assert!(html.contains("coins"), "missing target text");
        assert!(html.contains("75"), "missing score");
        assert!(html.contains("QA Report"), "missing report title");
        assert!(html.contains("applyFilters"), "missing JS filter");
        assert!(
            !html.contains("All segments pass"),
            "should not show all-pass msg"
        );
    }

    #[test]
    fn test_generate_qa_html_empty_details() {
        let html = generate_qa_html("EmptyProject", &[], "en");
        assert!(html.contains("All segments pass QA"));
        assert!(
            !html.contains("<table"),
            "should not emit table for empty details"
        );
    }

    #[test]
    fn test_generate_qa_html_fr_labels() {
        let details = vec![make_detail(
            "Map001.json",
            1,
            "source",
            "cible",
            85,
            vec![QaError::BomDetected],
        )];
        let html = generate_qa_html("MonJeu", &details, "fr");
        assert!(html.contains("Rapport QA"), "missing FR title");
        assert!(html.contains("Généré le"), "missing FR generated label");
        assert!(html.contains("BOM UTF-8"), "missing FR bom label");
    }
}
