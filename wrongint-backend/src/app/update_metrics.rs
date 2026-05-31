use crate::app;
use crate::app::{IndexScope, Metrics, SourceRepository};
use crate::domain::{GlobalIndex, SourceId, SourceIndex};
use crate::errors::Result;
use crate::record_application_handler_call;
use async_trait::async_trait;

#[derive(Clone)]
pub struct UpdateMetricsHandler<SR, M> {
    sources: SR,
    metrics: M,
}

impl<SR, M> UpdateMetricsHandler<SR, M>
where
    SR: SourceRepository + Send + Sync,
    M: Metrics + Send + Sync,
{
    pub fn new(sources: SR, metrics: M) -> Self {
        Self { sources, metrics }
    }

    async fn handle_inner(&self) -> Result<()> {
        let mut sources: Vec<SourceIndex> = Vec::new();
        let mut global_posts = 0usize;

        for id in SourceId::all() {
            let source = self.sources.get(id)?;
            match source.last_snapshot() {
                Some(snapshot) => {
                    let at = snapshot.captured_at();
                    let count = snapshot.posts().len();
                    global_posts += count;
                    let source_index =
                        SourceIndex::from_snapshots(id, at, at, vec![snapshot.clone()])?;
                    self.metrics
                        .record_index(IndexScope::Source(id), source_index.latest(), count);
                    sources.push(source_index);
                }
                None => {
                    self.metrics.record_index(IndexScope::Source(id), None, 0);
                }
            }
        }

        let global = GlobalIndex::from_sources(sources);
        self.metrics
            .record_index(IndexScope::Global, global.latest(), global_posts);

        Ok(())
    }
}

#[async_trait]
impl<SR, M> app::UpdateMetricsHandler for UpdateMetricsHandler<SR, M>
where
    SR: SourceRepository + Send + Sync,
    M: Metrics + Send + Sync,
{
    async fn handle(&self) -> Result<()> {
        record_application_handler_call!(self.metrics, "update_metrics", self.handle_inner().await)
    }
}
