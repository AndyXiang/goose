use super::*;
use chrono::{DateTime, NaiveDate, SecondsFormat, Utc};
use std::collections::BTreeMap;

pub struct DataHandler<'a> {
    db: &'a DataBase,
}

impl<'a> DataHandler<'a> {
    pub fn new(db: &'a DataBase) -> Self {
        Self { db }
    }

    pub fn upsert_bar(&self, symbol: AssetSymbol, ts: TimeStamp, bar: Bar) -> Result<()> {
        self.db
            .upsert_bar(symbol.to_string(), timestamp_to_string(ts), bar.into())
    }

    pub fn get_bar_with_symbol(&self, symbol: AssetSymbol) -> Result<Vec<(TimeStamp, Bar)>> {
        self.db
            .get_bar_with_symbol(symbol.to_string())?
            .into_iter()
            .map(|(ts, bar)| Ok((parse_timestamp(&ts)?, bar.try_into()?)))
            .collect()
    }

    pub fn get_bar_with_date(&self, date: TimeStamp) -> Result<Vec<(AssetSymbol, Bar)>> {
        self.db
            .get_bar_with_date(timestamp_to_string(date))?
            .into_iter()
            .map(|(symbol, bar)| Ok((symbol.parse()?, bar.try_into()?)))
            .collect()
    }

    pub fn get_bar_with_range(
        &self,
        start: TimeStamp,
        end: TimeStamp,
    ) -> Result<BTreeMap<AssetSymbol, Vec<(TimeStamp, Bar)>>> {
        let raw_bars = self
            .db
            .get_bar_with_range(timestamp_to_string(start), timestamp_to_string(end))?;

        let mut bars = BTreeMap::new();

        for (symbol, series) in raw_bars {
            let symbol = symbol.parse()?;
            let series = series
                .into_iter()
                .map(|(ts, bar)| Ok((parse_timestamp(&ts)?, bar.try_into()?)))
                .collect::<Result<Vec<_>>>()?;

            bars.insert(symbol, series);
        }

        Ok(bars)
    }

    pub fn get_bar_with_symbol_range(
        &self,
        symbol: AssetSymbol,
        start: TimeStamp,
        end: TimeStamp,
    ) -> Result<Vec<(TimeStamp, Bar)>> {
        self.db
            .get_bar_with_symbol_range(
                symbol.to_string(),
                timestamp_to_string(start),
                timestamp_to_string(end),
            )?
            .into_iter()
            .map(|(ts, bar)| Ok((parse_timestamp(&ts)?, bar.try_into()?)))
            .collect()
    }

    pub fn delete_bar_with_symbol_range(
        &self,
        symbol: AssetSymbol,
        start: TimeStamp,
        end: TimeStamp,
    ) -> Result<usize> {
        self.db.delete_bar_with_symbol_range(
            symbol.to_string(),
            timestamp_to_string(start),
            timestamp_to_string(end),
        )
    }
}

fn timestamp_to_string(ts: TimeStamp) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Secs, true)
}

fn parse_timestamp(value: &str) -> Result<TimeStamp> {
    if let Ok(ts) = DateTime::parse_from_rfc3339(value) {
        return Ok(ts.with_timezone(&Utc));
    }

    let date = NaiveDate::parse_from_str(value, "%Y-%m-%d")?;
    let datetime = date
        .and_hms_opt(0, 0, 0)
        .ok_or_else(|| Error::data(format!("invalid date: {value}")))?;

    Ok(DateTime::from_naive_utc_and_offset(datetime, Utc))
}
