use crate::app::Store;
use crate::domain::time::DateTime;
use crate::domain::{self, Post, SourceId};
use crate::errors::Result;
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct QueryService {
    store: Arc<dyn Store>,
    sources: Vec<SourceId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSel {
    One(SourceId),
    Global,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    Hour,
    Day,
}

impl Resolution {
    pub fn as_str(&self) -> &'static str {
        match self {
            Resolution::Hour => "hour",
            Resolution::Day => "day",
        }
    }

    pub fn parse(s: &str) -> Option<Resolution> {
        match s {
            "hour" => Some(Resolution::Hour),
            "day" => Some(Resolution::Day),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Point {
    pub t: DateTime,
    pub score: Option<f64>,
    pub comments: i64,
    pub upvotes: i64,
}

#[derive(Debug, Clone)]
pub struct Series {
    pub source: String,
    pub resolution: Resolution,
    pub points: Vec<Point>,
}

#[derive(Debug, Clone)]
pub struct SourceStatus {
    pub id: SourceId,
    pub current_score: Option<f64>,
    pub last_sample: Option<DateTime>,
}

impl QueryService {
    pub fn new(store: Arc<dyn Store>, sources: Vec<SourceId>) -> Self {
        Self { store, sources }
    }

    pub fn scores(
        &self,
        sel: SourceSel,
        from: DateTime,
        to: DateTime,
        res: Resolution,
    ) -> Result<Series> {
        let captures = match sel {
            SourceSel::One(source) => self.store.captures_in_range(source, from, to)?,
            SourceSel::Global => {
                let mut all = Vec::new();
                for source in &self.sources {
                    all.extend(self.store.captures_in_range(*source, from, to)?);
                }
                all
            }
        };

        let label = match sel {
            SourceSel::One(s) => s.to_string(),
            SourceSel::Global => "global".to_string(),
        };

        Ok(Series {
            source: label,
            resolution: res,
            points: bucket_and_score(&captures, res)?,
        })
    }

    pub fn sources_overview(&self) -> Result<Vec<SourceStatus>> {
        let mut out = Vec::new();
        for source in &self.sources {
            let last_sample = self.store.latest_sample_ts(*source)?;
            let current_score = match last_sample {
                Some(ts) => {
                    let posts: Vec<Post> = self
                        .store
                        .captures_in_range(*source, ts, ts)?
                        .into_iter()
                        .map(|(_, p)| p)
                        .collect();
                    let (c, s) = domain::totals(&posts);
                    domain::wrongint_score(c, s)
                }
                None => None,
            };
            out.push(SourceStatus {
                id: *source,
                current_score,
                last_sample,
            });
        }
        Ok(out)
    }
}

fn bucket_and_score(captures: &[(DateTime, Post)], res: Resolution) -> Result<Vec<Point>> {
    let mut buckets: BTreeMap<DateTime, (i64, i64)> = BTreeMap::new();
    for (at, post) in captures {
        let key = bucket_key(*at, res)?;
        let entry = buckets.entry(key).or_insert((0, 0));
        entry.0 += post.comments().value();
        entry.1 += post.score().value();
    }

    Ok(buckets
        .into_iter()
        .map(|(t, (comments, upvotes))| Point {
            t,
            score: domain::wrongint_score(comments, upvotes),
            comments,
            upvotes,
        })
        .collect())
}

fn bucket_key(at: DateTime, res: Resolution) -> Result<DateTime> {
    match res {
        Resolution::Hour => Ok(at),
        Resolution::Day => at.truncate_to_day(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Store;
    use crate::domain::{NumberOfComments, PostId, Score, Snapshot, Title, Url};
    use std::sync::Mutex;

    fn ts(h: i64) -> DateTime {
        DateTime::new_from_unix_timestamp(1_780_000_000 + h * 3600).unwrap()
    }

    fn post(source: SourceId, c: i64, s: i64) -> Post {
        Post::new(
            source,
            PostId::new(format!("{c}-{s}")).unwrap(),
            Title::new("t"),
            Url::new("u"),
            NumberOfComments::new(c),
            Score::Points(s),
        )
    }

    #[derive(Default)]
    struct FakeStore {
        rows: Mutex<Vec<(DateTime, Post)>>,
    }

    impl Store for FakeStore {
        fn put_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
            let mut rows = self.rows.lock().unwrap();
            for post in snapshot.posts() {
                rows.push((snapshot.captured_at(), post.clone()));
            }
            Ok(())
        }
        fn captures_in_range(
            &self,
            source: SourceId,
            from: DateTime,
            to: DateTime,
        ) -> Result<Vec<(DateTime, Post)>> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .filter(|(at, p)| p.source() == source && *at >= from && *at <= to)
                .cloned()
                .collect())
        }
        fn latest_sample_ts(&self, source: SourceId) -> Result<Option<DateTime>> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .filter(|(_, p)| p.source() == source)
                .map(|(at, _)| *at)
                .max())
        }
    }

    fn snap(source: SourceId, at: DateTime, posts: Vec<Post>) -> Snapshot {
        Snapshot::new(source, at, posts).unwrap()
    }

    #[test]
    fn hour_resolution_one_point_per_tick() {
        let store = Arc::new(FakeStore::default());
        store
            .put_snapshot(&snap(
                SourceId::HackerNews,
                ts(13),
                vec![
                    post(SourceId::HackerNews, 4, 2),
                    post(SourceId::HackerNews, 6, 3),
                ],
            ))
            .unwrap();
        store
            .put_snapshot(&snap(
                SourceId::HackerNews,
                ts(14),
                vec![post(SourceId::HackerNews, 10, 0)],
            ))
            .unwrap();

        let q = QueryService::new(store, vec![SourceId::HackerNews, SourceId::Lobsters]);
        let series = q
            .scores(
                SourceSel::One(SourceId::HackerNews),
                ts(0),
                ts(23),
                Resolution::Hour,
            )
            .unwrap();
        assert_eq!(series.points.len(), 2);
        assert_eq!(series.points[0].score, Some(2.0));
        assert_eq!(series.points[1].score, None);
    }

    #[test]
    fn global_pools_across_sources_per_tick() {
        let store = Arc::new(FakeStore::default());
        store
            .put_snapshot(&snap(
                SourceId::HackerNews,
                ts(13),
                vec![post(SourceId::HackerNews, 10, 5)],
            ))
            .unwrap();
        store
            .put_snapshot(&snap(
                SourceId::Lobsters,
                ts(13),
                vec![post(SourceId::Lobsters, 2, 5)],
            ))
            .unwrap();

        let q = QueryService::new(store, vec![SourceId::HackerNews, SourceId::Lobsters]);
        let series = q
            .scores(SourceSel::Global, ts(0), ts(23), Resolution::Hour)
            .unwrap();
        assert_eq!(series.points.len(), 1);
        assert_eq!(series.points[0].score, Some(1.2));
    }

    #[test]
    fn day_resolution_pools_raw_posts() {
        let store = Arc::new(FakeStore::default());
        store
            .put_snapshot(&snap(
                SourceId::HackerNews,
                ts(13),
                vec![post(SourceId::HackerNews, 10, 5)],
            ))
            .unwrap();
        store
            .put_snapshot(&snap(
                SourceId::HackerNews,
                ts(14),
                vec![post(SourceId::HackerNews, 2, 5)],
            ))
            .unwrap();

        let q = QueryService::new(store, vec![SourceId::HackerNews, SourceId::Lobsters]);
        let series = q
            .scores(
                SourceSel::One(SourceId::HackerNews),
                ts(0),
                ts(23),
                Resolution::Day,
            )
            .unwrap();
        assert_eq!(series.points.len(), 1);
        assert_eq!(series.points[0].score, Some(1.2));
    }
}
