use crate::app;
use crate::app::{Metrics, SnapshotRepository, SnapshotTaker, SourceRepository};
use crate::domain::time::DateTime;
use crate::domain::SourceId;
use crate::errors::Result;
use crate::record_application_handler_call;
use async_trait::async_trait;

#[derive(Clone)]
pub struct CaptureSnapshotsHandler<SR, R, S, M> {
    sources: SR,
    snapshots: R,
    snapshot_taker: S,
    metrics: M,
}

impl<SR, R, S, M> CaptureSnapshotsHandler<SR, R, S, M>
where
    SR: SourceRepository + Send + Sync,
    R: SnapshotRepository + Send + Sync,
    S: SnapshotTaker + Send + Sync,
    M: Metrics + Send + Sync,
{
    pub fn new(sources: SR, snapshots: R, snapshot_taker: S, metrics: M) -> Self {
        Self {
            sources,
            snapshots,
            snapshot_taker,
            metrics,
        }
    }

    async fn handle_inner(&self) -> Result<()> {
        for id in self.snapshot_taker.sources() {
            if let Err(err) = self.capture(id).await {
                log::error!("failed to capture source {id}: {err}");
            }
        }
        Ok(())
    }

    async fn capture(&self, id: SourceId) -> Result<()> {
        let mut source = self.sources.get(id)?;
        if !source.should_capture_new_snapshot(DateTime::now()) {
            return Ok(());
        }

        let attempted_at = DateTime::now();
        let result = match self.snapshot_taker.take(id).await {
            Ok(snapshot) => {
                self.snapshots.save(&snapshot)?;
                source.record_capture(snapshot, attempted_at);
                Ok(())
            }
            Err(err) => {
                source.record_attempt(attempted_at);
                Err(err)
            }
        };
        self.sources.save(&source)?;
        result
    }
}

#[async_trait]
impl<SR, R, S, M> app::CaptureSnapshotsHandler for CaptureSnapshotsHandler<SR, R, S, M>
where
    SR: SourceRepository + Send + Sync,
    R: SnapshotRepository + Send + Sync,
    S: SnapshotTaker + Send + Sync,
    M: Metrics + Send + Sync,
{
    async fn handle(&self) -> Result<()> {
        record_application_handler_call!(
            self.metrics,
            "capture_snapshots",
            self.handle_inner().await
        )
    }
}
