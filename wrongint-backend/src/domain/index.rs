use crate::domain::time::{Date, DateTime, Duration};
use crate::domain::{Post, PostScore, Snapshot};
use crate::errors::Result;
use std::collections::BTreeMap;

const HOUR_SECONDS: u64 = 3600;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Index {
    value: f64,
}

impl Index {
    pub fn from_snapshot(snapshot: &Snapshot) -> Option<Index> {
        Self::from_posts(snapshot.posts())
    }

    pub fn from_posts<'a>(posts: impl IntoIterator<Item = &'a Post>) -> Option<Index> {
        let mut comments = 0i64;
        let mut score = 0i64;
        let mut any = false;
        for post in posts {
            any = true;
            comments += post.comments().value();
            score += match post.score() {
                PostScore::Points(points) => points.value(),
                PostScore::UpvotesAndDownvotes(votes) => votes.net(),
            };
        }
        if !any || score <= 0 {
            return None;
        }
        Some(Index {
            value: comments as f64 / score as f64,
        })
    }

    pub fn value(&self) -> f64 {
        self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ohlc {
    open: Index,
    high: Index,
    low: Index,
    close: Index,
}

impl Ohlc {
    pub fn from_samples(samples: &[Index]) -> Option<Ohlc> {
        let open = *samples.first()?;
        let close = *samples.last()?;
        let high = *samples
            .iter()
            .max_by(|a, b| a.value().total_cmp(&b.value()))?;
        let low = *samples
            .iter()
            .min_by(|a, b| a.value().total_cmp(&b.value()))?;
        Some(Ohlc {
            open,
            high,
            low,
            close,
        })
    }

    pub fn open(&self) -> Index {
        self.open
    }

    pub fn high(&self) -> Index {
        self.high
    }

    pub fn low(&self) -> Index {
        self.low
    }

    pub fn close(&self) -> Index {
        self.close
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IndexCandle {
    date: Date,
    hour: u32,
    ohlc: Option<Ohlc>,
}

impl IndexCandle {
    pub fn date(&self) -> Date {
        self.date
    }

    pub fn hour(&self) -> u32 {
        self.hour
    }

    pub fn ohlc(&self) -> Option<Ohlc> {
        self.ohlc
    }
}

#[derive(Debug, Clone)]
pub struct IndexOverTime {
    candles: Vec<IndexCandle>,
}

impl IndexOverTime {
    pub fn from_snapshots(
        from: DateTime,
        to: DateTime,
        mut snapshots: Vec<Snapshot>,
    ) -> Result<IndexOverTime> {
        snapshots.sort_by_key(|s| s.captured_at().unix_timestamp());

        let mut samples_by_hour: BTreeMap<i64, Vec<Index>> = BTreeMap::new();
        for snapshot in &snapshots {
            if let Some(index) = Index::from_snapshot(snapshot) {
                let hour = snapshot.captured_at().truncate_to_hour()?.unix_timestamp();
                samples_by_hour.entry(hour).or_default().push(index);
            }
        }

        let step = Duration::new_from_seconds(HOUR_SECONDS);
        let mut bucket = from.truncate_to_hour()?;
        let last = to.truncate_to_hour()?;

        let mut candles = Vec::new();
        while bucket <= last {
            let ohlc = samples_by_hour
                .get(&bucket.unix_timestamp())
                .and_then(|samples| Ohlc::from_samples(samples));
            candles.push(IndexCandle {
                date: bucket.date(),
                hour: bucket.hour_of_day(),
                ohlc,
            });
            bucket = bucket + step;
        }
        Ok(IndexOverTime { candles })
    }

    pub fn candles(&self) -> &[IndexCandle] {
        &self.candles
    }
}
