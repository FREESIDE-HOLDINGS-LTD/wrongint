pub mod capture_snapshots;
pub mod get_index_series;

use crate::domain;
use crate::domain::time::{DateTime, Duration};
use crate::domain::{Index, Snapshot, Source, SourceId};
use crate::errors::Result;
use async_trait::async_trait;

#[async_trait]
pub trait CaptureSnapshotsHandler {
    async fn handle(&self) -> Result<()>;
}

#[async_trait]
pub trait GetIndexSeriesHandler: Send + Sync {
    async fn handle(&self, v: &GetIndexSeries) -> Result<IndexSeries>;
}

pub struct GetIndexSeries {
    source: SourceId,
    from: DateTime,
    to: DateTime,
}

impl GetIndexSeries {
    pub fn new(source: SourceId, from: DateTime, to: DateTime) -> Self {
        Self { source, from, to }
    }

    pub fn source(&self) -> SourceId {
        self.source
    }

    pub fn from(&self) -> DateTime {
        self.from
    }

    pub fn to(&self) -> DateTime {
        self.to
    }
}

pub trait SourceRepository {
    fn get(&self, id: SourceId) -> Result<Source>;
    fn save(&self, source: &Source) -> Result<()>;
}

pub trait SnapshotRepository {
    fn save(&self, snapshot: &Snapshot) -> Result<()>;
    fn in_range(&self, source: SourceId, from: DateTime, to: DateTime) -> Result<Vec<Snapshot>>;
}

#[async_trait]
pub trait SnapshotTaker: Send + Sync {
    fn sources(&self) -> Vec<SourceId>;
    async fn take(&self, source: SourceId) -> Result<Snapshot>;
}

pub trait Metrics {
    fn record_application_handler_call(
        &self,
        handler_name: &str,
        result: ApplicationHandlerCallResult,
        duration: Duration,
    );

    fn record_snapshot(&self, source: SourceId, index: Option<Index>, post_count: usize);
}

pub enum ApplicationHandlerCallResult {
    Ok,
    Error,
}

impl<T> From<&Result<T>> for ApplicationHandlerCallResult {
    fn from(result: &Result<T>) -> Self {
        match result {
            Ok(_) => ApplicationHandlerCallResult::Ok,
            Err(_) => ApplicationHandlerCallResult::Error,
        }
    }
}

pub struct IndexSeries {
    source: SourceId,
    points: Vec<IndexPoint>,
}

impl IndexSeries {
    pub fn new(source: SourceId, points: Vec<IndexPoint>) -> Self {
        Self { source, points }
    }

    pub fn source(&self) -> SourceId {
        self.source
    }

    pub fn points(&self) -> &[IndexPoint] {
        &self.points
    }
}

pub struct IndexPoint {
    captured_at: DateTime,
    index: Option<Index>,
}

impl IndexPoint {
    pub fn captured_at(&self) -> DateTime {
        self.captured_at
    }

    pub fn index(&self) -> Option<Index> {
        self.index
    }
}

impl From<&domain::Snapshot> for IndexPoint {
    fn from(snapshot: &domain::Snapshot) -> Self {
        Self {
            captured_at: snapshot.captured_at(),
            index: Index::from_snapshot(snapshot),
        }
    }
}

#[macro_export]
macro_rules! record_application_handler_call {
    ($metrics:expr, $handler_name:expr, $expr:expr) => {{
        let start = $crate::domain::time::DateTime::now();
        let result = $expr;
        $metrics.record_application_handler_call(
            $handler_name,
            (&result).into(),
            $crate::domain::time::DateTime::now() - start,
        );
        result
    }};
}
