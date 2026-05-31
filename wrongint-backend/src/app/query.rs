use crate::app::Store;
use crate::domain::{self, PostCapture, SourceId, Ts};
use crate::errors::Result;
use std::collections::BTreeMap;
use std::sync::Arc;

pub struct QueryService {
    store: Arc<dyn Store>,
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
    pub t: Ts,
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
    pub name: String,
    pub current_score: Option<f64>,
    pub last_sample: Option<Ts>,
}

impl QueryService {
    pub fn new(store: Arc<dyn Store>) -> Self {
        Self { store }
    }

    pub fn scores(&self, sel: SourceSel, from: Ts, to: Ts, res: Resolution) -> Result<Series> {
        let posts = match sel {
            SourceSel::One(source) => self.store.captures_in_range(source, from, to)?,
            SourceSel::Global => {
                let mut all = Vec::new();
                for source in SourceId::all() {
                    all.extend(self.store.captures_in_range(*source, from, to)?);
                }
                all
            }
        };

        let label = match sel {
            SourceSel::One(s) => s.as_str().to_string(),
            SourceSel::Global => "global".to_string(),
        };

        Ok(Series {
            source: label,
            resolution: res,
            points: bucket_and_score(&posts, res),
        })
    }

    pub fn sources_overview(&self) -> Result<Vec<SourceStatus>> {
        let mut out = Vec::new();
        for source in SourceId::all() {
            let last_sample = self.store.latest_sample_ts(*source)?;
            let current_score = match last_sample {
                Some(ts) => {
                    let posts = self.store.captures_in_range(*source, ts, ts)?;
                    let (c, u) = domain::totals(&posts);
                    domain::wrongint_score(c, u)
                }
                None => None,
            };
            out.push(SourceStatus {
                id: *source,
                name: source.display_name().to_string(),
                current_score,
                last_sample,
            });
        }
        Ok(out)
    }
}

fn bucket_and_score(posts: &[PostCapture], res: Resolution) -> Vec<Point> {
    let mut buckets: BTreeMap<Ts, (i64, i64)> = BTreeMap::new();
    for p in posts {
        let key = bucket_key(p.sampled_at, res);
        let entry = buckets.entry(key).or_insert((0, 0));
        entry.0 += p.comments;
        entry.1 += p.upvotes;
    }

    buckets
        .into_iter()
        .map(|(t, (comments, upvotes))| Point {
            t,
            score: domain::wrongint_score(comments, upvotes),
            comments,
            upvotes,
        })
        .collect()
}

fn bucket_key(ts: Ts, res: Resolution) -> Ts {
    match res {
        Resolution::Hour => ts,
        Resolution::Day => ts.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Store;
    use chrono::TimeZone;
    use std::sync::Mutex;

    fn ts(h: u32, m: u32) -> Ts {
        chrono::Utc.with_ymd_and_hms(2026, 5, 31, h, m, 0).unwrap()
    }

    fn post(source: SourceId, c: i64, u: i64, at: Ts) -> PostCapture {
        PostCapture {
            source,
            post_id: format!("{c}-{u}"),
            title: "t".into(),
            url: "u".into(),
            comments: c,
            upvotes: u,
            sampled_at: at,
        }
    }

    #[derive(Default)]
    struct FakeStore {
        rows: Mutex<Vec<PostCapture>>,
    }

    impl Store for FakeStore {
        fn put_sample(&self, sample: &crate::domain::Sample) -> Result<()> {
            self.rows
                .lock()
                .unwrap()
                .extend(sample.posts.iter().cloned());
            Ok(())
        }
        fn captures_in_range(
            &self,
            source: SourceId,
            from: Ts,
            to: Ts,
        ) -> Result<Vec<PostCapture>> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .filter(|p| p.source == source && p.sampled_at >= from && p.sampled_at <= to)
                .cloned()
                .collect())
        }
        fn latest_sample_ts(&self, source: SourceId) -> Result<Option<Ts>> {
            Ok(self
                .rows
                .lock()
                .unwrap()
                .iter()
                .filter(|p| p.source == source)
                .map(|p| p.sampled_at)
                .max())
        }
    }

    #[test]
    fn hour_resolution_one_point_per_tick() {
        let store = Arc::new(FakeStore::default());
        store
            .put_sample(&crate::domain::Sample {
                source: SourceId::Hn,
                sampled_at: ts(13, 0),
                posts: vec![
                    post(SourceId::Hn, 4, 2, ts(13, 0)),
                    post(SourceId::Hn, 6, 3, ts(13, 0)),
                ],
            })
            .unwrap();
        store
            .put_sample(&crate::domain::Sample {
                source: SourceId::Hn,
                sampled_at: ts(14, 0),
                posts: vec![post(SourceId::Hn, 10, 0, ts(14, 0))],
            })
            .unwrap();

        let q = QueryService::new(store);
        let series = q
            .scores(
                SourceSel::One(SourceId::Hn),
                ts(0, 0),
                ts(23, 0),
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
            .put_sample(&crate::domain::Sample {
                source: SourceId::Hn,
                sampled_at: ts(13, 0),
                posts: vec![post(SourceId::Hn, 10, 5, ts(13, 0))],
            })
            .unwrap();
        store
            .put_sample(&crate::domain::Sample {
                source: SourceId::Lobsters,
                sampled_at: ts(13, 0),
                posts: vec![post(SourceId::Lobsters, 2, 5, ts(13, 0))],
            })
            .unwrap();

        let q = QueryService::new(store);
        let series = q
            .scores(SourceSel::Global, ts(0, 0), ts(23, 0), Resolution::Hour)
            .unwrap();
        assert_eq!(series.points.len(), 1);
        assert_eq!(series.points[0].score, Some(1.2));
    }

    #[test]
    fn day_resolution_pools_raw_posts() {
        let store = Arc::new(FakeStore::default());
        store
            .put_sample(&crate::domain::Sample {
                source: SourceId::Hn,
                sampled_at: ts(13, 0),
                posts: vec![post(SourceId::Hn, 10, 5, ts(13, 0))],
            })
            .unwrap();
        store
            .put_sample(&crate::domain::Sample {
                source: SourceId::Hn,
                sampled_at: ts(14, 0),
                posts: vec![post(SourceId::Hn, 2, 5, ts(14, 0))],
            })
            .unwrap();

        let q = QueryService::new(store);
        let series = q
            .scores(
                SourceSel::One(SourceId::Hn),
                ts(0, 0),
                ts(23, 0),
                Resolution::Day,
            )
            .unwrap();
        assert_eq!(series.points.len(), 1);
        assert_eq!(series.points[0].score, Some(1.2));
    }
}
