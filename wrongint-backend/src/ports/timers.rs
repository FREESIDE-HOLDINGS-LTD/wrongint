use crate::app::IngestService;
use crate::domain::Ts;
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
            let tick = truncate_to_seconds(chrono::Utc::now());
            let report = self.ingest.ingest_tick(tick).await;
            info!("ingest tick {tick}: {report}");
        }
    }

    async fn align_to_boundary(&self) {
        let interval_secs = self.interval.as_secs() as i64;
        if interval_secs <= 1 {
            return;
        }
        let now = chrono::Utc::now().timestamp();
        let wait = interval_secs - (now % interval_secs);
        if wait != interval_secs {
            tokio::time::sleep(Duration::from_secs(wait as u64)).await;
        }
    }
}

fn truncate_to_seconds(ts: Ts) -> Ts {
    chrono::DateTime::from_timestamp(ts.timestamp(), 0).unwrap_or(ts)
}
