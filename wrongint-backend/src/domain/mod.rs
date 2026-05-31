pub mod time;

use crate::domain::time::DateTime;
use crate::errors::Result;
use anyhow::anyhow;
use std::fmt;

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
    title: Title,
    url: Url,
    comments: NumberOfComments,
    score: Score,
}

impl Post {
    pub fn new(
        source: SourceId,
        post_id: PostId,
        title: Title,
        url: Url,
        comments: NumberOfComments,
        score: Score,
    ) -> Self {
        Self {
            source,
            post_id,
            title,
            url,
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

    pub fn title(&self) -> &Title {
        &self.title
    }

    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn comments(&self) -> NumberOfComments {
        self.comments
    }

    pub fn score(&self) -> Score {
        self.score
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceId {
    HackerNews,
    Lobsters,
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
pub struct Title {
    value: String,
}

impl Title {
    pub fn new(s: impl Into<String>) -> Self {
        Self { value: s.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for Title {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Url {
    value: String,
}

impl Url {
    pub fn new(s: impl Into<String>) -> Self {
        Self { value: s.into() }
    }

    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for Url {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NumberOfComments {
    value: i64,
}

impl NumberOfComments {
    pub fn new(n: i64) -> Self {
        Self { value: n }
    }

    pub fn value(&self) -> i64 {
        self.value
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Score {
    Points(i64),
    UpvotesAndDownvotes(i64),
}

impl Score {
    pub fn value(&self) -> i64 {
        match self {
            Score::Points(v) => *v,
            Score::UpvotesAndDownvotes(v) => *v,
        }
    }
}

pub fn wrongint_score(comments: i64, score: i64) -> Option<f64> {
    if score <= 0 {
        return None;
    }
    Some(comments as f64 / score as f64)
}

pub fn totals(posts: &[Post]) -> (i64, i64) {
    posts.iter().fold((0i64, 0i64), |(c, s), p| {
        (c + p.comments.value(), s + p.score.value())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn score_basic() {
        assert_eq!(wrongint_score(10, 5), Some(2.0));
        assert_eq!(wrongint_score(0, 5), Some(0.0));
    }

    #[test]
    fn score_guards_nonpositive_denominator() {
        assert_eq!(wrongint_score(10, 0), None);
        assert_eq!(wrongint_score(10, -3), None);
    }

    #[test]
    fn source_id_display() {
        assert_eq!(SourceId::HackerNews.to_string(), "hackernews");
        assert_eq!(SourceId::Lobsters.to_string(), "lobsters");
    }

    fn make_post(c: i64, s: Score) -> Post {
        Post::new(
            SourceId::HackerNews,
            PostId::new("x").unwrap(),
            Title::new("t"),
            Url::new("u"),
            NumberOfComments::new(c),
            s,
        )
    }

    #[test]
    fn totals_sums() {
        let posts = vec![
            make_post(3, Score::Points(4)),
            make_post(7, Score::Points(6)),
        ];
        assert_eq!(totals(&posts), (10, 10));
    }

    #[test]
    fn snapshot_rejects_empty() {
        assert!(Snapshot::new(SourceId::HackerNews, DateTime::now(), vec![]).is_err());
    }

    #[test]
    fn snapshot_holds_posts() {
        let snap = Snapshot::new(
            SourceId::Lobsters,
            DateTime::now(),
            vec![make_post(1, Score::Points(2))],
        )
        .unwrap();
        assert_eq!(snap.posts().len(), 1);
        assert_eq!(snap.source(), SourceId::Lobsters);
    }

    #[test]
    fn post_id_rejects_empty() {
        assert!(PostId::new("").is_err());
        assert_eq!(PostId::new("7").unwrap().as_str(), "7");
    }

    #[test]
    fn score_value_ignores_variant() {
        assert_eq!(Score::Points(5).value(), 5);
        assert_eq!(Score::UpvotesAndDownvotes(-3).value(), -3);
    }

    #[test]
    fn unix_timestamp_roundtrips() {
        let t = DateTime::new_from_unix_timestamp(1_780_000_000).unwrap();
        assert_eq!(t.unix_timestamp(), 1_780_000_000);
    }
}
