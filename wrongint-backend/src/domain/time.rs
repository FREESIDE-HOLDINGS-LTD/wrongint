use crate::errors::Result;
use anyhow::anyhow;
use chrono::{DurationRound, Timelike};
use std::fmt::Display;
use std::ops::{Add, Sub};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    d: chrono::NaiveDate,
}

impl Date {
    pub fn to_iso(&self) -> String {
        self.d.format("%Y-%m-%d").to_string()
    }
}

impl Display for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_iso())
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq, Ord, Hash)]
pub struct DateTime {
    dt: chrono::DateTime<chrono::FixedOffset>,
}

impl DateTime {
    pub fn now() -> Self {
        Self {
            dt: chrono::Utc::now().fixed_offset(),
        }
    }

    pub fn new_from_unix_timestamp(unix_timestamp: i64) -> Result<Self> {
        let dt = chrono::DateTime::from_timestamp(unix_timestamp, 0)
            .ok_or_else(|| anyhow!("bad unix timestamp"))?;
        Ok(Self {
            dt: dt.fixed_offset(),
        })
    }

    pub fn new_from_rfc3339(s: &str) -> Result<Self> {
        let dt = chrono::DateTime::parse_from_rfc3339(s)?;
        Ok(Self { dt })
    }

    fn new(dt: chrono::DateTime<chrono::FixedOffset>) -> Self {
        Self { dt }
    }

    pub fn unix_timestamp(&self) -> i64 {
        self.dt.timestamp()
    }

    pub fn date(&self) -> Date {
        Date {
            d: self.dt.with_timezone(&chrono::Utc).date_naive(),
        }
    }

    pub fn hour_of_day(&self) -> u32 {
        self.dt.with_timezone(&chrono::Utc).hour()
    }

    pub fn truncate_to_day(&self) -> Result<Self> {
        Ok(Self::new(
            self.dt.duration_trunc(chrono::Duration::days(1))?,
        ))
    }

    pub fn truncate_to_hour(&self) -> Result<Self> {
        Ok(Self::new(
            self.dt.duration_trunc(chrono::Duration::hours(1))?,
        ))
    }

    pub fn to_rfc3339(&self) -> String {
        self.dt
            .with_timezone(&chrono::Utc)
            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
    }
}

impl Display for DateTime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_rfc3339())
    }
}

impl Sub<Duration> for DateTime {
    type Output = DateTime;

    fn sub(self, rhs: Duration) -> Self::Output {
        DateTime::new(self.dt - rhs.d)
    }
}

impl Add<Duration> for DateTime {
    type Output = DateTime;

    fn add(self, rhs: Duration) -> Self::Output {
        DateTime::new(self.dt + rhs.d)
    }
}

impl Sub<DateTime> for DateTime {
    type Output = Duration;

    fn sub(self, rhs: DateTime) -> Self::Output {
        Duration::new(self.dt - rhs.dt)
    }
}

#[derive(Debug, Clone, Copy, PartialOrd, PartialEq, Eq, Ord)]
pub struct Duration {
    d: chrono::Duration,
}

impl Duration {
    pub fn new_from_seconds(seconds: u64) -> Self {
        Self {
            d: chrono::Duration::seconds(seconds as i64),
        }
    }

    pub fn new_from_days(days: u64) -> Self {
        Self {
            d: chrono::Duration::days(days as i64),
        }
    }

    fn new(d: chrono::Duration) -> Self {
        Self { d }
    }

    pub fn to_std(&self) -> std::time::Duration {
        self.d.to_std().unwrap_or(std::time::Duration::from_secs(0))
    }
}
