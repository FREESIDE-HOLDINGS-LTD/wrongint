pub mod redb;
pub mod sources;

use crate::app;
use crate::app::{ApplicationHandlerCallResult, IndexScope};
use crate::config::Config;
use crate::domain::SnapshotIndex;
use crate::domain::time::Duration;
use crate::errors::Result;
use prometheus::{CounterVec, GaugeVec, HistogramOpts, HistogramVec, Opts, Registry, labels};
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

pub struct ConfigLoader {
    path: PathBuf,
}

impl ConfigLoader {
    pub fn new<P: Into<PathBuf>>(path: P) -> Self {
        Self { path: path.into() }
    }

    pub fn load(&self) -> Result<Config> {
        let content = fs::read_to_string(&self.path)?;
        let transport: TomlConfig = toml::from_str(&content)?;
        Config::try_from(transport)
    }
}

#[derive(Deserialize)]
struct TomlConfig {
    http_address: String,
    database_path: String,
    request_timeout_secs: u64,
    user_agent: String,
}

impl TryFrom<TomlConfig> for Config {
    type Error = crate::errors::Error;

    fn try_from(value: TomlConfig) -> std::result::Result<Self, Self::Error> {
        Config::new(
            value.http_address,
            value.database_path,
            value.request_timeout_secs,
            value.user_agent,
        )
    }
}

#[derive(Clone)]
pub struct Metrics {
    registry: Registry,

    metric_application_handler_calls_counter: CounterVec,
    metric_application_handler_calls_histogram: HistogramVec,
    metric_index: GaugeVec,
    metric_posts_captured: GaugeVec,
}

impl Metrics {
    pub fn new() -> Result<Self> {
        let registry = Registry::new_custom(Some("wrongint".into()), None)?;

        let metric_application_handler_calls_counter = CounterVec::new(
            Opts::new(
                "application_handler_calls_counter",
                "application handler calls counter",
            ),
            &["handler_name", "result"],
        )?;
        registry.register(Box::new(metric_application_handler_calls_counter.clone()))?;

        let metric_application_handler_calls_histogram = HistogramVec::new(
            HistogramOpts::new(
                "application_handler_calls_histogram",
                "application handler calls durations",
            ),
            &["handler_name", "result"],
        )?;
        registry.register(Box::new(metric_application_handler_calls_histogram.clone()))?;

        let metric_index = GaugeVec::new(
            Opts::new("index", "latest wrongint index (comments/score)"),
            &["source"],
        )?;
        registry.register(Box::new(metric_index.clone()))?;

        let metric_posts_captured = GaugeVec::new(
            Opts::new("posts_captured", "post count in the last snapshot"),
            &["source"],
        )?;
        registry.register(Box::new(metric_posts_captured.clone()))?;

        Ok(Self {
            registry,

            metric_application_handler_calls_counter,
            metric_application_handler_calls_histogram,
            metric_index,
            metric_posts_captured,
        })
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl app::Metrics for Metrics {
    fn record_application_handler_call(
        &self,
        handler_name: &str,
        result: ApplicationHandlerCallResult,
        duration: Duration,
    ) {
        let labels = labels! {
            "handler_name" => handler_name,
            "result" => match result {
                ApplicationHandlerCallResult::Ok => "ok",
                ApplicationHandlerCallResult::Error => "error",
            },
        };

        self.metric_application_handler_calls_counter
            .with(&labels)
            .inc();

        self.metric_application_handler_calls_histogram
            .with(&labels)
            .observe(duration.to_std().as_secs_f64());
    }

    fn record_index(&self, scope: IndexScope, index: Option<SnapshotIndex>, post_count: usize) {
        let scope = scope.to_string();
        self.metric_index
            .with(&labels! { "source" => scope.as_str() })
            .set(index.map(|i| i.value()).unwrap_or(f64::NAN));
        self.metric_posts_captured
            .with(&labels! { "source" => scope.as_str() })
            .set(post_count as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures;

    #[test]
    fn loads_config_from_file_successfully() -> Result<()> {
        let expected = Config::new(
            "0.0.0.0:8080",
            "local_database.redb",
            10,
            "wrongint/0.1 (+https://example.invalid)",
        )?;
        let loader = ConfigLoader::new(fixtures::test_file_path("local_config.toml"));
        let config = loader.load()?;
        assert_eq!(expected, config);
        Ok(())
    }
}
