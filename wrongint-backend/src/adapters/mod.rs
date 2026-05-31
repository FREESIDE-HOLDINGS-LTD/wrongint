pub mod db;
pub mod sources;

use crate::app;
use crate::config::Config;
use crate::domain::{SourceId, Ts};
use crate::errors::Result;
use prometheus::{CounterVec, GaugeVec, Opts, Registry, labels};
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
    sample_interval_secs: u64,
    request_timeout_secs: u64,
    request_retries: u32,
    hn_front_page_len: usize,
    user_agent: String,
}

impl TryFrom<TomlConfig> for Config {
    type Error = crate::errors::Error;

    fn try_from(value: TomlConfig) -> std::result::Result<Self, Self::Error> {
        Config::new(
            value.http_address,
            value.database_path,
            value.sample_interval_secs,
            value.request_timeout_secs,
            value.request_retries,
            value.hn_front_page_len,
            value.user_agent,
        )
    }
}

#[derive(Clone)]
pub struct Metrics {
    registry: Registry,
    score: GaugeVec,
    fetch_total: CounterVec,
    last_sample_timestamp: GaugeVec,
    posts_captured: GaugeVec,
}

impl Metrics {
    pub fn new() -> Result<Self> {
        let registry = Registry::new_custom(Some("wrongint".into()), None)?;

        let score = GaugeVec::new(
            Opts::new("score", "latest wrongint score (comments/upvotes)"),
            &["source"],
        )?;
        registry.register(Box::new(score.clone()))?;

        let fetch_total = CounterVec::new(
            Opts::new("fetch_total", "front-page fetch attempts by result"),
            &["source", "result"],
        )?;
        registry.register(Box::new(fetch_total.clone()))?;

        let last_sample_timestamp = GaugeVec::new(
            Opts::new(
                "last_sample_timestamp",
                "unix seconds of the last successful sample",
            ),
            &["source"],
        )?;
        registry.register(Box::new(last_sample_timestamp.clone()))?;

        let posts_captured = GaugeVec::new(
            Opts::new("posts_captured", "post count in the last sample"),
            &["source"],
        )?;
        registry.register(Box::new(posts_captured.clone()))?;

        Ok(Self {
            registry,
            score,
            fetch_total,
            last_sample_timestamp,
            posts_captured,
        })
    }

    pub fn registry(&self) -> &Registry {
        &self.registry
    }
}

impl app::Metrics for Metrics {
    fn record_fetch(&self, source: SourceId, ok: bool) {
        let result = if ok { "ok" } else { "err" };
        self.fetch_total
            .with(&labels! { "source" => source.as_str(), "result" => result })
            .inc();
    }

    fn set_score(&self, label: &str, score: Option<f64>) {
        self.score
            .with(&labels! { "source" => label })
            .set(score.unwrap_or(f64::NAN));
    }

    fn set_last_sample(&self, source: SourceId, ts: Ts) {
        self.last_sample_timestamp
            .with(&labels! { "source" => source.as_str() })
            .set(ts.timestamp() as f64);
    }

    fn set_posts_captured(&self, source: SourceId, count: usize) {
        self.posts_captured
            .with(&labels! { "source" => source.as_str() })
            .set(count as f64);
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
            60,
            10,
            2,
            30,
            "wrongint/0.1 (+https://example.invalid)",
        )?;
        let loader = ConfigLoader::new(fixtures::test_file_path("local_config.toml"));
        let config = loader.load()?;
        assert_eq!(expected, config);
        Ok(())
    }
}
