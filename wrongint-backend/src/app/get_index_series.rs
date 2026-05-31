use crate::app;
use crate::app::{GetIndexSeries, IndexCandles, IndexScope, Metrics, SnapshotRepository};
use crate::domain::{IndexOverTime, Snapshot, SourceId};
use crate::errors::Result;
use crate::record_application_handler_call;
use async_trait::async_trait;

#[derive(Clone)]
pub struct GetIndexSeriesHandler<R, M> {
    repository: R,
    metrics: M,
}

impl<R, M> GetIndexSeriesHandler<R, M>
where
    R: SnapshotRepository + Send + Sync,
    M: Metrics + Send + Sync,
{
    pub fn new(repository: R, metrics: M) -> Self {
        Self {
            repository,
            metrics,
        }
    }

    async fn handle_inner(&self, v: &GetIndexSeries) -> Result<IndexCandles> {
        let mut snapshots: Vec<Snapshot> = Vec::new();
        match v.scope() {
            IndexScope::Source(id) => {
                snapshots.extend(self.repository.in_range(id, v.from(), v.to())?);
            }
            IndexScope::Global => {
                for id in SourceId::all() {
                    snapshots.extend(self.repository.in_range(id, v.from(), v.to())?);
                }
            }
        }
        Ok(IndexCandles::new(
            v.scope(),
            IndexOverTime::from_snapshots(v.from(), v.to(), snapshots)?,
        ))
    }
}

#[async_trait]
impl<R, M> app::GetIndexSeriesHandler for GetIndexSeriesHandler<R, M>
where
    R: SnapshotRepository + Send + Sync,
    M: Metrics + Send + Sync,
{
    async fn handle(&self, v: &GetIndexSeries) -> Result<IndexCandles> {
        record_application_handler_call!(
            self.metrics,
            "get_index_series",
            self.handle_inner(v).await
        )
    }
}
