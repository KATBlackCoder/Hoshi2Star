//! Opt-in, resumable full StandGirl comparison.
//!
//! All game copies, SQLite databases and reports live below the explicit
//! `H2S_FULL_PILOT_ROOT`; none of them belong in Git.

#[path = "support/full_pilot/mod.rs"]
mod full_pilot;

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires StandGirl, H2S_FULL_PILOT_ROOT and local Ollama"]
async fn compare_full_standgirl_translation_with_and_without_terminology() {
    full_pilot::run().await.unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[ignore = "requires an existing full StandGirl pilot and local Ollama"]
async fn repair_standgirl_terminology_critical_segments() {
    full_pilot::retry_critical().await.unwrap();
}
