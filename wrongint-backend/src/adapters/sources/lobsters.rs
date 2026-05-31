use crate::domain::time::DateTime;
use crate::domain::{
    ExternalUrl, Post, PostComments, PostId, PostScore, PostTitle, SourceId, UpvotesAndDownvotes,
};
use crate::errors::Result;
use serde::Deserialize;

const HOTTEST_URL: &str = "https://lobste.rs/hottest.json";

#[derive(Clone)]
pub struct Lobsters {
    client: reqwest::Client,
}

impl Lobsters {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    pub async fn fetch(&self) -> Result<Vec<Post>> {
        let items: Vec<LobstersItem> = self
            .client
            .get(HOTTEST_URL)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(items
            .into_iter()
            .filter_map(LobstersItem::into_post)
            .collect())
    }
}

#[derive(Deserialize)]
struct LobstersItem {
    short_id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    #[serde(default)]
    score: i64,
    #[serde(default)]
    comment_count: i64,
    #[serde(default)]
    created_at: Option<String>,
}

impl LobstersItem {
    fn into_post(self) -> Option<Post> {
        // The lobste.rs API only exposes a net score, so encode it as up/down
        // votes that preserve that net: a positive net is all upvotes, a
        // negative net is all downvotes.
        let votes = if self.score >= 0 {
            UpvotesAndDownvotes::new(self.score, 0)
        } else {
            UpvotesAndDownvotes::new(0, -self.score)
        };
        let posted_at = match self.created_at.as_deref() {
            Some(s) => DateTime::new_from_rfc3339(s).ok()?,
            None => DateTime::now(),
        };
        let external_url = ExternalUrl::new(self.url).ok();
        Some(Post::new(
            SourceId::Lobsters,
            PostId::new(self.short_id).ok()?,
            PostTitle::new(self.title).ok()?,
            external_url,
            posted_at,
            PostComments::new(self.comment_count).ok()?,
            PostScore::UpvotesAndDownvotes(votes.ok()?),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_hottest_items() -> Result<()> {
        let json = r#"[
            {"short_id":"abc","title":"x","url":"https://e.com/a","score":12,"comment_count":3,"created_at":"2024-01-02T09:30:00.000-07:00"},
            {"short_id":"def","title":"y","url":"https://e.com/b","score":-2,"comment_count":40,"created_at":"2024-01-02T09:30:00.000-07:00"}
        ]"#;
        let items: Vec<LobstersItem> = serde_json::from_str(json)?;
        let posts: Vec<Post> = items.into_iter().filter_map(|i| i.into_post()).collect();
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0].post_id().as_str(), "abc");
        match posts[0].score() {
            PostScore::UpvotesAndDownvotes(v) => assert_eq!(v.net(), 12),
            _ => panic!("wrong variant"),
        }
        match posts[1].score() {
            PostScore::UpvotesAndDownvotes(v) => assert_eq!(v.net(), -2),
            _ => panic!("wrong variant"),
        }
        assert_eq!(posts[1].comments().value(), 40);
        Ok(())
    }
}
