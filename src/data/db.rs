use super::*;
use crate::error::Result;
use rusqlite::{Connection, params};
use std::collections::BTreeMap;

// The struct accept data query and record
pub struct DataBase {
    conn: Connection,
}

impl DataBase {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        Ok(Self { conn })
    }

    pub fn init_schema(&self) -> Result<usize> {
        // create basic daily_bar table
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS daily_bars (
                symbol TEXT NOT NULL,
                date TEXT NOT NULL,
                open   TEXT NOT NULL,
                high   TEXT NOT NULL,
                low    TEXT NOT NULL,
                close  TEXT NOT NULL,
                volume TEXT NOT NULL,
                PRIMARY KEY (symbol, date)
            );

            CREATE INDEX IF NOT EXISTS idx_daily_bars_date_symbol
            ON daily_bars(date, symbol);
            ",
        )?;
        Ok(0)
    }

    pub fn upsert_bar(&self, symbol: String, ts: String, bar: BarRaw) -> Result<()> {
        self.conn.execute(
            "
            INSERT INTO daily_bars
                (symbol, date, open, high, low, close, volume)
            VALUES
                (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(symbol, date) DO UPDATE SET
                open = excluded.open,
                high = excluded.high,
                low = excluded.low,
                close = excluded.close,
                volume = excluded.volume
            ",
            params![
                symbol, ts, bar.open, bar.high, bar.low, bar.close, bar.volume
            ],
        )?;

        Ok(())
    }

    // query the bar data for the given symbol. Returns in (date, bar).
    pub fn get_bar_with_symbol(&self, symbol: String) -> Result<Vec<(String, BarRaw)>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT date, open, high, low, close, volume
            FROM daily_bars
            WHERE symbol = ?1
            ORDER BY date
            ",
        )?;

        let rows = stmt.query_map(params![symbol], |row| {
            Ok((
                row.get(0)?,
                BarRaw {
                    open: row.get(1)?,
                    high: row.get(2)?,
                    low: row.get(3)?,
                    close: row.get(4)?,
                    volume: row.get(5)?,
                },
            ))
        })?;

        let bars = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(bars)
    }

    // Query the bar data for the given date. Returns in (symbol, bar)
    pub fn get_bar_with_date(&self, date: String) -> Result<Vec<(String, BarRaw)>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT symbol, open, high, low, close, volume
            FROM daily_bars
            WHERE date = ?1
            ORDER BY symbol 
            ",
        )?;

        let rows = stmt.query_map(params![date], |row| {
            Ok((
                row.get(0)?,
                BarRaw {
                    open: row.get(1)?,
                    high: row.get(2)?,
                    low: row.get(3)?,
                    close: row.get(4)?,
                    volume: row.get(5)?,
                },
            ))
        })?;

        let bars = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(bars)
    }

    // Query all symbols in the given date range. Returns in symbol -> Vec<(date, bar)>.
    pub fn get_bar_with_range(
        &self,
        start: String,
        end: String,
    ) -> Result<BTreeMap<String, Vec<(String, BarRaw)>>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT symbol, date, open, high, low, close, volume
            FROM daily_bars
            WHERE date >= ?1 AND date <= ?2
            ORDER BY date, symbol
            ",
        )?;

        let mut rows = stmt.query(params![start, end])?;
        let mut bars_by_symbol: BTreeMap<String, Vec<(String, BarRaw)>> = BTreeMap::new();

        while let Some(row) = rows.next()? {
            let symbol = row.get(0)?;
            let date = row.get(1)?;
            let bar = BarRaw {
                open: row.get(2)?,
                high: row.get(3)?,
                low: row.get(4)?,
                close: row.get(5)?,
                volume: row.get(6)?,
            };

            bars_by_symbol
                .entry(symbol)
                .or_insert_with(Vec::new)
                .push((date, bar));
        }

        Ok(bars_by_symbol)
    }

    // Query one symbol in the given date range. Returns in (date, bar).
    pub fn get_bar_with_symbol_range(
        &self,
        symbol: String,
        start: String,
        end: String,
    ) -> Result<Vec<(String, BarRaw)>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT date, open, high, low, close, volume
            FROM daily_bars
            WHERE symbol = ?1 AND date >= ?2 AND date <= ?3
            ORDER BY date
            ",
        )?;

        let rows = stmt.query_map(params![symbol, start, end], |row| {
            Ok((
                row.get(0)?,
                BarRaw {
                    open: row.get(1)?,
                    high: row.get(2)?,
                    low: row.get(3)?,
                    close: row.get(4)?,
                    volume: row.get(5)?,
                },
            ))
        })?;

        let bars = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(bars)
    }

    // Delete one symbol in the given date range. Returns deleted row count.
    pub fn delete_bar_with_symbol_range(
        &self,
        symbol: String,
        start: String,
        end: String,
    ) -> Result<usize> {
        let deleted = self.conn.execute(
            "
            DELETE FROM daily_bars
            WHERE symbol = ?1 AND date >= ?2 AND date <= ?3
            ",
            params![symbol, start, end],
        )?;

        Ok(deleted)
    }
}
