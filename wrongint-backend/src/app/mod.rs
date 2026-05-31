pub mod ingest;
pub mod query;

use crate::domain::{PostCapture, Sample, SourceId, Ts};
use crate::errors::Result;
use async_trait::async_trait;

pub use ingest::{IngestService, TickReport};
pub use query::{Point, QueryService, Resolution, Series, SourceSel, SourceStatus};

pub trait Store: Send + Sync {
    fn put_sample(&self, sample: &Sample) -> Result<()>;
    fn captures_in_range(&self, source: SourceId, from: Ts, to: Ts) -> Result<Vec<PostCapture>>;
    fn latest_sample_ts(&self, source: SourceId) -> Result<Option<Ts>>;
}

#[async_trait]
pub trait Source: Send + Sync {
    fn id(&self) -> SourceId;
    async fn fetch_front_page(&self) -> Result<Vec<PostCapture>>;
}

pub trait Metrics: Send + Sync {
    fn record_fetch(&self, source: SourceId, ok: bool);
    fn set_score(&self, label: &str, score: Option<f64>);
    fn set_last_sample(&self, source: SourceId, ts: Ts);
    fn set_posts_captured(&self, source: SourceId, count: usize);
}
