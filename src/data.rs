use crate::error::{Error, Result};
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use rust_decimal::Decimal;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use uuid::{Uuid, uuid};

mod db;
mod entity;
mod handler;

pub use db::DataBase;
pub use entity::*;
pub use handler::*;

pub const GOOSE_NAMESPACE: Uuid = uuid::uuid!("8771e211-d0f2-4a12-80fc-8d2b7244a231");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TimeStamp(DateTime<Utc>);

impl TimeStamp {
    pub fn new(value: DateTime<Utc>) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> DateTime<Utc> {
        self.0
    }
}

impl Display for TimeStamp {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.to_rfc3339_opts(SecondsFormat::Secs, true))
    }
}

impl From<DateTime<Utc>> for TimeStamp {
    fn from(value: DateTime<Utc>) -> Self {
        Self(value)
    }
}

impl From<TimeStamp> for DateTime<Utc> {
    fn from(value: TimeStamp) -> Self {
        value.0
    }
}

impl FromStr for TimeStamp {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        if let Ok(ts) = DateTime::parse_from_rfc3339(value) {
            return Ok(Self(ts.with_timezone(&Utc)));
        }

        for format in ["%Y-%m-%d", "%Y%m%d"] {
            if let Ok(date) = NaiveDate::parse_from_str(value, format) {
                let datetime = date
                    .and_hms_opt(0, 0, 0)
                    .ok_or_else(|| Error::data(format!("invalid date: {value}")))?;

                return Ok(Self(DateTime::from_naive_utc_and_offset(datetime, Utc)));
            }
        }

        Err(Error::data(format!("invalid timestamp: {value}")))
    }
}

impl TryFrom<&str> for TimeStamp {
    type Error = Error;

    fn try_from(value: &str) -> Result<Self> {
        value.parse()
    }
}

impl TryFrom<String> for TimeStamp {
    type Error = Error;

    fn try_from(value: String) -> Result<Self> {
        value.parse()
    }
}

#[derive(Debug, Clone)]
pub struct Bar {
    pub open: Decimal,
    pub high: Decimal,
    pub low: Decimal,
    pub close: Decimal,
    pub volume: Decimal,
}

impl Bar {
    pub fn to_raw(self) -> BarRaw {
        BarRaw {
            open: self.open.to_string(),
            high: self.high.to_string(),
            low: self.low.to_string(),
            close: self.close.to_string(),
            volume: self.volume.to_string(),
        }
    }
}

impl From<Bar> for BarRaw {
    fn from(bar: Bar) -> Self {
        bar.to_raw()
    }
}

#[derive(Debug, Clone)]
pub struct BarRaw {
    pub open: String,
    pub high: String,
    pub low: String,
    pub close: String,
    pub volume: String,
}

impl BarRaw {
    pub fn to_bar(self) -> Result<Bar> {
        Ok(Bar {
            open: self.open.parse()?,
            high: self.high.parse()?,
            low: self.low.parse()?,
            close: self.close.parse()?,
            volume: self.volume.parse()?,
        })
    }
}

impl TryFrom<BarRaw> for Bar {
    type Error = Error;

    fn try_from(raw: BarRaw) -> Result<Self> {
        raw.to_bar()
    }
}

pub enum DataKind {
    DailyBar,
    News,
}
