use crate::app;
use crate::app::{IndexScope, Metrics, SourceRepository};
use crate::domain::{Index, Snapshot, SourceId};
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
        let mut snapshots: Vec<Snapshot> = Vec::new();

        for id in SourceId::all() {
            let source = self.sources.get(id)?;
            match source.last_snapshot() {
                Some(snapshot) => {
                    let index = Index::from_snapshot(snapshot);
                    self.metrics.record_index(
                        IndexScope::Source(id),
                        index,
                        snapshot.posts().len(),
                    );
                    snapshots.push(snapshot.clone());
                }
                None => {
                    self.metrics.record_index(IndexScope::Source(id), None, 0);
                }
            }
        }

        let global_index = Index::from_posts(snapshots.iter().flat_map(Snapshot::posts));
        let global_posts = snapshots.iter().map(|s| s.posts().len()).sum();
        self.metrics
            .record_index(IndexScope::Global, global_index, global_posts);

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
