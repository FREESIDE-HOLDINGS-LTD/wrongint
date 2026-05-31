use crate::app;
use crate::domain::time::DateTime;
use crate::domain::{NumberOfComments, Post, PostId, Score, Snapshot, SourceId, Title, Url};
use crate::errors::Result;
use anyhow::{Context, anyhow};
use redb::{ReadableDatabase, TableDefinition};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const CAPTURES: TableDefinition<&[u8], &[u8]> = TableDefinition::new("captures");
const SAMPLES_META: TableDefinition<&[u8], &[u8]> = TableDefinition::new("samples_meta");

const SIGN_FLIP: u64 = 0x8000_0000_0000_0000;

fn source_to_u8(source: SourceId) -> u8 {
    match source {
        SourceId::HackerNews => 0,
        SourceId::Lobsters => 1,
    }
}

fn source_from_u8(v: u8) -> Result<SourceId> {
    match v {
        0 => Ok(SourceId::HackerNews),
        1 => Ok(SourceId::Lobsters),
        other => Err(anyhow!("unknown source byte {other}").into()),
    }
}

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

fn unix_from_key(key: &[u8]) -> Result<i64> {
    if key.len() < 9 {
        return Err(anyhow!("key too short").into());
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&key[1..9]);
    Ok((u64::from_be_bytes(buf) ^ SIGN_FLIP) as i64)
}

fn ts_from_unix(unix: i64) -> Result<DateTime> {
    DateTime::new_from_unix_timestamp(unix)
}

#[derive(Serialize, Deserialize)]
struct MetaValue {
    post_count: u64,
    ok: bool,
}

#[derive(Serialize, Deserialize)]
enum PersistedScore {
    Points(i64),
    UpvotesAndDownvotes(i64),
}

impl From<Score> for PersistedScore {
    fn from(s: Score) -> Self {
        match s {
            Score::Points(v) => PersistedScore::Points(v),
            Score::UpvotesAndDownvotes(v) => PersistedScore::UpvotesAndDownvotes(v),
        }
    }
}

impl From<PersistedScore> for Score {
    fn from(s: PersistedScore) -> Self {
        match s {
            PersistedScore::Points(v) => Score::Points(v),
            PersistedScore::UpvotesAndDownvotes(v) => Score::UpvotesAndDownvotes(v),
        }
    }
}

#[derive(Serialize, Deserialize)]
struct PersistedPost {
    source: u8,
    post_id: String,
    title: String,
    url: String,
    comments: i64,
    score: PersistedScore,
}

impl From<&Post> for PersistedPost {
    fn from(p: &Post) -> Self {
        PersistedPost {
            source: source_to_u8(p.source()),
            post_id: p.post_id().as_str().to_string(),
            title: p.title().as_str().to_string(),
            url: p.url().as_str().to_string(),
            comments: p.comments().value(),
            score: p.score().into(),
        }
    }
}

impl TryFrom<PersistedPost> for Post {
    type Error = crate::errors::Error;

    fn try_from(p: PersistedPost) -> std::result::Result<Self, Self::Error> {
        Ok(Post::new(
            source_from_u8(p.source)?,
            PostId::new(p.post_id)?,
            Title::new(p.title),
            Url::new(p.url),
            NumberOfComments::new(p.comments),
            p.score.into(),
        ))
    }
}

impl app::Store for Database {
    fn put_snapshot(&self, snapshot: &Snapshot) -> Result<()> {
        let source = source_to_u8(snapshot.source());
        let unix = snapshot.captured_at().unix_timestamp();

        let write_txn = self.db.begin_write()?;
        {
            let mut captures = write_txn.open_table(CAPTURES)?;
            for post in snapshot.posts() {
                let key = capture_key(source, unix, post.post_id().as_str());
                let value = serde_json::to_vec(&PersistedPost::from(post))?;
                captures.insert(key.as_slice(), value.as_slice())?;
            }

            let mut meta = write_txn.open_table(SAMPLES_META)?;
            let meta_value = MetaValue {
                post_count: snapshot.posts().len() as u64,
                ok: true,
            };
            let key = meta_key(source, unix);
            let value = serde_json::to_vec(&meta_value)?;
            meta.insert(key.as_slice(), value.as_slice())?;
        }
        write_txn.commit()?;
        Ok(())
    }

    fn captures_in_range(
        &self,
        source: SourceId,
        from: DateTime,
        to: DateTime,
    ) -> Result<Vec<(DateTime, Post)>> {
        let s = source_to_u8(source);
        let lower = capture_key(s, from.unix_timestamp(), "");
        let upper = time_prefix(s, to.unix_timestamp().saturating_add(1));

        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(CAPTURES) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };

        let mut out = Vec::new();
        for row in table.range(lower.as_slice()..upper.as_slice())? {
            let (key, value) = row?;
            let ts = ts_from_unix(unix_from_key(key.value())?)?;
            let persisted: PersistedPost = serde_json::from_slice(value.value())?;
            out.push((ts, persisted.try_into()?));
        }
        Ok(out)
    }

    fn latest_sample_ts(&self, source: SourceId) -> Result<Option<DateTime>> {
        let s = source_to_u8(source);
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
                let unix = unix_from_key(key.value())?;
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
    use crate::domain::{NumberOfComments, PostId, Score, SourceId, Title, Url};

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

    fn ts(h: i64) -> DateTime {
        DateTime::new_from_unix_timestamp(1_780_000_000 + h * 3600).unwrap()
    }

    fn post(id: &str, c: i64, s: i64) -> Post {
        Post::new(
            SourceId::HackerNews,
            PostId::new(id).unwrap(),
            Title::new("t"),
            Url::new("u"),
            NumberOfComments::new(c),
            Score::Points(s),
        )
    }

    fn snapshot(at: DateTime, posts: Vec<Post>) -> Snapshot {
        Snapshot::new(SourceId::HackerNews, at, posts).unwrap()
    }

    #[test]
    fn put_then_range_roundtrips() {
        let db = tmp_db();
        db.put_snapshot(&snapshot(ts(13), vec![post("a", 1, 2), post("b", 3, 4)]))
            .unwrap();
        db.put_snapshot(&snapshot(ts(15), vec![post("c", 5, 6)]))
            .unwrap();

        let all = db
            .captures_in_range(SourceId::HackerNews, ts(0), ts(23))
            .unwrap();
        assert_eq!(all.len(), 3);

        let window = db
            .captures_in_range(SourceId::HackerNews, ts(13), ts(13))
            .unwrap();
        assert_eq!(window.len(), 2);
        assert!(window.iter().all(|(t, _)| *t == ts(13)));

        let lob = db
            .captures_in_range(SourceId::Lobsters, ts(0), ts(23))
            .unwrap();
        assert!(lob.is_empty());
    }

    #[test]
    fn latest_sample_ts_returns_newest() {
        let db = tmp_db();
        assert_eq!(db.latest_sample_ts(SourceId::HackerNews).unwrap(), None);

        db.put_snapshot(&snapshot(ts(13), vec![post("a", 1, 2)]))
            .unwrap();
        db.put_snapshot(&snapshot(ts(15), vec![post("c", 5, 6)]))
            .unwrap();

        assert_eq!(
            db.latest_sample_ts(SourceId::HackerNews).unwrap(),
            Some(ts(15))
        );
        assert_eq!(db.latest_sample_ts(SourceId::Lobsters).unwrap(), None);
    }
}
