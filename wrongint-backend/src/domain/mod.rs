use serde::{Deserialize, Serialize};

pub type Ts = chrono::DateTime<chrono::Utc>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceId {
    Hn,
    Lobsters,
}

impl SourceId {
    pub fn all() -> &'static [SourceId] {
        &[SourceId::Hn, SourceId::Lobsters]
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            SourceId::Hn => "hn",
            SourceId::Lobsters => "lobsters",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            SourceId::Hn => "Hacker News",
            SourceId::Lobsters => "lobste.rs",
        }
    }

    pub fn as_u8(&self) -> u8 {
        match self {
            SourceId::Hn => 0,
            SourceId::Lobsters => 1,
        }
    }

    pub fn from_u8(v: u8) -> Option<SourceId> {
        match v {
            0 => Some(SourceId::Hn),
            1 => Some(SourceId::Lobsters),
            _ => None,
        }
    }

    pub fn parse(s: &str) -> Option<SourceId> {
        match s {
            "hn" => Some(SourceId::Hn),
            "lobsters" => Some(SourceId::Lobsters),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PostCapture {
    pub source: SourceId,
    pub post_id: String,
    pub title: String,
    pub url: String,
    pub comments: i64,
    pub upvotes: i64,
    pub sampled_at: Ts,
}

#[derive(Debug, Clone)]
pub struct Sample {
    pub source: SourceId,
    pub sampled_at: Ts,
    pub posts: Vec<PostCapture>,
}

pub fn wrongint_score(comments: i64, upvotes: i64) -> Option<f64> {
    if upvotes <= 0 {
        return None;
    }
    Some(comments as f64 / upvotes as f64)
}

pub fn totals(posts: &[PostCapture]) -> (i64, i64) {
    posts
        .iter()
        .fold((0i64, 0i64), |(c, u), p| (c + p.comments, u + p.upvotes))
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
    fn source_id_roundtrips() {
        for s in SourceId::all() {
            assert_eq!(SourceId::from_u8(s.as_u8()), Some(*s));
            assert_eq!(SourceId::parse(s.as_str()), Some(*s));
        }
        assert_eq!(SourceId::parse("nope"), None);
        assert_eq!(SourceId::from_u8(9), None);
    }

    #[test]
    fn totals_sums() {
        let p = |c, u| PostCapture {
            source: SourceId::Hn,
            post_id: "x".into(),
            title: "t".into(),
            url: "u".into(),
            comments: c,
            upvotes: u,
            sampled_at: chrono::Utc::now(),
        };
        let posts = vec![p(3, 4), p(7, 6)];
        assert_eq!(totals(&posts), (10, 10));
    }
}
