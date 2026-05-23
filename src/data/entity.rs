use crate::{
    data::{Bar, TimeStamp},
    error::{Error, Result},
};
use std::{
    fmt::{self, Display, Formatter},
    str::FromStr,
};
use uuid::Uuid;

pub trait Entity: Display {
    const ENTITY_TYPE: &'static str;
    // type Data;
    fn id(&self) -> Uuid;
    fn schema(&self) -> &'static str;
}

pub struct StockCN {
    pub code: String,
    pub name: String,
    pub exchange: String,
    pub list_date: TimeStamp,
    pub delist_date: Option<TimeStamp>,
}

impl Display for StockCN {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "stockcn_{}_{}", self.exchange, self.code)
    }
}

impl Entity for StockCN {
    const ENTITY_TYPE: &'static str = "stock";
    // type Data = (TimeStamp, Bar);

    fn id(&self) -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_OID, self.to_string().as_bytes())
    }

    fn schema(&self) -> &'static str {
        "
        CREATE TABLE IF NOT EXISTS stock_cn (
            id          TEXT NOT NULL,
            date        TEXT NOT NULL,
            code        TEXT NOT NULL,
            name        TEXT NOT NULL,
            exchange    TEXT NOT NULL,
            list_date   TEXT NOT NULL,
            delist_date TEXT,
            PRIMARY KEY (id, date)
        );
        "
    }
}

pub enum ExchangeCN {
    SZ,
    SH,
    BJ
}

impl Display for ExchangeCN {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SZ => write!(f, "exchange_cn_sz"),
            Self::SH => write!(f, "exchange_cn_sh"),
            Self::BJ => write!(f, "exchange_cn_bj"),
        }
    }
}

impl Entity for ExchangeCN {
    const ENTITY_TYPE: &'static str = "exchange";

    fn id(&self) -> Uuid {
        Uuid::new_v5(&Uuid::NAMESPACE_OID, self.to_string().as_bytes())
    }

    fn schema(&self) -> &'static str {
        "
        CREATE TABLE IF NOT EXISTS exchange_cn (
            id          TEXT NOT NULL,
            date        TEXT NOT NULL,
            PRIMARY KEY (id, date)
        );
        "
    } 
}
