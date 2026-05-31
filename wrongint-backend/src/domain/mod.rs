pub mod index;
pub mod time;

pub use index::{
    GlobalIndex, IndexCandle, IndexOverTime, Ohlc, PostIndex, SnapshotIndex, SourceIndex,
};

use crate::domain::time::{DateTime, Duration};
use crate::errors::Result;
use anyhow::anyhow;
use std::fmt;

pub const CAPTURE_INTERVAL_SECONDS: u64 = 10 * 60;
pub const RETRY_INTERVAL_SECONDS: u64 = 5 * 60;

#[derive(Debug, Clone)]
pub struct Source {
    id: SourceId,
    last_snapshot: Option<Snapshot>,
    last_attempt_at: Option<DateTime>,
}

impl Source {
    pub fn new(
        id: SourceId,
        last_snapshot: Option<Snapshot>,
        last_attempt_at: Option<DateTime>,
    ) -> Self {
        Self {
            id,
            last_snapshot,
            last_attempt_at,
        }
    }

    pub fn id(&self) -> SourceId {
        self.id
    }

    pub fn last_snapshot(&self) -> Option<&Snapshot> {
        self.last_snapshot.as_ref()
    }

    pub fn last_attempt_at(&self) -> Option<DateTime> {
        self.last_attempt_at
    }

    pub fn record_attempt(&mut self, at: DateTime) {
        self.last_attempt_at = Some(at);
    }

    pub fn record_capture(&mut self, snapshot: Snapshot, at: DateTime) {
        self.last_snapshot = Some(snapshot);
        self.last_attempt_at = Some(at);
    }

    pub fn should_capture_new_snapshot(&self, now: DateTime) -> bool {
        if let Some(snapshot) = &self.last_snapshot
            && now - snapshot.captured_at() < Duration::new_from_seconds(CAPTURE_INTERVAL_SECONDS)
        {
            return false;
        }
        if let Some(attempt) = self.last_attempt_at
            && now - attempt < Duration::new_from_seconds(RETRY_INTERVAL_SECONDS)
        {
            return false;
        }
        true
    }
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    source: SourceId,
    captured_at: DateTime,
    posts: Vec<Post>,
}

impl Snapshot {
    pub fn new(source: SourceId, captured_at: DateTime, posts: Vec<Post>) -> Result<Self> {
        if posts.is_empty() {
            return Err(anyhow!("snapshot for {source} has no posts").into());
        }
        Ok(Self {
            source,
            captured_at,
            posts,
        })
    }

    pub fn source(&self) -> SourceId {
        self.source
    }

    pub fn captured_at(&self) -> DateTime {
        self.captured_at
    }

    pub fn posts(&self) -> &[Post] {
        &self.posts
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Post {
    source: SourceId,
    post_id: PostId,
    title: PostTitle,
    url: PostUrl,
    posted_at: DateTime,
    comments: PostComments,
    score: PostScore,
}

impl Post {
    pub fn new(
        source: SourceId,
        post_id: PostId,
        title: PostTitle,
        url: PostUrl,
        posted_at: DateTime,
        comments: PostComments,
        score: PostScore,
    ) -> Self {
        Self {
            source,
            post_id,
            title,
            url,
            posted_at,
            comments,
            score,
        }
    }

    pub fn source(&self) -> SourceId {
        self.source
    }

    pub fn post_id(&self) -> &PostId {
        &self.post_id
    }

    pub fn title(&self) -> &PostTitle {
        &self.title
    }

    pub fn url(&self) -> &PostUrl {
        &self.url
    }

    pub fn posted_at(&self) -> DateTime {
        self.posted_at
    }

    pub fn comments(&self) -> PostComments {
        self.comments
    }

    pub fn score(&self) -> PostScore {
        self.score
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    HackerNews,
    Lobsters,
}

impl SourceId {
    pub fn all() -> [SourceId; 2] {
        [SourceId::HackerNews, SourceId::Lobsters]
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SourceId::HackerNews => "hackernews",
            SourceId::Lobsters => "lobsters",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PostId {
    value: String,
}

impl PostId {
    pub fn new(s: impl Into<String>) -> Result<Self> {
        let value = s.into();
        if value.is_empty() {
            return Err(anyhow!("post_id can't be empty").into());
        }
        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for PostId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PostTitle {
    value: String,
}

impl PostTitle {
    pub fn new(s: impl Into<String>) -> Result<Self> {
        let value = s.into();
        if value.is_empty() {
            return Err(anyhow!("post title can't be empty").into());
        }
        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for PostTitle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct PostUrl {
    value: String,
}

impl PostUrl {
    pub fn new(s: impl Into<String>) -> Result<Self> {
        let value = s.into();
        if value.is_empty() {
            return Err(anyhow!("post url can't be empty").into());
        }
        Ok(Self { value })
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for PostUrl {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostComments {
    value: i64,
}

impl PostComments {
    pub fn new(n: i64) -> Result<Self> {
        if n < 0 {
            return Err(anyhow!("post comments can't be negative").into());
        }
        Ok(Self { value: n })
    }

    pub fn value(&self) -> i64 {
        self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PostScore {
    Points(Points),
    UpvotesAndDownvotes(UpvotesAndDownvotes),
}

impl PostScore {
    pub fn net(&self) -> i64 {
        match self {
            PostScore::Points(points) => points.value(),
            PostScore::UpvotesAndDownvotes(votes) => votes.net(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Points {
    value: i64,
}

impl Points {
    pub fn new(value: i64) -> Result<Self> {
        if value < 0 {
            return Err(anyhow!("points can't be negative").into());
        }
        Ok(Self { value })
    }

    pub fn value(&self) -> i64 {
        self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UpvotesAndDownvotes {
    upvotes: i64,
    downvotes: i64,
}

impl UpvotesAndDownvotes {
    pub fn new(upvotes: i64, downvotes: i64) -> Result<Self> {
        if upvotes < 0 {
            return Err(anyhow!("upvotes can't be negative").into());
        }
        if downvotes < 0 {
            return Err(anyhow!("downvotes can't be negative").into());
        }
        Ok(Self { upvotes, downvotes })
    }

    pub fn upvotes(&self) -> i64 {
        self.upvotes
    }

    pub fn downvotes(&self) -> i64 {
        self.downvotes
    }

    pub fn net(&self) -> i64 {
        self.upvotes - self.downvotes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_post(c: i64, s: PostScore) -> Result<Post> {
        Ok(Post::new(
            SourceId::HackerNews,
            PostId::new("x")?,
            PostTitle::new("t")?,
            PostUrl::new("u")?,
            DateTime::now(),
            PostComments::new(c)?,
            s,
        ))
    }

    mod source {
        use super::super::*;

        fn at(unix: i64) -> Result<DateTime> {
            DateTime::new_from_unix_timestamp(unix)
        }

        fn snapshot_at(now: DateTime) -> Result<Snapshot> {
            Snapshot::new(
                SourceId::HackerNews,
                now,
                vec![super::make_post(1, PostScore::Points(Points::new(2)?))?],
            )
        }

        #[test]
        fn captures_when_never_attempted_or_captured() {
            let source = Source::new(SourceId::HackerNews, None, None);
            assert!(source.should_capture_new_snapshot(DateTime::now()));
        }

        #[test]
        fn skips_within_retry_backoff_after_failed_attempt() -> Result<()> {
            let source = Source::new(SourceId::HackerNews, None, Some(at(1_780_000_000)?));
            assert!(!source.should_capture_new_snapshot(at(1_780_000_200)?));
            Ok(())
        }

        #[test]
        fn retries_after_retry_backoff_elapsed() -> Result<()> {
            let source = Source::new(SourceId::HackerNews, None, Some(at(1_780_000_000)?));
            assert!(source.should_capture_new_snapshot(at(1_780_000_360)?));
            Ok(())
        }

        #[test]
        fn skips_within_capture_interval_after_success() -> Result<()> {
            let captured_at = at(1_780_000_000)?;
            let source = Source::new(
                SourceId::HackerNews,
                Some(snapshot_at(captured_at)?),
                Some(captured_at),
            );
            assert!(!source.should_capture_new_snapshot(at(1_780_000_400)?));
            Ok(())
        }

        #[test]
        fn captures_after_capture_interval_elapsed() -> Result<()> {
            let captured_at = at(1_780_000_000)?;
            let source = Source::new(
                SourceId::HackerNews,
                Some(snapshot_at(captured_at)?),
                Some(captured_at),
            );
            assert!(source.should_capture_new_snapshot(at(1_780_000_660)?));
            Ok(())
        }
    }

    mod snapshot {
        use super::super::*;

        #[test]
        fn rejects_empty() {
            assert!(Snapshot::new(SourceId::HackerNews, DateTime::now(), vec![]).is_err());
        }

        #[test]
        fn holds_posts() -> Result<()> {
            let at = DateTime::now();
            let snap = Snapshot::new(
                SourceId::Lobsters,
                at,
                vec![super::make_post(1, PostScore::Points(Points::new(2)?))?],
            )?;
            assert_eq!(snap.posts().len(), 1);
            assert_eq!(snap.source(), SourceId::Lobsters);
            assert_eq!(snap.captured_at(), at);
            Ok(())
        }
    }

    mod index {
        use super::super::*;

        #[test]
        fn none_when_score_nonpositive() -> Result<()> {
            let snap = Snapshot::new(
                SourceId::HackerNews,
                DateTime::now(),
                vec![super::make_post(10, PostScore::Points(Points::new(0)?))?],
            )?;
            assert_eq!(SnapshotIndex::from_snapshot(&snap), None);
            Ok(())
        }

        #[test]
        fn ratio_of_comments_over_score() -> Result<()> {
            let snap = Snapshot::new(
                SourceId::HackerNews,
                DateTime::now(),
                vec![
                    super::make_post(4, PostScore::Points(Points::new(2)?))?,
                    super::make_post(
                        6,
                        PostScore::UpvotesAndDownvotes(UpvotesAndDownvotes::new(4, 1)?),
                    )?,
                ],
            )?;
            assert_eq!(
                SnapshotIndex::from_snapshot(&snap).map(|i| i.value()),
                Some(2.0)
            );
            Ok(())
        }
    }

    mod post {
        use super::super::*;

        #[test]
        fn exposes_fields() -> Result<()> {
            let posted = DateTime::new_from_unix_timestamp(1_780_000_000)?;
            let post = Post::new(
                SourceId::Lobsters,
                PostId::new("abc")?,
                PostTitle::new("title")?,
                PostUrl::new("https://e.com/a")?,
                posted,
                PostComments::new(9)?,
                PostScore::UpvotesAndDownvotes(UpvotesAndDownvotes::new(12, 2)?),
            );
            assert_eq!(post.source(), SourceId::Lobsters);
            assert_eq!(post.post_id().as_str(), "abc");
            assert_eq!(post.title().as_str(), "title");
            assert_eq!(post.url().as_str(), "https://e.com/a");
            assert_eq!(post.posted_at(), posted);
            assert_eq!(post.comments().value(), 9);
            assert_eq!(
                post.score(),
                PostScore::UpvotesAndDownvotes(UpvotesAndDownvotes::new(12, 2)?)
            );
            Ok(())
        }

        #[test]
        fn builds_real_hackernews_post() -> Result<()> {
            let post = Post::new(
                SourceId::HackerNews,
                PostId::new("38901234")?,
                PostTitle::new("Show HN: A redb-backed time series for forum sentiment")?,
                PostUrl::new("https://news.ycombinator.com/item?id=38901234")?,
                DateTime::new_from_rfc3339("2024-01-02T15:04:05Z")?,
                PostComments::new(128)?,
                PostScore::Points(Points::new(342)?),
            );
            assert_eq!(post.source(), SourceId::HackerNews);
            assert_eq!(post.post_id().as_str(), "38901234");
            assert_eq!(
                post.url().as_str(),
                "https://news.ycombinator.com/item?id=38901234"
            );
            assert_eq!(post.score(), PostScore::Points(Points::new(342)?));
            Ok(())
        }

        #[test]
        fn builds_real_lobsters_post() -> Result<()> {
            let post = Post::new(
                SourceId::Lobsters,
                PostId::new("xq8dao")?,
                PostTitle::new("Hexagonal architecture in Rust, revisited")?,
                PostUrl::new("https://lobste.rs/s/xq8dao")?,
                DateTime::new_from_rfc3339("2024-01-02T09:30:00Z")?,
                PostComments::new(37)?,
                PostScore::UpvotesAndDownvotes(UpvotesAndDownvotes::new(54, 6)?),
            );
            assert_eq!(post.source(), SourceId::Lobsters);
            assert_eq!(post.post_id().as_str(), "xq8dao");
            assert_eq!(post.url().as_str(), "https://lobste.rs/s/xq8dao");
            match post.score() {
                PostScore::UpvotesAndDownvotes(v) => assert_eq!(v.net(), 48),
                _ => panic!("wrong variant"),
            }
            Ok(())
        }
    }

    mod source_id {
        use super::super::*;

        #[test]
        fn displays_as_slug() {
            assert_eq!(SourceId::HackerNews.to_string(), "hackernews");
            assert_eq!(SourceId::Lobsters.to_string(), "lobsters");
        }
    }

    mod post_id {
        use super::super::*;

        #[test]
        fn rejects_empty() {
            assert!(PostId::new("").is_err());
        }

        #[test]
        fn exposes_value() -> Result<()> {
            let id = PostId::new("7")?;
            assert_eq!(id.as_str(), "7");
            assert_eq!(id.to_string(), "7");
            Ok(())
        }
    }

    mod post_title {
        use super::super::*;

        #[test]
        fn rejects_empty() {
            assert!(PostTitle::new("").is_err());
        }

        #[test]
        fn exposes_value() -> Result<()> {
            let title = PostTitle::new("hello")?;
            assert_eq!(title.as_str(), "hello");
            assert_eq!(title.to_string(), "hello");
            Ok(())
        }
    }

    mod post_url {
        use super::super::*;

        #[test]
        fn rejects_empty() {
            assert!(PostUrl::new("").is_err());
        }

        #[test]
        fn exposes_value() -> Result<()> {
            let url = PostUrl::new("https://e.com")?;
            assert_eq!(url.as_str(), "https://e.com");
            assert_eq!(url.to_string(), "https://e.com");
            Ok(())
        }
    }

    mod post_comments {
        use super::super::*;

        #[test]
        fn rejects_negative() {
            assert!(PostComments::new(-1).is_err());
        }

        #[test]
        fn accepts_zero_and_positive() -> Result<()> {
            assert_eq!(PostComments::new(0)?.value(), 0);
            assert_eq!(PostComments::new(42)?.value(), 42);
            Ok(())
        }
    }

    mod post_score {
        use super::super::*;

        #[test]
        fn carries_points() -> Result<()> {
            assert_eq!(
                PostScore::Points(Points::new(5)?),
                PostScore::Points(Points::new(5)?)
            );
            Ok(())
        }

        #[test]
        fn carries_votes() -> Result<()> {
            let s = PostScore::UpvotesAndDownvotes(UpvotesAndDownvotes::new(8, 6)?);
            match s {
                PostScore::UpvotesAndDownvotes(v) => assert_eq!(v.net(), 2),
                _ => panic!("wrong variant"),
            }
            Ok(())
        }
    }

    mod points {
        use super::super::*;

        #[test]
        fn rejects_negative() {
            assert!(Points::new(-1).is_err());
        }

        #[test]
        fn carries_value() -> Result<()> {
            assert_eq!(Points::new(0)?.value(), 0);
            assert_eq!(Points::new(7)?.value(), 7);
            Ok(())
        }
    }

    mod upvotes_and_downvotes {
        use super::super::*;

        #[test]
        fn rejects_negative_counts() {
            assert!(UpvotesAndDownvotes::new(-1, 0).is_err());
            assert!(UpvotesAndDownvotes::new(0, -1).is_err());
        }

        #[test]
        fn exposes_counts() -> Result<()> {
            let v = UpvotesAndDownvotes::new(10, 3)?;
            assert_eq!(v.upvotes(), 10);
            assert_eq!(v.downvotes(), 3);
            Ok(())
        }

        #[test]
        fn net_is_upvotes_minus_downvotes() -> Result<()> {
            assert_eq!(UpvotesAndDownvotes::new(10, 3)?.net(), 7);
            assert_eq!(UpvotesAndDownvotes::new(2, 5)?.net(), -3);
            Ok(())
        }
    }
}
