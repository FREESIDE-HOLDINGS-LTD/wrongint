use crate::errors::Result;
use anyhow::anyhow;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    http_address: String,
    database_path: String,
    request_timeout_secs: u64,
    user_agent: String,
}

impl Config {
    pub fn new(
        http_address: impl Into<String>,
        database_path: impl Into<String>,
        request_timeout_secs: u64,
        user_agent: impl Into<String>,
    ) -> Result<Self> {
        let http_address = http_address.into();
        if http_address.is_empty() {
            return Err(anyhow!("http_address can't be empty").into());
        }
        let database_path = database_path.into();
        if database_path.is_empty() {
            return Err(anyhow!("database_path can't be empty").into());
        }
        if request_timeout_secs == 0 {
            return Err(anyhow!("request_timeout_secs must be > 0").into());
        }
        let user_agent = user_agent.into();
        if user_agent.is_empty() {
            return Err(anyhow!("user_agent can't be empty").into());
        }
        Ok(Self {
            http_address,
            database_path,
            request_timeout_secs,
            user_agent,
        })
    }

    pub fn http_address(&self) -> &str {
        &self.http_address
    }

    pub fn database_path(&self) -> &str {
        &self.database_path
    }

    pub fn request_timeout_secs(&self) -> u64 {
        self.request_timeout_secs
    }

    pub fn user_agent(&self) -> &str {


        &self.user_agent
    }
}
