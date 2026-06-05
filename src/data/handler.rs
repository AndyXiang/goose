use crate::{
    data::DataBase,
    error::{Error, Result},
};
use std::collections::HashMap;

pub trait DataSource {}

impl DataSource for DataBase {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RestMethod {
    Post,
    Put,
    Patch,
    Get,
    Delete,
}

#[derive(Debug, Clone)]
pub struct RestRequest {
    // Path segments are already split by the caller.
    pub method: RestMethod,
    pub path: Vec<String>,
    pub body: HashMap<String, String>,
}

impl RestRequest {
    pub fn new(method: RestMethod, path: Vec<String>, body: HashMap<String, String>) -> Self {
        Self { method, path, body }
    }
}

#[derive(Clone)]
pub struct DataHandler<'a, S: DataSource> {
    source: &'a S,
}

impl<'a, S: DataSource> DataHandler<'a, S> {
    pub fn new(source: &'a S) -> Self {
        Self { source }
    }
}

impl<'a> DataHandler<'a, DataBase> {
    // Main entry for REST-like requests.
    pub fn request(&self, request: RestRequest) -> Result<Vec<HashMap<String, String>>> {
        // Resolve the target table, selected column, and path filters.
        let (table, query, params) = self.verify_path(request.path.clone())?;
        let body = match request.method {
            RestMethod::Get | RestMethod::Delete => Vec::new(),
            RestMethod::Post | RestMethod::Put | RestMethod::Patch => {
                self.verify_body(request.body)?
            }
        };

        match request.method {
            RestMethod::Get => {
                // GET returns raw rows; typed APIs can convert them later.
                let (sql, values) = build_select_sql(&table, &query, &params);
                self.source
                    .query_params(&sql, rusqlite::params_from_iter(values.iter()))
            }
            RestMethod::Delete => {
                // DELETE is only allowed with path filters.
                let (sql, values) = build_delete_sql(&table, &params)?;
                self.source
                    .execute_params(&sql, rusqlite::params_from_iter(values.iter()))?;
                Ok(Vec::new())
            }
            RestMethod::Post | RestMethod::Put | RestMethod::Patch => {
                // Body values are bound as SQL parameters inside build_upsert_sql.
                let (sql, values) = build_upsert_sql(&table, &body)?;
                self.source.execute_params(
                    &sql,
                    rusqlite::params_from_iter(values.iter().map(String::as_str)),
                )?;
                Ok(Vec::new())
            }
        }
    }

    // Parse path as: table/filter_key/filter_value/.../select_target.
    fn verify_path(&self, path: Vec<String>) -> Result<(String, String, Vec<(String, String)>)> {
        // The first segment is the table. The last segment is the SELECT target.
        // Middle segments must appear in key/value pairs and become WHERE filters.
        let mut iter = path.into_iter();
        let mut parsed_path = (String::new(), String::new(), Vec::new());

        let table = iter.next();
        if let Some(table) = table {
            validate_identifier(&table)?;
            if !self.source.tables()?.contains(&table) {
                return Err(Error::db(format!(
                    "404 Not Found: table {} does not exist.",
                    table
                )));
            } else {
                parsed_path.0 = table;
            }
        } else {
            return Err(Error::db("400 BadRequest: missing request path."));
        }

        let query = iter.next_back();
        if let Some(query) = query {
            validate_select_target(&query)?;
            parsed_path.1 = query;
        } else {
            return Ok(parsed_path);
        }

        let mut params = Vec::new();
        loop {
            match (iter.next(), iter.next()) {
                (Some(key), Some(val)) => {
                    validate_identifier(&key)?;
                    params.push((key, val));
                }
                (None, None) => break,
                _ => return Err(Error::db("400 BadRequest: invalid path.")),
            }
        }
        parsed_path.2 = params;

        return Ok(parsed_path);
    }

    // Validate body column names and return stable key order for SQL generation.
    fn verify_body(&self, body: HashMap<String, String>) -> Result<Vec<(String, String)>> {
        let mut parsed_body = Vec::with_capacity(body.len());
        for (key, value) in body {
            validate_identifier(&key)?;
            parsed_body.push((key, value));
        }
        parsed_body.sort_by(|left, right| left.0.cmp(&right.0));

        Ok(parsed_body)
    }
}

// Build a parameterized SELECT statement from the parsed path.
fn build_select_sql(
    table: &str,
    query: &str,
    params: &[(String, String)],
) -> (String, Vec<String>) {
    let select = if query.is_empty() { "*" } else { query };
    let mut sql = format!("SELECT {select} FROM {table}");
    let values = append_where_clause(&mut sql, params, 1);
    sql.push(';');
    (sql, values)
}

// Build a parameterized DELETE statement and reject full-table deletes.
fn build_delete_sql(table: &str, params: &[(String, String)]) -> Result<(String, Vec<String>)> {
    if params.is_empty() {
        return Err(Error::db("400 BadRequest: missing delete condition."));
    }

    let mut sql = format!("DELETE FROM {table}");
    let values = append_where_clause(&mut sql, params, 1);
    sql.push(';');
    Ok((sql, values))
}

// Build a parameterized UPSERT statement from body fields.
fn build_upsert_sql(table: &str, body: &[(String, String)]) -> Result<(String, Vec<String>)> {
    if body.is_empty() {
        return Err(Error::db("400 BadRequest: missing upsert body."));
    }

    let columns = body.iter().map(|(key, _)| key.as_str()).collect::<Vec<_>>();
    let values = body
        .iter()
        .map(|(_, value)| value.clone())
        .collect::<Vec<_>>();
    let placeholders = (1..=body.len())
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>();
    let conflict_columns = conflict_columns_for_table(table, &columns)?;
    let updates = columns
        .iter()
        .filter(|column| !conflict_columns.contains(column))
        .map(|column| format!("{column} = excluded.{column}"))
        .collect::<Vec<_>>();
    let conflict_action = if updates.is_empty() {
        "DO NOTHING".to_string()
    } else {
        format!("DO UPDATE SET {}", updates.join(", "))
    };

    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({}) ON CONFLICT({}) {conflict_action};",
        columns.join(", "),
        placeholders.join(", "),
        conflict_columns.join(", ")
    );

    Ok((sql, values))
}

// Choose the conflict key that matches each known table schema.
fn conflict_columns_for_table(table: &str, columns: &[&str]) -> Result<Vec<&'static str>> {
    let conflict_columns = match table {
        "calendar" => vec!["market", "date"],
        "daily_bar" => vec!["id", "date", "exchange"],
        "exchange_cn" | "stock_cn" => vec!["id", "date"],
        "entity" => vec!["id"],
        _ if columns.contains(&"id") && columns.contains(&"date") => vec!["id", "date"],
        _ if columns.contains(&"id") => vec!["id"],
        _ => return Err(Error::db("400 BadRequest: missing upsert conflict column.")),
    };

    for column in &conflict_columns {
        if !columns.contains(column) {
            return Err(Error::db(format!(
                "400 BadRequest: missing upsert conflict column: {column}."
            )));
        }
    }

    Ok(conflict_columns)
}

// Append WHERE clauses with placeholders and return the matching values.
fn append_where_clause(
    sql: &mut String,
    params: &[(String, String)],
    placeholder_start: usize,
) -> Vec<String> {
    let mut values = Vec::with_capacity(params.len());
    for (i, (key, value)) in params.iter().enumerate() {
        if i == 0 {
            sql.push_str(" WHERE ");
        } else {
            sql.push_str(" AND ");
        }
        sql.push_str(&format!("{key} = ?{}", placeholder_start + i));
        values.push(value.clone());
    }
    values
}

// SELECT supports either one safe column name or all columns.
fn validate_select_target(value: &str) -> Result<()> {
    if value == "*" {
        Ok(())
    } else {
        validate_identifier(value)
    }
}

// Only identifiers are interpolated into SQL, so they must be strict.
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
