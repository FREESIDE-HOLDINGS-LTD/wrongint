pub mod capture_snapshots;
pub mod get_index_series;
pub mod get_snapshot;
pub mod update_metrics;

use crate::domain::time::{DateTime, Duration};
use crate::domain::{Index, IndexOverTime, Snapshot, Source, SourceId};
use crate::errors::Result;
use async_trait::async_trait;

#[async_trait]
pub trait CaptureSnapshotsHandler {
    async fn handle(&self) -> Result<()>;
}

#[async_trait]
pub trait GetIndexSeriesHandler: Send + Sync {
    async fn handle(&self, v: &GetIndexSeries) -> Result<IndexCandles>;
}

#[async_trait]
pub trait UpdateMetricsHandler {
    async fn handle(&self) -> Result<()>;
}

#[async_trait]
pub trait GetSnapshotHandler: Send + Sync {
    async fn handle(&self, source: SourceId) -> Result<Option<Snapshot>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexScope {
    Global,
    Source(SourceId),
}

impl std::fmt::Display for IndexScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IndexScope::Global => write!(f, "all"),
            IndexScope::Source(id) => write!(f, "{id}"),
        }
    }
}

pub struct GetIndexSeries {
    scope: IndexScope,
    from: DateTime,
    to: DateTime,
}

impl GetIndexSeries {
    pub fn new(scope: IndexScope, from: DateTime, to: DateTime) -> Self {
        Self { scope, from, to }
    }

    pub fn scope(&self) -> IndexScope {
        self.scope
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

pub trait UnitOfWork {
    fn sources(&self) -> &dyn SourceRepository;
    fn snapshots(&self) -> &dyn SnapshotRepository;
}

pub trait Transactor {
    fn execute<F, T>(&self, work: F) -> Result<T>
    where
        F: FnOnce(&dyn UnitOfWork) -> Result<T>;
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

    fn record_index(&self, scope: IndexScope, index: Option<Index>, post_count: usize);
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

pub struct IndexCandles {
    scope: IndexScope,
    over_time: IndexOverTime,
}

impl IndexCandles {
    pub fn new(scope: IndexScope, over_time: IndexOverTime) -> Self {
        Self { scope, over_time }
    }

    pub fn scope(&self) -> IndexScope {
        self.scope
    }

    pub fn over_time(&self) -> &IndexOverTime {
        &self.over_time
    }
}

#[macro_export]
macro_rules! record_application_handler_call {
    ($metrics:expr, $handler_name:expr, $expr:expr) => {{
        let start = $crate::domain::time::DateTime::now();
        let result = $expr;
        let duration = $crate::domain::time::DateTime::now() - start;
        $metrics.record_application_handler_call($handler_name, (&result).into(), duration);
        log::debug!(
            "application handler {} {} in {}ms",
            $handler_name,
            if result.is_ok() { "ok" } else { "error" },
            duration.to_std().as_millis()
        );
        result
    }};
}
