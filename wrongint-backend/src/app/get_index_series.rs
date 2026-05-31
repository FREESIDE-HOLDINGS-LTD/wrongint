use crate::app;
use crate::app::{GetIndexSeries, IndexCandles, IndexScope, Metrics, SnapshotRepository};
use crate::domain::{IndexOverTime, SourceId};
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
        let over_time = match v.scope() {
            IndexScope::Source(id) => {
                let snapshots = self.repository.in_range(id, v.from(), v.to())?;
                IndexOverTime::from_snapshots(v.from(), v.to(), snapshots)?
            }
            IndexScope::Global => {
                let mut sources: Vec<IndexOverTime> = Vec::new();
                for id in SourceId::all() {
                    let snapshots = self.repository.in_range(id, v.from(), v.to())?;
                    sources.push(IndexOverTime::from_snapshots(v.from(), v.to(), snapshots)?);
                }
                IndexOverTime::from_sources(sources)
            }
        };
        Ok(IndexCandles::new(v.scope(), over_time))
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
