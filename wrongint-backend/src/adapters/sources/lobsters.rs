use crate::app;
use crate::domain::{NumberOfComments, Post, PostId, Score, SourceId, Title, Url};
use crate::errors::Result;
use async_trait::async_trait;
use serde::Deserialize;

const HOTTEST_URL: &str = "https://lobste.rs/hottest.json";

pub struct Lobsters {
    client: reqwest::Client,
}

impl Lobsters {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
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
}

impl LobstersItem {
    fn into_post(self) -> Option<Post> {
        Some(Post::new(
            SourceId::Lobsters,
            PostId::new(self.short_id).ok()?,
            Title::new(self.title),
            Url::new(self.url),
            NumberOfComments::new(self.comment_count),
            Score::UpvotesAndDownvotes(self.score),
        ))
    }
}

#[async_trait]
impl app::Source for Lobsters {
    fn id(&self) -> SourceId {
        SourceId::Lobsters
    }

    async fn fetch_front_page(&self) -> Result<Vec<Post>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_hottest_items() {
        let json = r#"[
            {"short_id":"abc","title":"x","url":"https://e.com/a","score":12,"comment_count":3},
            {"short_id":"def","title":"y","url":"https://e.com/b","score":-2,"comment_count":40}
        ]"#;
        let items: Vec<LobstersItem> = serde_json::from_str(json).unwrap();
        let posts: Vec<Post> = items.into_iter().filter_map(|i| i.into_post()).collect();
        assert_eq!(posts.len(), 2);
        assert_eq!(posts[0].post_id().as_str(), "abc");
        assert_eq!(posts[0].score(), Score::UpvotesAndDownvotes(12));
        assert_eq!(posts[1].score(), Score::UpvotesAndDownvotes(-2));
        assert_eq!(posts[1].comments().value(), 40);
    }
}
