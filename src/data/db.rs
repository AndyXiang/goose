use crate::error::Result;
use rusqlite::{Connection, Params, types::ValueRef};
use std::collections::HashMap;

// Thin wrapper around the SQLite connection.
pub struct DataBase {
    conn: Connection,
}

impl DataBase {
    // Open the database and create the built-in tables.
    pub fn new(path: &str) -> Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS entity (
                id              TEXT NOT NULL,
                entity_type     TEXT NOT NULL,
                entity_table    TEXT NOT NULL,
                PRIMARY KEY (id)
            );

            CREATE TABLE IF NOT EXISTS calendar (
                market  TEXT NOT NULL,
                date    TEXT NOT NULL,
                is_open INTEGER NOT NULL,
                PRIMARY KEY (market, date)
            );

            CREATE TABLE IF NOT EXISTS daily_bar (
                id       TEXT NOT NULL,
                date     TEXT NOT NULL,
                exchange TEXT NOT NULL,
                open     TEXT NOT NULL,
                high     TEXT NOT NULL,
                low      TEXT NOT NULL,
                close    TEXT NOT NULL,
                volume   TEXT NOT NULL,
                amount   TEXT,
                qfq      TEXT,
                PRIMARY KEY (id, date, exchange),
                FOREIGN KEY (id) REFERENCES entity(id),
                FOREIGN KEY (exchange, date) REFERENCES calendar(market, date)
            );

            ",
        )?;

        Ok(Self { conn })
    }

    // Execute SQL without bound parameters, usually for schema statements.
    pub fn execute(&self, sql: &str) -> Result<usize> {
        self.conn.execute_batch(sql)?;
        Ok(0)
    }

    // Return all user-defined table names.
    pub fn tables(&self) -> Result<Vec<String>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT name
            FROM sqlite_schema
            WHERE type = 'table'
              AND name NOT LIKE 'sqlite_%'
            ORDER BY name
            ",
        )?;

        let rows = stmt.query_map([], |row| row.get(0))?;
        let tables = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(tables)
    }

    // Execute one parameterized statement and return affected rows.
    pub fn execute_params<P: Params>(&self, sql: &str, params: P) -> Result<usize> {
        let affected = self.conn.execute(sql, params)?;
        Ok(affected)
    }

    // Query rows and return raw string values keyed by column name.
    pub fn query_params<P: Params>(
        &self,
        sql: &str,
        params: P,
    ) -> Result<Vec<HashMap<String, String>>> {
        let mut stmt = self.conn.prepare(sql)?;
        let columns = stmt
            .column_names()
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();

        let rows = stmt.query_map(params, |row| {
            let mut record = HashMap::with_capacity(columns.len());
            for (index, column) in columns.iter().enumerate() {
                let value = sqlite_value_to_string(row.get_ref(index)?);
                record.insert(column.clone(), value);
            }
            Ok(record)
        })?;

        let records = rows.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(records)
    }
}

// Keep database output raw; typed conversion belongs in DataHandler APIs.
fn sqlite_value_to_string(value: ValueRef<'_>) -> String {
    match value {
        ValueRef::Null => String::new(),
        ValueRef::Integer(value) => value.to_string(),
        ValueRef::Real(value) => value.to_string(),
        ValueRef::Text(value) => String::from_utf8_lossy(value).into_owned(),
        ValueRef::Blob(value) => format!("{value:?}"),
    }
}
