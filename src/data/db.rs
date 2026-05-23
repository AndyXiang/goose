use crate::{
    data::{BarRaw, Entity},
    error::Result,
};
use rusqlite::{params, Connection};

// The struct accept data query and record
pub struct DataBase {
    conn: Connection,
}

impl DataBase {
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute(
            "
            CREATE TABLE IF NOT EXISTS daily_bar (
                id      TEXT NOT NULL,
                date    TEXT NOT NULL,
                open    TEXT NOT NULL,
                high    TEXT NOT NULL,
                low     TEXT NOT NULL,
                close   TEXT NOT NULL,
                volume  TEXT NOT NULL,
                amount  TEXT NOT NULL,
                qfq     TEXT NOT NULL,
                PRIMARY KEY (id, date)
            );
            ",
            params![],
        )?;

        Ok(Self { conn })
    }

    pub fn register_entity(&self, entity: impl Entity) -> Result<usize> {
        self.conn.execute_batch(entity.schema())?;
        Ok(0)
    }

    // get the candlestick data for given id in the range of [start, end]
    pub fn get_bar(&self, id: &str, start: &str, end: &str) -> Result<Vec<(String, BarRaw)>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT date, open, high, low, close, volume
            FROM daily_bar
            WHERE id = ?1 AND date >= ?2 AND date <= ?3
            ORDER BY date
            ",
        )?;

        let rows = stmt.query_map(params![id, start, end], |row| {
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

    pub fn execute(&self, sql: &str) -> Result<usize> {
        self.conn.execute_batch(sql)?;
        Ok(0)
    }
}
