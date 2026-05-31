use crate::app::{Metrics, Source, Store};
use crate::domain::time::DateTime;
use crate::domain::{self, Post, Snapshot, SourceId};
use crate::errors::Result;
use futures::future::join_all;
use log::warn;
use std::fmt;
use std::sync::Arc;
use std::time::Duration;

pub struct IngestService {
    sources: Vec<Arc<dyn Source>>,
    store: Arc<dyn Store>,
    metrics: Arc<dyn Metrics>,
    retries: u32,
}

impl IngestService {
    pub fn new(
        sources: Vec<Arc<dyn Source>>,
        store: Arc<dyn Store>,
        metrics: Arc<dyn Metrics>,
        retries: u32,
    ) -> Self {
        Self {
            sources,
            store,
            metrics,
            retries,
        }
    }

    pub async fn ingest_tick(&self, tick: DateTime) -> TickReport {
        let fetches = self.sources.iter().map(|src| async move {
            let id = src.id();
            (id, fetch_with_retries(src.as_ref(), self.retries).await)
        });
        let results = join_all(fetches).await;

        let mut report = TickReport::default();
        let mut global_posts: Vec<Post> = Vec::new();

        for (id, result) in results {
            match result {
                Ok(posts) => match Snapshot::new(id, tick, posts.clone()) {
                    Ok(snapshot) => match self.store.put_snapshot(&snapshot) {
                        Ok(()) => {
                            self.metrics.record_fetch(id, true);
                            self.metrics.set_last_sample(id, tick);
                            self.metrics.set_posts_captured(id, posts.len());
                            let (c, s) = domain::totals(&posts);
                            self.metrics
                                .set_score(&id.to_string(), domain::wrongint_score(c, s));
                            global_posts.extend(posts.iter().cloned());
                            report.push(id, Ok(posts.len()));
                        }
                        Err(err) => {
                            warn!("store failed for {}: {err}", id);
                            self.metrics.record_fetch(id, false);
                            report.push(id, Err(()));
                        }
                    },
                    Err(err) => {
                        warn!("dropping snapshot for {}: {err}", id);
                        self.metrics.record_fetch(id, false);
                        report.push(id, Err(()));
                    }
                },
                Err(err) => {
                    warn!("fetch failed for {}: {err}", id);
                    self.metrics.record_fetch(id, false);
                    report.push(id, Err(()));
                }
            }
        }

        let (gc, gu) = domain::totals(&global_posts);
        self.metrics
            .set_score("global", domain::wrongint_score(gc, gu));

        report
    }
}

async fn fetch_with_retries(src: &dyn Source, retries: u32) -> Result<Vec<Post>> {
    let mut attempt = 0;
    loop {
        match src.fetch_front_page().await {
            Ok(posts) => return Ok(posts),
            Err(err) => {
                if attempt >= retries {
                    return Err(err);
                }
                attempt += 1;
                warn!(
                    "fetch attempt {attempt} for {} failed, retrying: {err}",
                    src.id()
                );
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}

#[derive(Default)]
pub struct TickReport {
    per_source: Vec<(SourceId, std::result::Result<usize, ()>)>,
}

impl TickReport {
    fn push(&mut self, source: SourceId, result: std::result::Result<usize, ()>) {
        self.per_source.push((source, result));
    }

    pub fn per_source(&self) -> &[(SourceId, std::result::Result<usize, ()>)] {
        &self.per_source
    }
}

impl fmt::Display for TickReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self
            .per_source
            .iter()
            .map(|(id, r)| match r {
                Ok(n) => format!("{}={n}", id),
                Err(()) => format!("{}=err", id),
            })
            .collect();
        write!(f, "[{}]", parts.join(" "))
    }
}
