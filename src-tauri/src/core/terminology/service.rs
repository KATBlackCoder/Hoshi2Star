use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::{Mutex, Semaphore};

use super::analyzer::japanese::JapaneseAnalyzer;
use super::analyzer::MorphologicalAnalyzer;
use super::scanner::{self, ScanDone, ScanEvent, ScanEventSink, ScanSummary};
use super::{Result, TerminologyError};

#[derive(Clone)]
struct ActiveScan {
    scan_id: String,
    cancelled: Arc<AtomicBool>,
}

pub struct TerminologyService {
    analyzer: Arc<dyn MorphologicalAnalyzer>,
    scan_slots: Semaphore,
    active_by_project: Mutex<HashMap<String, ActiveScan>>,
}

impl TerminologyService {
    pub fn new(analyzer: Arc<dyn MorphologicalAnalyzer>, max_concurrent_scans: usize) -> Self {
        Self {
            analyzer,
            scan_slots: Semaphore::new(max_concurrent_scans.max(1)),
            active_by_project: Mutex::new(HashMap::new()),
        }
    }

    pub fn embedded_japanese() -> Result<Arc<Self>> {
        Ok(Arc::new(Self::new(
            Arc::new(JapaneseAnalyzer::new_embedded_ipadic()?),
            1,
        )))
    }

    pub async fn scan_project(
        &self,
        pool: &SqlitePool,
        project_id: &str,
        event_sink: Option<ScanEventSink>,
    ) -> Result<ScanSummary> {
        let scan_id = uuid::Uuid::new_v4().to_string();
        let cancelled = Arc::new(AtomicBool::new(false));
        {
            let mut active = self.active_by_project.lock().await;
            if active.contains_key(project_id) {
                return Err(TerminologyError::InvalidInput(format!(
                    "a terminology scan is already running for project `{project_id}`"
                )));
            }
            active.insert(
                project_id.to_string(),
                ActiveScan {
                    scan_id: scan_id.clone(),
                    cancelled: cancelled.clone(),
                },
            );
        }

        let result = match self.scan_slots.acquire().await {
            Ok(_permit) => {
                scanner::scan_project(
                    pool,
                    self.analyzer.clone(),
                    &scan_id,
                    project_id,
                    cancelled,
                    event_sink.clone(),
                )
                .await
            }
            Err(_) => Err(TerminologyError::Analyzer(
                "terminology scan scheduler is closed".to_string(),
            )),
        };
        self.active_by_project.lock().await.remove(project_id);
        if let Some(event_sink) = event_sink {
            let (status, error) = match &result {
                Ok(summary) => (summary.status.as_str().to_string(), None),
                Err(error) => ("failed".to_string(), Some(error.to_string())),
            };
            event_sink(ScanEvent::Done(ScanDone {
                scan_id,
                project_id: project_id.to_string(),
                status,
                error,
            }));
        }
        result
    }

    pub async fn cancel_scan(&self, scan_id: &str) -> bool {
        let active = self.active_by_project.lock().await;
        if let Some(scan) = active.values().find(|scan| scan.scan_id == scan_id) {
            scan.cancelled
                .store(true, std::sync::atomic::Ordering::Relaxed);
            true
        } else {
            false
        }
    }

    pub async fn active_scan_id(&self, project_id: &str) -> Option<String> {
        self.active_by_project
            .lock()
            .await
            .get(project_id)
            .map(|scan| scan.scan_id.clone())
    }
}
