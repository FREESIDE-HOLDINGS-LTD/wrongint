use crate::app;
use crate::app::{Metrics, SourceRepository};
use crate::domain::{Snapshot, SourceId};
use crate::errors::Result;
use crate::record_application_handler_call;
use async_trait::async_trait;

#[derive(Clone)]
pub struct GetSnapshotHandler<SR, M> {
    sources: SR,
    metrics: M,
}

impl<SR, M> GetSnapshotHandler<SR, M>
where
    SR: SourceRepository + Send + Sync,
    M: Metrics + Send + Sync,
{
    pub fn new(sources: SR, metrics: M) -> Self {
        Self { sources, metrics }
    }

    async fn handle_inner(&self, source: SourceId) -> Result<Option<Snapshot>> {
        Ok(self.sources.get(source)?.last_snapshot().cloned())
    }
}

#[async_trait]
impl<SR, M> app::GetSnapshotHandler for GetSnapshotHandler<SR, M>
where
    SR: SourceRepository + Send + Sync,
    M: Metrics + Send + Sync,
{
    async fn handle(&self, source: SourceId) -> Result<Option<Snapshot>> {
        record_application_handler_call!(
            self.metrics,
            "get_snapshot",
            self.handle_inner(source).await
        )
    }
}
