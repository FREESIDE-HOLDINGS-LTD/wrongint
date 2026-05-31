use crate::app;
use crate::app::{Metrics, SnapshotTaker, SourceRepository, Transactor};
use crate::domain::SourceId;
use crate::domain::time::DateTime;
use crate::errors::{Error, Result};
use crate::record_application_handler_call;
use async_trait::async_trait;

#[derive(Clone)]
pub struct CaptureSnapshotsHandler<SR, T, S, M> {
    sources: SR,
    transactor: T,
    snapshot_taker: S,
    metrics: M,
}

impl<SR, T, S, M> CaptureSnapshotsHandler<SR, T, S, M>
where
    SR: SourceRepository + Send + Sync,
    T: Transactor + Send + Sync,
    S: SnapshotTaker + Send + Sync,
    M: Metrics + Send + Sync,
{
    pub fn new(sources: SR, transactor: T, snapshot_taker: S, metrics: M) -> Self {
        Self {
            sources,
            transactor,
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
        let taken_snapshot = self.snapshot_taker.take(id).await;

        let fetch_error = self.transactor.execute(move |uow| {
            let fetch_error: Option<Error> = match taken_snapshot {
                Ok(snapshot) => {
                    uow.snapshots().save(&snapshot)?;
                    source.record_capture(snapshot, attempted_at);
                    None
                }
                Err(err) => {
                    source.record_attempt(attempted_at);
                    Some(err)
                }
            };
            uow.sources().save(&source)?;
            Ok(fetch_error)
        })?;

        match fetch_error {
            Some(err) => Err(err),
            None => Ok(()),
        }
    }
}

#[async_trait]
impl<SR, T, S, M> app::CaptureSnapshotsHandler for CaptureSnapshotsHandler<SR, T, S, M>
where
    SR: SourceRepository + Send + Sync,
    T: Transactor + Send + Sync,
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
