use crate::domain::time::{Date, DateTime, Duration};
use crate::domain::{Post, PostScore, Snapshot};
use crate::errors::Result;
use std::collections::BTreeMap;

const HOUR_SECONDS: u64 = 3600;

#[derive(Debug, Clone)]
pub struct IndexOverTime {
    candles: Vec<IndexCandle>,
}

impl IndexOverTime {
    pub fn from_snapshots(
        from: DateTime,
        to: DateTime,
        snapshots: Vec<Snapshot>,
    ) -> Result<IndexOverTime> {
        let ohlc_by_hour = Self::ohlc_by_hour(&Self::samples_by_hour(&snapshots)?);
        Self::fill(from, to, &ohlc_by_hour)
    }

    pub fn from_sources(sources: Vec<IndexOverTime>) -> IndexOverTime {
        let len = sources.iter().map(|s| s.candles.len()).max().unwrap_or(0);
        let mut candles = Vec::with_capacity(len);
        for i in 0..len {
            let at: Vec<IndexCandle> = sources
                .iter()
                .filter_map(|s| s.candles.get(i).copied())
                .collect();
            let Some(first) = at.first() else {
                continue;
            };
            let ohlcs: Vec<Ohlc> = at.iter().filter_map(|c| c.ohlc).collect();
            candles.push(IndexCandle {
                date: first.date,
                hour: first.hour,
                ohlc: Ohlc::mean(&ohlcs),
            });
        }
        IndexOverTime { candles }
    }

    pub fn candles(&self) -> &[IndexCandle] {
        &self.candles
    }

    fn samples_by_hour(snapshots: &[Snapshot]) -> Result<BTreeMap<i64, Vec<Index>>> {
        let mut sorted: Vec<&Snapshot> = snapshots.iter().collect();
        sorted.sort_by_key(|s| s.captured_at().unix_timestamp());

        let mut samples_by_hour: BTreeMap<i64, Vec<Index>> = BTreeMap::new();
        for snapshot in sorted {
            if let Some(index) = Index::from_snapshot(snapshot) {
                let hour = snapshot.captured_at().truncate_to_hour()?.unix_timestamp();
                samples_by_hour.entry(hour).or_default().push(index);
            }
        }
        Ok(samples_by_hour)
    }

    fn ohlc_by_hour(samples_by_hour: &BTreeMap<i64, Vec<Index>>) -> BTreeMap<i64, Ohlc> {
        samples_by_hour
            .iter()
            .filter_map(|(hour, samples)| Ohlc::from_samples(samples).map(|ohlc| (*hour, ohlc)))
            .collect()
    }

    fn fill(
        from: DateTime,
        to: DateTime,
        ohlc_by_hour: &BTreeMap<i64, Ohlc>,
    ) -> Result<IndexOverTime> {
        let step = Duration::new_from_seconds(HOUR_SECONDS);
        let mut bucket = from.truncate_to_hour()?;
        let last = to.truncate_to_hour()?;

        let mut candles = Vec::new();
        while bucket <= last {
            let ohlc = ohlc_by_hour.get(&bucket.unix_timestamp()).copied();
            candles.push(IndexCandle {
                date: bucket.date(),
                hour: bucket.hour_of_day(),
                ohlc,
            });
            bucket = bucket + step;
        }
        Ok(IndexOverTime { candles })
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

    pub fn mean(candles: &[Ohlc]) -> Option<Ohlc> {
        Some(Ohlc {
            open: Index::mean(candles.iter().map(|c| &c.open))?,
            high: Index::mean(candles.iter().map(|c| &c.high))?,
            low: Index::mean(candles.iter().map(|c| &c.low))?,
            close: Index::mean(candles.iter().map(|c| &c.close))?,
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

    pub fn mean<'a>(indexes: impl IntoIterator<Item = &'a Index>) -> Option<Index> {
        let mut sum = 0.0;
        let mut count = 0u32;
        for index in indexes {
            sum += index.value;
            count += 1;
        }
        if count == 0 {
            return None;
        }
        Some(Index {
            value: sum / count as f64,
        })
    }

    pub fn value(&self) -> f64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Points, PostComments, PostId, PostTitle, PostUrl, SourceId};

    const HOUR: i64 = 3600;

    fn dt(unix: i64) -> DateTime {
        DateTime::new_from_unix_timestamp(unix).unwrap()
    }

    fn post(comments: i64, score: i64) -> Post {
        Post::new(
            SourceId::HackerNews,
            PostId::new("x").unwrap(),
            PostTitle::new("t").unwrap(),
            PostUrl::new("u").unwrap(),
            DateTime::now(),
            PostComments::new(comments).unwrap(),
            PostScore::Points(Points::new(score).unwrap()),
        )
    }

    fn snap(unix: i64, comments: i64, score: i64) -> Snapshot {
        Snapshot::new(SourceId::HackerNews, dt(unix), vec![post(comments, score)]).unwrap()
    }

    fn index(comments: i64, score: i64) -> Index {
        Index::from_posts([&post(comments, score)]).unwrap()
    }

    #[test]
    fn index_mean_averages_values() {
        let a = index(2, 2);
        let b = index(6, 2);
        assert_eq!(Index::mean([&a, &b]).map(|i| i.value()), Some(2.0));
    }

    #[test]
    fn index_mean_none_when_empty() {
        let empty: [&Index; 0] = [];
        assert_eq!(Index::mean(empty), None);
    }

    #[test]
    fn ohlc_from_samples_picks_open_high_low_close() {
        let samples = [index(2, 2), index(6, 2), index(4, 2), index(3, 2)];
        let o = Ohlc::from_samples(&samples).unwrap();
        assert_eq!(o.open().value(), 1.0);
        assert_eq!(o.high().value(), 3.0);
        assert_eq!(o.low().value(), 1.0);
        assert_eq!(o.close().value(), 1.5);
    }

    #[test]
    fn ohlc_from_samples_none_when_empty() {
        assert_eq!(Ohlc::from_samples(&[]), None);
    }

    #[test]
    fn ohlc_mean_averages_each_field() {
        let a = Ohlc::from_samples(&[index(4, 2), index(8, 2), index(2, 2), index(6, 2)]).unwrap();
        let b =
            Ohlc::from_samples(&[index(8, 2), index(16, 2), index(6, 2), index(10, 2)]).unwrap();
        let m = Ohlc::mean(&[a, b]).unwrap();
        assert_eq!(m.open().value(), 3.0);
        assert_eq!(m.high().value(), 6.0);
        assert_eq!(m.low().value(), 2.0);
        assert_eq!(m.close().value(), 4.0);
    }

    #[test]
    fn ohlc_mean_none_when_empty() {
        assert_eq!(Ohlc::mean(&[]), None);
    }

    #[test]
    fn from_snapshots_bins_by_hour_and_fills_gaps() {
        let base = 1_780_000_000;
        let series = IndexOverTime::from_snapshots(
            dt(base),
            dt(base + HOUR),
            vec![snap(base, 4, 2), snap(base + 600, 2, 2)],
        )
        .unwrap();
        let candles = series.candles();
        assert_eq!(candles.len(), 2);

        let first = candles[0].ohlc().unwrap();
        assert_eq!(first.open().value(), 2.0);
        assert_eq!(first.close().value(), 1.0);
        assert_eq!(first.high().value(), 2.0);
        assert_eq!(first.low().value(), 1.0);

        assert_eq!(candles[1].ohlc(), None);
    }

    #[test]
    fn from_sources_means_candles_and_skips_absent_source() {
        let base = 1_780_000_000;
        let from = dt(base);
        let to = dt(base + 2 * HOUR);

        let hn = IndexOverTime::from_snapshots(
            from,
            to,
            vec![snap(base, 4, 2), snap(base + HOUR, 12, 2)],
        )
        .unwrap();
        let lobsters = IndexOverTime::from_snapshots(from, to, vec![snap(base, 8, 2)]).unwrap();

        let global = IndexOverTime::from_sources(vec![hn, lobsters]);
        let candles = global.candles();
        assert_eq!(candles.len(), 3);

        let hour0 = candles[0].ohlc().unwrap();
        assert_eq!(hour0.open().value(), 3.0);
        assert_eq!(hour0.close().value(), 3.0);

        let hour1 = candles[1].ohlc().unwrap();
        assert_eq!(hour1.open().value(), 6.0);

        assert_eq!(candles[2].ohlc(), None);
    }

    #[test]
    fn from_sources_empty_when_no_sources() {
        let global = IndexOverTime::from_sources(vec![]);
        assert!(global.candles().is_empty());
    }
}
