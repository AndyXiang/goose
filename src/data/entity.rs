use crate::{
    data::{DataBase, DataHandler, GOOSE_NAMESPACE, RestMethod, RestRequest, TimeStamp},
    error::Result,
};
use std::{
    collections::HashMap,
    fmt::{self, Display, Formatter},
};
use uuid::Uuid;

pub trait Entity: Display {
    const ENTITY_TYPE: &'static str;
    // type Data;
    fn id(&self) -> Uuid;
    fn table(&self) -> &'static str;
    fn schema(&self) -> &'static str;
}

#[derive(Debug, Clone)]
pub enum ExchangeCN {
    SZ,
    SH,
    BJ,
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
    const ENTITY_TYPE: &'static str = "exchange_cn";

    fn id(&self) -> Uuid {
        Uuid::new_v5(&GOOSE_NAMESPACE, self.to_string().as_bytes())
    }

    fn table(&self) -> &'static str {
        "exchange_cn"
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

#[derive(Debug, Clone)]
pub struct StockCN {
    pub id: Uuid,
    pub exchange: ExchangeCN,
    pub code: String,
    pub name: String,
    pub list_date: TimeStamp,
}

impl StockCN {
    pub fn new(
        exchange: ExchangeCN,
        code: impl ToString,
        name: impl ToString,
        list_date: TimeStamp,
    ) -> Self {
        let id = Uuid::new_v5(
            &GOOSE_NAMESPACE,
            format!("{}.{}", exchange, code.to_string()).as_bytes(),
        );
        Self {
            id,
            exchange,
            code: code.to_string(),
            name: name.to_string(),
            list_date,
        }
    }

    pub fn upsert(
        &self,
        handler: &DataHandler<'_, DataBase>,
        date: TimeStamp,
        package: HashMap<String, String>,
    ) -> Result<()> {
        let name = package
            .get("name")
            .cloned()
            .unwrap_or_else(|| self.name.clone());

        let list_date: TimeStamp = package
            .get("list_date")
            .map(|value| value.parse())
            .transpose()?
            .unwrap_or(self.list_date);

        let delist_date: Option<TimeStamp> = package
            .get("delist_date")
            .map(|value| value.parse())
            .transpose()?;

        let mut package = package;
        package.insert("id".to_string(), self.id.to_string());
        package.insert("date".to_string(), date.to_string());
        package.insert("code".to_string(), self.code.clone());
        package.insert("name".to_string(), name);
        package.insert("exchange".to_string(), self.exchange.to_string());
        package.insert("list_date".to_string(), list_date.to_string());

        if let Some(delist_date) = delist_date {
            package.insert("delist_date".to_string(), delist_date.to_string());
        }

        let request = RestRequest::new(RestMethod::Patch, vec![self.table().to_string()], package);

        handler.request(request)?;
        Ok(())
    }
}

impl Display for StockCN {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}({})", self.exchange, self.code, self.name)
    }
}

impl Entity for StockCN {
    const ENTITY_TYPE: &'static str = "stock_cn";
    // type Data = (TimeStamp, Bar);

    fn id(&self) -> Uuid {
        self.id
    }

    fn table(&self) -> &'static str {
        "stock_cn"
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
