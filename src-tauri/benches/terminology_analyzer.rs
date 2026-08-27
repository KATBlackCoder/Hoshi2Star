use std::borrow::Cow;
use std::error::Error;
use std::hint::black_box;
use std::time::{Duration, Instant};

use lindera::dictionary::load_dictionary;
use lindera::mode::Mode;
use lindera::segmenter::Segmenter;
use serde::Deserialize;

const PILOT_SEGMENT_COUNT: usize = 1_191;
const WARM_RUNS: usize = 5;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FixtureCase {
    id: String,
    text: String,
}

fn rss_kib() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    let line = status.lines().find(|line| line.starts_with("VmRSS:"))?;
    line.split_whitespace().nth(1)?.parse().ok()
}

fn percentile(samples: &[Duration], percentile: f64) -> Duration {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let index = ((sorted.len() - 1) as f64 * percentile).ceil() as usize;
    sorted[index]
}

fn segment_corpus(segmenter: &Segmenter, corpus: &[String]) -> Result<usize, Box<dyn Error>> {
    let mut token_count = 0;
    for text in corpus {
        let mut tokens = segmenter.segment(Cow::Borrowed(black_box(text.as_str())))?;
        for token in &mut tokens {
            token_count += 1;
            black_box(token.details());
        }
    }
    Ok(token_count)
}

fn main() -> Result<(), Box<dyn Error>> {
    let fixture: Vec<FixtureCase> =
        serde_json::from_str(include_str!("../tests/fixtures/terminology/ja_tokens.json"))?;
    assert!(!fixture.is_empty(), "Japanese benchmark corpus is empty");

    let corpus: Vec<String> = (0..PILOT_SEGMENT_COUNT)
        .map(|index| fixture[index % fixture.len()].text.clone())
        .collect();
    let case_ids = fixture
        .iter()
        .map(|case| case.id.as_str())
        .collect::<Vec<_>>()
        .join(",");

    let rss_before = rss_kib();
    let init_started = Instant::now();
    let dictionary = load_dictionary("embedded://ipadic")?;
    let segmenter = Segmenter::new(Mode::Normal, dictionary, None);
    let init_elapsed = init_started.elapsed();
    let rss_after_init = rss_kib();

    let cold_started = Instant::now();
    let cold_tokens = segment_corpus(&segmenter, &corpus)?;
    let cold_elapsed = cold_started.elapsed();
    let rss_after_cold = rss_kib();

    let mut warm_samples = Vec::with_capacity(WARM_RUNS);
    let mut warm_tokens = 0;
    for _ in 0..WARM_RUNS {
        let started = Instant::now();
        warm_tokens = segment_corpus(&segmenter, &corpus)?;
        warm_samples.push(started.elapsed());
    }
    let rss_after_warm = rss_kib();

    let executable_size = std::env::current_exe()
        .ok()
        .and_then(|path| std::fs::metadata(path).ok())
        .map(|metadata| metadata.len());
    let rss_delta = rss_before
        .zip(rss_after_warm)
        .map(|(before, after)| after.saturating_sub(before));

    println!("analyzer_version={}", lindera::get_version());
    println!("dictionary=embedded://ipadic");
    println!("fixture_cases={}", fixture.len());
    println!("fixture_ids={case_ids}");
    println!("corpus_segments={}", corpus.len());
    println!("init_ms={:.3}", init_elapsed.as_secs_f64() * 1_000.0);
    println!("cold_ms={:.3}", cold_elapsed.as_secs_f64() * 1_000.0);
    println!("cold_tokens={cold_tokens}");
    println!(
        "warm_median_ms={:.3}",
        percentile(&warm_samples, 0.5).as_secs_f64() * 1_000.0
    );
    println!(
        "warm_p95_ms={:.3}",
        percentile(&warm_samples, 0.95).as_secs_f64() * 1_000.0
    );
    println!("warm_tokens={warm_tokens}");
    println!("rss_before_kib={}", rss_before.unwrap_or_default());
    println!("rss_after_init_kib={}", rss_after_init.unwrap_or_default());
    println!("rss_after_cold_kib={}", rss_after_cold.unwrap_or_default());
    println!("rss_after_warm_kib={}", rss_after_warm.unwrap_or_default());
    println!("rss_delta_kib={}", rss_delta.unwrap_or_default());
    println!(
        "bench_executable_bytes={}",
        executable_size.unwrap_or_default()
    );

    assert_eq!(cold_tokens, warm_tokens, "token count changed between runs");
    assert!(
        cold_elapsed < Duration::from_secs(5),
        "cold scan exceeds gate"
    );
    assert!(
        percentile(&warm_samples, 0.95) < Duration::from_secs(5),
        "warm p95 scan exceeds gate"
    );
    if let Some(delta) = rss_delta {
        assert!(delta <= 180 * 1_024, "embedded dictionary exceeds RSS gate");
    }

    Ok(())
}
