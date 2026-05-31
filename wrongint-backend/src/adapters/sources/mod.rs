pub mod hn;
pub mod lobsters;

use crate::errors::Result;
use std::time::Duration;

pub use hn::HackerNews;
pub use lobsters::Lobsters;

pub fn new_client(user_agent: &str, timeout_secs: u64) -> Result<reqwest::Client> {
    let client = reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(timeout_secs))
        .build()?;
    Ok(client)
}
