use crate::app;
use crate::domain::{NumberOfComments, Post, PostId, Score, SourceId, Title, Url};
use crate::errors::Result;
use async_trait::async_trait;
use futures::stream::StreamExt;
use serde::Deserialize;

const TOP_STORIES_URL: &str = "https://hacker-news.firebaseio.com/v0/topstories.json";
const ITEM_CONCURRENCY: usize = 10;

pub struct HackerNews {
    client: reqwest::Client,
    front_page_len: usize,
}

impl HackerNews {
    pub fn new(client: reqwest::Client, front_page_len: usize) -> Self {
        Self {
            client,
            front_page_len,
        }
    }

    fn item_url(id: i64) -> String {
        format!("https://hacker-news.firebaseio.com/v0/item/{id}.json")
    }

    fn permalink(id: i64) -> String {
        format!("https://news.ycombinator.com/item?id={id}")
    }
}

#[derive(Deserialize)]
struct HnItem {
    id: i64,
    #[serde(rename = "type")]
    kind: Option<String>,
    #[serde(default)]
    score: i64,
    #[serde(default)]
    descendants: i64,
    #[serde(default)]
    title: String,
    url: Option<String>,
    #[serde(default)]
    dead: bool,
    #[serde(default)]
    deleted: bool,
}

impl HnItem {
    fn into_post(self) -> Option<Post> {
        if self.dead || self.deleted {
            return None;
        }
        if self.kind.as_deref() != Some("story") {
            return None;
        }
        let url = self.url.unwrap_or_else(|| HackerNews::permalink(self.id));
        Some(Post::new(
            SourceId::HackerNews,
            PostId::new(self.id.to_string()).ok()?,
            Title::new(self.title),
            Url::new(url),
            NumberOfComments::new(self.descendants),
            Score::Points(self.score),
        ))
    }
}

#[async_trait]
impl app::Source for HackerNews {
    fn id(&self) -> SourceId {
        SourceId::HackerNews
    }

    async fn fetch_front_page(&self) -> Result<Vec<Post>> {
        let ids: Vec<i64> = self
            .client
            .get(TOP_STORIES_URL)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let front: Vec<i64> = ids.into_iter().take(self.front_page_len).collect();

        let items: Vec<Option<Post>> = futures::stream::iter(front)
            .map(|id| {
                let client = self.client.clone();
                async move {
                    let item: HnItem = client
                        .get(Self::item_url(id))
                        .send()
                        .await?
                        .error_for_status()?
                        .json()
                        .await?;
                    Ok::<Option<Post>, crate::errors::Error>(item.into_post())
                }
            })
            .buffer_unordered(ITEM_CONCURRENCY)
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .collect::<Result<Vec<_>>>()?;

        Ok(items.into_iter().flatten().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_story_item() {
        let json = r#"{"id":1,"type":"story","score":100,"descendants":42,
            "title":"hello","url":"https://example.com"}"#;
        let item: HnItem = serde_json::from_str(json).unwrap();
        let post = item.into_post().unwrap();
        assert_eq!(post.post_id().as_str(), "1");
        assert_eq!(post.score(), Score::Points(100));
        assert_eq!(post.comments().value(), 42);
        assert_eq!(post.url().as_str(), "https://example.com");
    }

    #[test]
    fn self_post_uses_permalink() {
        let json = r#"{"id":7,"type":"story","score":5,"descendants":0,"title":"ask"}"#;
        let item: HnItem = serde_json::from_str(json).unwrap();
        let post = item.into_post().unwrap();
        assert_eq!(
            post.url().as_str(),
            "https://news.ycombinator.com/item?id=7"
        );
    }

    #[test]
    fn skips_non_story_and_dead() {
        let job = r#"{"id":2,"type":"job","score":1}"#;
        assert!(
            serde_json::from_str::<HnItem>(job)
                .unwrap()
                .into_post()
                .is_none()
        );
        let dead = r#"{"id":3,"type":"story","score":1,"dead":true}"#;
        assert!(
            serde_json::from_str::<HnItem>(dead)
                .unwrap()
                .into_post()
                .is_none()
        );
    }
}
