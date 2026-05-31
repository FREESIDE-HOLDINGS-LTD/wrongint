use crate::app;
use crate::app::Transactor;
use crate::domain::time::DateTime;
use crate::domain::{
    ExternalUrl, Points, Post, PostComments, PostId, PostScore, PostTitle, Snapshot, Source,
    SourceId, UpvotesAndDownvotes,
};
use crate::errors::Result;
use anyhow::{Context, anyhow};
use redb::{ReadableDatabase, TableDefinition};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

const CAPTURES: TableDefinition<&[u8], &[u8]> = TableDefinition::new("captures");
const SOURCES: TableDefinition<&[u8], &[u8]> = TableDefinition::new("sources");

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

fn unix_from_key(key: &[u8]) -> Result<i64> {
    if key.len() < 9 {
        return Err(anyhow!("key too short").into());
    }
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&key[1..9]);
    Ok((u64::from_be_bytes(buf) ^ SIGN_FLIP) as i64)
}

#[derive(Serialize, Deserialize)]
enum PersistedScore {
    Points(i64),
    UpvotesAndDownvotes { upvotes: i64, downvotes: i64 },
}

impl From<PostScore> for PersistedScore {
    fn from(s: PostScore) -> Self {
        match s {
            PostScore::Points(p) => PersistedScore::Points(p.value()),
            PostScore::UpvotesAndDownvotes(v) => PersistedScore::UpvotesAndDownvotes {
                upvotes: v.upvotes(),
                downvotes: v.downvotes(),
            },
        }
    }
}

impl TryFrom<PersistedScore> for PostScore {
    type Error = crate::errors::Error;

    fn try_from(s: PersistedScore) -> std::result::Result<Self, Self::Error> {
        Ok(match s {
            PersistedScore::Points(v) => PostScore::Points(Points::new(v)?),
            PersistedScore::UpvotesAndDownvotes { upvotes, downvotes } => {
                PostScore::UpvotesAndDownvotes(UpvotesAndDownvotes::new(upvotes, downvotes)?)
            }
        })
    }
}

#[derive(Serialize, Deserialize)]
struct PersistedPost {
    source: u8,
    post_id: String,
    title: String,
    external_url: Option<String>,
    posted_at: i64,
    comments: i64,
    score: PersistedScore,
}

impl From<&Post> for PersistedPost {
    fn from(p: &Post) -> Self {
        PersistedPost {
            source: source_to_u8(p.source()),
            post_id: p.post_id().as_str().to_string(),
            title: p.title().as_str().to_string(),
            external_url: p.external_url().map(|u| u.as_str().to_string()),
            posted_at: p.posted_at().unix_timestamp(),
            comments: p.comments().value(),
            score: p.score().into(),
        }
    }
}

impl TryFrom<PersistedPost> for Post {
    type Error = crate::errors::Error;

    fn try_from(p: PersistedPost) -> std::result::Result<Self, Self::Error> {
        let external_url = p.external_url.map(ExternalUrl::new).transpose()?;
        Ok(Post::new(
            source_from_u8(p.source)?,
            PostId::new(p.post_id)?,
            PostTitle::new(p.title)?,
            external_url,
            DateTime::new_from_unix_timestamp(p.posted_at)?,
            PostComments::new(p.comments)?,
            p.score.try_into()?,
        ))
    }
}

#[derive(Serialize, Deserialize)]
struct PersistedSnapshot {
    source: u8,
    captured_at: i64,
    posts: Vec<PersistedPost>,
}

impl From<&Snapshot> for PersistedSnapshot {
    fn from(s: &Snapshot) -> Self {
        PersistedSnapshot {
            source: source_to_u8(s.source()),
            captured_at: s.captured_at().unix_timestamp(),
            posts: s.posts().iter().map(PersistedPost::from).collect(),
        }
    }
}

impl TryFrom<PersistedSnapshot> for Snapshot {
    type Error = crate::errors::Error;

    fn try_from(s: PersistedSnapshot) -> std::result::Result<Self, Self::Error> {
        let posts = s
            .posts
            .into_iter()
            .map(Post::try_from)
            .collect::<Result<Vec<_>>>()?;
        Snapshot::new(
            source_from_u8(s.source)?,
            DateTime::new_from_unix_timestamp(s.captured_at)?,
            posts,
        )
    }
}

#[derive(Serialize, Deserialize)]
struct PersistedSource {
    id: u8,
    last_snapshot: Option<PersistedSnapshot>,
    last_attempt_at: Option<i64>,
}

impl From<&Source> for PersistedSource {
    fn from(s: &Source) -> Self {
        PersistedSource {
            id: source_to_u8(s.id()),
            last_snapshot: s.last_snapshot().map(PersistedSnapshot::from),
            last_attempt_at: s.last_attempt_at().map(|t| t.unix_timestamp()),
        }
    }
}

impl TryFrom<PersistedSource> for Source {
    type Error = crate::errors::Error;

    fn try_from(s: PersistedSource) -> std::result::Result<Self, Self::Error> {
        let last_snapshot = s.last_snapshot.map(Snapshot::try_from).transpose()?;
        let last_attempt_at = s
            .last_attempt_at
            .map(DateTime::new_from_unix_timestamp)
            .transpose()?;
        Ok(Source::new(
            source_from_u8(s.id)?,
            last_snapshot,
            last_attempt_at,
        ))
    }
}

impl Database {
    fn read_source(&self, id: SourceId) -> Result<Source> {
        let key = vec![source_to_u8(id)];

        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(SOURCES) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => {
                return Ok(Source::new(id, None, None));
            }
            Err(e) => return Err(e.into()),
        };

        match table.get(key.as_slice())? {
            Some(value) => {
                let persisted: PersistedSource = serde_json::from_slice(value.value())?;
                persisted.try_into()
            }
            None => Ok(Source::new(id, None, None)),
        }
    }

    fn read_in_range(
        &self,
        source: SourceId,
        from: DateTime,
        to: DateTime,
    ) -> Result<Vec<Snapshot>> {
        let s = source_to_u8(source);
        let lower = time_prefix(s, from.unix_timestamp());
        let upper = time_prefix(s, to.unix_timestamp().saturating_add(1));
        self.snapshots_in_key_range(source, &lower, &upper)
    }

    /// Read the capture key range, grouping consecutive posts that share a
    /// `captured_at` (unix) into one [`Snapshot`]. Keys sort by (source, unix,
    /// post_id), so a single snapshot's posts are always contiguous.
    fn snapshots_in_key_range(
        &self,
        source: SourceId,
        lower: &[u8],
        upper: &[u8],
    ) -> Result<Vec<Snapshot>> {
        let read_txn = self.db.begin_read()?;
        let table = match read_txn.open_table(CAPTURES) {
            Ok(t) => t,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(Vec::new()),
            Err(e) => return Err(e.into()),
        };

        let mut snapshots: Vec<Snapshot> = Vec::new();
        let mut current_unix: Option<i64> = None;
        let mut posts: Vec<Post> = Vec::new();

        for row in table.range(lower..upper)? {
            let (key, value) = row?;
            let unix = unix_from_key(key.value())?;
            let persisted: PersistedPost = serde_json::from_slice(value.value())?;
            let post: Post = persisted.try_into()?;

            match current_unix {
                Some(u) if u == unix => posts.push(post),
                Some(u) => {
                    snapshots.push(Snapshot::new(
                        source,
                        DateTime::new_from_unix_timestamp(u)?,
                        std::mem::take(&mut posts),
                    )?);
                    current_unix = Some(unix);
                    posts.push(post);
                }
                None => {
                    current_unix = Some(unix);
                    posts.push(post);
                }
            }
        }

        if let Some(u) = current_unix {
            snapshots.push(Snapshot::new(
                source,
                DateTime::new_from_unix_timestamp(u)?,
                posts,
            )?);
        }

        Ok(snapshots)
    }
}

impl app::SnapshotRepository for Database {
    fn save(&self, snapshot: &Snapshot) -> Result<()> {
        self.execute(|uow| uow.snapshots().save(snapshot))
    }

    fn in_range(&self, source: SourceId, from: DateTime, to: DateTime) -> Result<Vec<Snapshot>> {
        self.read_in_range(source, from, to)
    }
}

impl app::SourceRepository for Database {
    fn get(&self, id: SourceId) -> Result<Source> {
        self.read_source(id)
    }

    fn save(&self, source: &Source) -> Result<()> {
        self.execute(|uow| uow.sources().save(source))
    }
}

impl app::Transactor for Database {
    fn execute<F, T>(&self, work: F) -> Result<T>
    where
        F: FnOnce(&dyn app::UnitOfWork) -> Result<T>,
    {
        let txn = self.db.begin_write()?;
        let result = work(&RedbUnitOfWork {
            db: self,
            txn: &txn,
        });
        match result {
            Ok(value) => {
                txn.commit()?;
                Ok(value)
            }
            Err(err) => Err(err),
        }
    }
}

struct RedbUnitOfWork<'a> {
    db: &'a Database,
    txn: &'a redb::WriteTransaction,
}

impl app::UnitOfWork for RedbUnitOfWork<'_> {
    fn sources(&self) -> &dyn app::SourceRepository {
        self
    }

    fn snapshots(&self) -> &dyn app::SnapshotRepository {
        self
    }
}

impl app::SnapshotRepository for RedbUnitOfWork<'_> {
    fn save(&self, snapshot: &Snapshot) -> Result<()> {
        let source = source_to_u8(snapshot.source());
        let unix = snapshot.captured_at().unix_timestamp();

        let mut captures = self.txn.open_table(CAPTURES)?;
        for post in snapshot.posts() {
            let key = capture_key(source, unix, post.post_id().as_str());
            let value = serde_json::to_vec(&PersistedPost::from(post))?;
            captures.insert(key.as_slice(), value.as_slice())?;
        }
        Ok(())
    }

    fn in_range(&self, source: SourceId, from: DateTime, to: DateTime) -> Result<Vec<Snapshot>> {
        self.db.read_in_range(source, from, to)
    }
}

impl app::SourceRepository for RedbUnitOfWork<'_> {
    fn get(&self, id: SourceId) -> Result<Source> {
        self.db.read_source(id)
    }

    fn save(&self, source: &Source) -> Result<()> {
        let key = vec![source_to_u8(source.id())];
        let value = serde_json::to_vec(&PersistedSource::from(source))?;

        let mut table = self.txn.open_table(SOURCES)?;
        table.insert(key.as_slice(), value.as_slice())?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{SnapshotRepository, SourceRepository, Transactor};

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
            PostTitle::new("t").unwrap(),
            Some(ExternalUrl::new("u").unwrap()),
            ts(0),
            PostComments::new(c).unwrap(),
            PostScore::Points(Points::new(s).unwrap()),
        )
    }

    fn snapshot(at: DateTime, posts: Vec<Post>) -> Snapshot {
        Snapshot::new(SourceId::HackerNews, at, posts).unwrap()
    }

    #[test]
    fn save_then_in_range_roundtrips() {
        let db = tmp_db();
        db.execute(|uow| {
            uow.snapshots()
                .save(&snapshot(ts(13), vec![post("a", 1, 2), post("b", 3, 4)]))?;
            uow.snapshots()
                .save(&snapshot(ts(15), vec![post("c", 5, 6)]))?;
            Ok(())
        })
        .unwrap();

        let all = db.in_range(SourceId::HackerNews, ts(0), ts(23)).unwrap();
        assert_eq!(all.len(), 2);
        assert_eq!(all[0].captured_at(), ts(13));
        assert_eq!(all[0].posts().len(), 2);
        assert_eq!(all[1].captured_at(), ts(15));
        assert_eq!(all[1].posts().len(), 1);

        let window = db.in_range(SourceId::HackerNews, ts(13), ts(13)).unwrap();
        assert_eq!(window.len(), 1);
        assert_eq!(window[0].captured_at(), ts(13));

        let lob = db.in_range(SourceId::Lobsters, ts(0), ts(23)).unwrap();
        assert!(lob.is_empty());
    }

    #[test]
    fn source_defaults_then_persists() {
        let db = tmp_db();
        let initial = SourceRepository::get(&db, SourceId::HackerNews).unwrap();
        assert!(initial.last_snapshot().is_none());
        assert!(initial.last_attempt_at().is_none());

        let attempt = ts(15);
        db.execute(|uow| {
            uow.sources().save(&Source::new(
                SourceId::HackerNews,
                Some(snapshot(ts(15), vec![post("c", 5, 6)])),
                Some(attempt),
            ))
        })
        .unwrap();

        let loaded = SourceRepository::get(&db, SourceId::HackerNews).unwrap();
        assert_eq!(loaded.last_attempt_at(), Some(attempt));
        let last_snapshot = loaded.last_snapshot().unwrap();
        assert_eq!(last_snapshot.captured_at(), ts(15));
        assert_eq!(last_snapshot.posts().len(), 1);

        assert!(
            SourceRepository::get(&db, SourceId::Lobsters)
                .unwrap()
                .last_snapshot()
                .is_none()
        );
    }

    #[test]
    fn execute_rolls_back_on_error() {
        let db = tmp_db();
        let result: Result<()> = db.execute(|uow| {
            uow.snapshots()
                .save(&snapshot(ts(13), vec![post("a", 1, 2)]))?;
            Err(anyhow!("boom").into())
        });
        assert!(result.is_err());

        let all = db.in_range(SourceId::HackerNews, ts(0), ts(23)).unwrap();
        assert!(all.is_empty());
    }
}
