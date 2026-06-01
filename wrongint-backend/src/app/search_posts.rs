use crate::app;
use crate::app::{Metrics, SnapshotRepository};
use crate::domain::time::DateTime;
use crate::domain::{Post, SourceId};
use crate::errors::Result;
use crate::record_application_handler_call;
use async_trait::async_trait;

#[derive(Clone)]
pub struct SearchPostsHandler<R, M> {
    repository: R,
    metrics: M,
}

impl<R, M> SearchPostsHandler<R, M>
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

    async fn handle_inner(&self, query: &str) -> Result<Option<Post>> {
        let needle = query.to_lowercase();
        // Whole-history scan: from the epoch to now, every source.
        let from = DateTime::new_from_unix_timestamp(0)?;
        let to = DateTime::now();

        let mut best: Option<Post> = None;
        for id in SourceId::all() {
            for snapshot in self.repository.in_range(id, from, to)? {
                for post in snapshot.posts() {
                    if !post.title().as_str().to_lowercase().contains(&needle) {
                        continue;
                    }
                    // Keep the most recently posted match. (The same post recurs
                    // across snapshots; posted_at is stable, so this dedupes too.)
                    if best
                        .as_ref()
                        .is_none_or(|b| post.posted_at() > b.posted_at())
                    {
                        best = Some(post.clone());
                    }
                }
            }
        }
        Ok(best)
    }
}

#[async_trait]
impl<R, M> app::SearchPostsHandler for SearchPostsHandler<R, M>
where
    R: SnapshotRepository + Send + Sync,
    M: Metrics + Send + Sync,
{
    async fn handle(&self, query: &str) -> Result<Option<Post>> {
        record_application_handler_call!(
            self.metrics,
            "search_posts",
            self.handle_inner(query).await
        )
    }
}
