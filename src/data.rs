use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::fmt::{self, Display, Formatter};
use uuid::Uuid;

mod db;
mod handler;

#[derive(Clone, Copy, Debug, Hash)]
// represent tradable assets
pub enum Asset {
    Stock,
    StockUS,
    Futures,
}

impl Display for Asset {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Stock => write!(f, "stock"),
            Self::StockUS => write!(f, "stockus"),
            Self::Futures => write!(f, "futures"),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash)]
pub enum Exchange {
    SZ,
    BJ,
    SH,
}

impl Display for Exchange {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SZ => write!(f, "sz"),
            Self::BJ => write!(f, "bj"),
            Self::SH => write!(f, "sh"),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash)]
pub struct AssetSymbol {
    asset: Asset,
    exchange: Exchange,
    id: Uuid,
}

impl AssetSymbol {
    pub fn to_string(self) -> String {
        format!("{}_{}_{}", self.asset, self.exchange, self.id)
    }
}

pub type TimeStamp = DateTime<Utc>;

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
