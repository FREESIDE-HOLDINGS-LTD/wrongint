pub mod ingest;
pub mod query;

use crate::domain::time::DateTime;
use crate::domain::{Post, Snapshot, SourceId};
use crate::errors::Result;
use async_trait::async_trait;

pub use ingest::{IngestService, TickReport};
pub use query::{Point, QueryService, Resolution, Series, SourceSel, SourceStatus};

pub trait Store: Send + Sync {
    fn put_snapshot(&self, snapshot: &Snapshot) -> Result<()>;
    fn captures_in_range(
        &self,
        source: SourceId,
        from: DateTime,
        to: DateTime,
    ) -> Result<Vec<(DateTime, Post)>>;
    fn latest_sample_ts(&self, source: SourceId) -> Result<Option<DateTime>>;
}

#[async_trait]
pub trait Source: Send + Sync {
    fn id(&self) -> SourceId;
    async fn fetch_front_page(&self) -> Result<Vec<Post>>;
}

pub trait Metrics: Send + Sync {
    fn record_fetch(&self, source: SourceId, ok: bool);
    fn set_score(&self, label: &str, score: Option<f64>);
    fn set_last_sample(&self, source: SourceId, ts: DateTime);
    fn set_posts_captured(&self, source: SourceId, count: usize);
}
