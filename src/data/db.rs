use crate::error::{Error, Result};
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
                adjust   TEXT,
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

    // Check one table name without loading the whole table list.
    pub fn table_exists(&self, table: &str) -> Result<bool> {
        validate_identifier(table)?;

        let exists = self.conn.query_row(
            "
            SELECT EXISTS (
                SELECT 1
                FROM sqlite_schema
                WHERE type = 'table'
                  AND name = ?1
                  AND name NOT LIKE 'sqlite_%'
            )
            ",
            [table],
            |row| row.get::<_, bool>(0),
        )?;

        Ok(exists)
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

    // Read primary key columns from SQLite schema metadata.
    pub fn primary_key_columns(&self, table: &str) -> Result<Vec<String>> {
        validate_identifier(table)?;

        let sql = format!("PRAGMA table_info({table})");
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt.query_map([], |row| {
            let name: String = row.get(1)?;
            let pk_order: i64 = row.get(5)?;
            Ok((pk_order, name))
        })?;

        let mut columns = rows
            .collect::<rusqlite::Result<Vec<_>>>()?
            .into_iter()
            .filter(|(pk_order, _)| *pk_order > 0)
            .collect::<Vec<_>>();
        columns.sort_by_key(|(pk_order, _)| *pk_order);

        let columns = columns
            .into_iter()
            .map(|(_, name)| name)
            .collect::<Vec<_>>();
        if columns.is_empty() {
            return Err(Error::db(format!("table has no primary key: {table}")));
        }

        Ok(columns)
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

// PRAGMA table_info cannot bind the table name, so keep it strict.
fn validate_identifier(value: &str) -> Result<()> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(Error::db("empty SQL identifier"));
    };

    if !(first.is_ascii_alphabetic() || first == '_') {
        return Err(Error::db(format!("invalid SQL identifier: {value}")));
    }

    if chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') {
        Ok(())
    } else {
        Err(Error::db(format!("invalid SQL identifier: {value}")))
    }
}
