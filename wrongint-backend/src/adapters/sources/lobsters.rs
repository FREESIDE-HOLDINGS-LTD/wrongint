use crate::app;
use crate::domain::{PostCapture, SourceId};
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
    fn into_capture(self) -> PostCapture {
        PostCapture {
            source: SourceId::Lobsters,
            post_id: self.short_id,
            title: self.title,
            url: self.url,
            comments: self.comment_count,
            upvotes: self.score,
            sampled_at: chrono::Utc::now(),
        }
    }
}

#[async_trait]
impl app::Source for Lobsters {
    fn id(&self) -> SourceId {
        SourceId::Lobsters
    }

    async fn fetch_front_page(&self) -> Result<Vec<PostCapture>> {
        let items: Vec<LobstersItem> = self
            .client
            .get(HOTTEST_URL)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        Ok(items.into_iter().map(LobstersItem::into_capture).collect())
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
        let caps: Vec<PostCapture> = items.into_iter().map(|i| i.into_capture()).collect();
        assert_eq!(caps.len(), 2);
        assert_eq!(caps[0].post_id, "abc");
        assert_eq!(caps[0].upvotes, 12);
        assert_eq!(caps[1].upvotes, -2);
        assert_eq!(caps[1].comments, 40);
    }
}
