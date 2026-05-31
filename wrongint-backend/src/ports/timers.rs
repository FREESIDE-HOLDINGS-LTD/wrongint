use crate::app::IngestService;
use crate::domain::time::DateTime;
use log::info;
use std::sync::Arc;
use std::time::Duration;

pub struct Scheduler {
    ingest: Arc<IngestService>,
    interval: Duration,
}

impl Scheduler {
    pub fn new(ingest: Arc<IngestService>, interval: Duration) -> Self {
        Self { ingest, interval }
    }

    pub async fn run(&self) {
        self.align_to_boundary().await;

        let mut ticker = tokio::time::interval(self.interval);
        loop {
            ticker.tick().await;
            let tick = DateTime::now();
            let report = self.ingest.ingest_tick(tick).await;
            info!("ingest tick {tick}: {report}");
        }
    }

    async fn align_to_boundary(&self) {
        let interval_secs = self.interval.as_secs() as i64;
        if interval_secs <= 1 {
            return;
        }
        let now = DateTime::now().unix_timestamp();
        let wait = interval_secs - (now % interval_secs);
        if wait != interval_secs {
            tokio::time::sleep(Duration::from_secs(wait as u64)).await;
        }
    }
}
