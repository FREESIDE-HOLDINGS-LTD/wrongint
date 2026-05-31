pub mod hn;
pub mod lobsters;

use crate::app;
use crate::domain::time::DateTime;
use crate::domain::{Snapshot, SourceId};
use crate::errors::Result;
use async_trait::async_trait;
use std::time::Duration;

pub use hn::HackerNews;
pub use lobsters::Lobsters;

pub fn new_client(user_agent: &str, timeout_secs: u64) -> Result<reqwest::Client> {
    let client = reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;
    Ok(client)
}

#[derive(Clone)]
pub struct Sources {
    hacker_news: HackerNews,
    lobsters: Lobsters,
}

impl Sources {
    pub fn new(hacker_news: HackerNews, lobsters: Lobsters) -> Self {
        Self {
            hacker_news,
            lobsters,
        }
    }
}

#[async_trait]
impl app::SnapshotTaker for Sources {
    fn sources(&self) -> Vec<SourceId> {
        vec![SourceId::HackerNews, SourceId::Lobsters]
    }

    async fn take(&self, source: SourceId) -> Result<Snapshot> {
        let posts = match source {
            SourceId::HackerNews => self.hacker_news.fetch().await?,
            SourceId::Lobsters => self.lobsters.fetch().await?,
        };
        Snapshot::new(source, DateTime::now(), posts)
    }
}
