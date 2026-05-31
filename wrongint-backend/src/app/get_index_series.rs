use crate::app;
use crate::app::{GetIndexSeries, IndexPoint, IndexSeries, Metrics, SnapshotRepository};
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

    async fn handle_inner(&self, v: &GetIndexSeries) -> Result<IndexSeries> {
        let snapshots = self.repository.in_range(v.source(), v.from(), v.to())?;
        let points: Vec<IndexPoint> = snapshots.iter().map(IndexPoint::from).collect();
        Ok(IndexSeries::new(v.source(), points))
    }
}

#[async_trait]
impl<R, M> app::GetIndexSeriesHandler for GetIndexSeriesHandler<R, M>
where
    R: SnapshotRepository + Send + Sync,
    M: Metrics + Send + Sync,
{
    async fn handle(&self, v: &GetIndexSeries) -> Result<IndexSeries> {
        record_application_handler_call!(
            self.metrics,
            "get_index_series",
            self.handle_inner(v).await
        )
    }
}
