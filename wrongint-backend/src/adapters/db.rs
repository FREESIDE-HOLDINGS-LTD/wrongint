use crate::app;
use crate::domain::{PostCapture, Sample, SourceId, Ts};
use crate::errors::Result;
use anyhow::{Context, anyhow};
use redb::{ReadableDatabase, TableDefinition};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const CAPTURES: TableDefinition<&[u8], &[u8]> = TableDefinition::new("captures");
const SAMPLES_META: TableDefinition<&[u8], &[u8]> = TableDefinition::new("samples_meta");

const SIGN_FLIP: u64 = 0x8000_0000_0000_0000;

#[derive(Clone)]
pub struct Database {
    db: Arc<redb::Database>,
}

impl Database {
    pub fn new(path: impl Into<String>) -> Result<Self> {
        let db = redb::Database::create(path.into()).context("failed to open database")?;
        Ok(Self { db: Arc::new(db) })
    }
}

fn capture_key(source: u8, unix: i64, post_id: &str) -> Vec<u8> {
    let mut k = Vec::with_capacity(9 + post_id.len());
    k.push(source);
    k.extend_from_slice(&((unix as u64) ^ SIGN_FLIP).to_be_bytes());
    k.extend_from_slice(post_id.as_bytes());
    k
}

fn time_prefix(source: u8, unix: i64) -> Vec<u8> {
    let mut k = Vec::with_capacity(9);
    k.push(source);
    k.extend_from_slice(&((unix as u64) ^ SIGN_FLIP).to_be_bytes());
    k
}

fn meta_key(source: u8, unix: i64) -> Vec<u8> {
    time_prefix(source, unix)
}

fn unix_from_meta_key(key: &[u8]) -> Result<i64> {
    if key.len() < 9 {
        return Err(anyhow!("meta key too short").into());
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&key[1..9]);
    Ok((u64::from_be_bytes(buf) ^ SIGN_FLIP) as i64)
}

fn ts_from_unix(unix: i64) -> Result<Ts> {
    chrono::DateTime::from_timestamp(unix, 0).ok_or_else(|| anyhow!("bad unix timestamp").into())
}

#[derive(Serialize, Deserialize)]
struct MetaValue {
    post_count: u64,
    ok: bool,
}

impl app::Store for Database {
    fn put_sample(&self, sample: &Sample) -> Result<()> {
        let source = sample.source.as_u8();
        let unix = sample.sampled_at.timestamp();

        let write_txn = self.db.begin_write()?;
        {
            let mut captures = write_txn.open_table(CAPTURES)?;
            for post in &sample.posts {
                let key = capture_key(source, unix, &post.post_id);
                let value = serde_json::to_vec(post)?;
                captures.insert(key.as_slice(), value.as_slice())?;
            }

            let mut meta = write_txn.open_table(SAMPLES_META)?;
            let meta_value = MetaValue {
                post_count: sample.posts.len() as u64,
                ok: true,
            };
            let key = meta_key(source, unix);
            let value = serde_json::to_vec(&meta_value)?;
            meta.insert(key.as_slice(), value.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    fn captures_in_range(&self, source: SourceId, from: Ts, to: Ts) -> Result<Vec<PostCapture>> {
        let s = source.as_u8();
        let lower = capture_key(s, from.timestamp(), "");
        let upper = time_prefix(s, to.timestamp().saturating_add(1));

        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(CAPTURES) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };

        let mut out = Vec::new();
        for row in table.range(lower.as_slice()..upper.as_slice())? {
            let (_key, value) = row?;
            let post: PostCapture = serde_json::from_slice(value.value())?;
            out.push(post);
        }
        Ok(out)
    }

    fn latest_sample_ts(&self, source: SourceId) -> Result<Option<Ts>> {
        let s = source.as_u8();
        let lower = vec![s];
        let upper = vec![s + 1];

        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(SAMPLES_META) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(e) => return Err(e.into()),
        };

        let mut range = table.range(lower.as_slice()..upper.as_slice())?;
        match range.next_back() {
            Some(row) => {
                let (key, _value) = row?;
                let unix = unix_from_meta_key(key.value())?;
                Ok(Some(ts_from_unix(unix)?))
            }
            None => Ok(None),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::Store;
    use crate::domain::SourceId;
    use chrono::TimeZone;

    fn tmp_db() -> Database {
        let dir = std::env::temp_dir();
        let path = dir.join(format!("wrongint-test-{}.redb", uuid_like()));
        Database::new(path.to_string_lossy().to_string()).unwrap()
    }

    fn uuid_like() -> u128 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    }

    fn ts(h: u32) -> Ts {
        chrono::Utc.with_ymd_and_hms(2026, 5, 31, h, 0, 0).unwrap()
    }

    fn post(id: &str, c: i64, u: i64, at: Ts) -> PostCapture {
        PostCapture {
            source: SourceId::Hn,
            post_id: id.into(),
            title: "t".into(),
            url: "u".into(),
            comments: c,
            upvotes: u,
            sampled_at: at,
        }
    }

    #[test]
    fn put_then_range_roundtrips() {
        let db = tmp_db();
        db.put_sample(&Sample {
            source: SourceId::Hn,
            sampled_at: ts(13),
            posts: vec![post("a", 1, 2, ts(13)), post("b", 3, 4, ts(13))],
        })
        .unwrap();
        db.put_sample(&Sample {
            source: SourceId::Hn,
            sampled_at: ts(15),
            posts: vec![post("c", 5, 6, ts(15))],
        })
        .unwrap();

        let all = db.captures_in_range(SourceId::Hn, ts(0), ts(23)).unwrap();
        assert_eq!(all.len(), 3);

        let window = db.captures_in_range(SourceId::Hn, ts(13), ts(13)).unwrap();
        assert_eq!(window.len(), 2);

        let lob = db
            .captures_in_range(SourceId::Lobsters, ts(0), ts(23))
            .unwrap();
        assert!(lob.is_empty());
    }

    #[test]
    fn latest_sample_ts_returns_newest() {
        let db = tmp_db();
        assert_eq!(db.latest_sample_ts(SourceId::Hn).unwrap(), None);

        db.put_sample(&Sample {
            source: SourceId::Hn,
            sampled_at: ts(13),
            posts: vec![post("a", 1, 2, ts(13))],
        })
        .unwrap();
        db.put_sample(&Sample {
            source: SourceId::Hn,
            sampled_at: ts(15),
            posts: vec![post("c", 5, 6, ts(15))],
        })
        .unwrap();

        assert_eq!(db.latest_sample_ts(SourceId::Hn).unwrap(), Some(ts(15)));
        assert_eq!(db.latest_sample_ts(SourceId::Lobsters).unwrap(), None);
    }
}
