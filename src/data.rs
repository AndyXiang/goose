use crate::error::{Error, Result};
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use std::fmt::{self, Display, Formatter};
use std::str::FromStr;
use uuid::Uuid;

mod db;
pub use db::DataBase;
mod handler;
pub use handler::DataHandler;

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

impl FromStr for Asset {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "stock" => Ok(Self::Stock),
            "stockus" => Ok(Self::StockUS),
            "futures" => Ok(Self::Futures),
            _ => Err(Error::data(format!("invalid asset: {value}"))),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

impl FromStr for Exchange {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        match value {
            "sz" => Ok(Self::SZ),
            "bj" => Ok(Self::BJ),
            "sh" => Ok(Self::SH),
            _ => Err(Error::data(format!("invalid exchange: {value}"))),
        }
    }
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, PartialOrd, Ord)]
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

impl FromStr for AssetSymbol {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self> {
        let mut parts = value.splitn(3, '_');
        let asset = parts
            .next()
            .ok_or_else(|| Error::data(format!("invalid asset symbol: {value}")))?
            .parse()?;
        let exchange = parts
            .next()
            .ok_or_else(|| Error::data(format!("invalid asset symbol: {value}")))?
            .parse()?;
        let id = parts
            .next()
            .ok_or_else(|| Error::data(format!("invalid asset symbol: {value}")))?
            .parse()?;

        Ok(Self {
            asset,
            exchange,
            id,
        })
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
