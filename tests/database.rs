#![allow(unused)]
use goose::{
    data::{DataBase, DataHandler, ExchangeCN::*, RestMethod, RestRequest, StockCN, TimeStamp},
    error::Result,
};
use std::collections::HashMap;
use uuid::Uuid;

fn test_stocks() -> Vec<StockCN> {
    vec![StockCN::new(
        SZ,
        "000001",
        "平安银行",
        "2010-01-01".parse().expect("valid stock list date"),
    )]
}

fn test_stocks_id() -> Vec<Uuid> {
    test_stocks().into_iter().map(|s| s.id).collect()
}

fn stock_id() -> String {
    test_stocks_id()[0].to_string()
}

const EXCHANGE: &str = "exchange_cn_sz";
const TRADE_DATE: &str = "2020-01-02";

fn test_database() -> Result<DataBase> {
    DataBase::new(":memory:")
}

fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(key, value)| ((*key).to_string(), (*value).to_string()))
        .collect()
}

fn request(
    handler: &DataHandler<'_, DataBase>,
    method: RestMethod,
    path: &[&str],
    body: HashMap<String, String>,
) -> Result<Vec<HashMap<String, String>>> {
    handler.request(RestRequest::new(
        method,
        path.iter().map(|part| (*part).to_string()).collect(),
        body,
    ))
}

fn insert_entity(handler: &DataHandler<'_, DataBase>) -> Result<()> {
    let id = stock_id();
    request(
        handler,
        RestMethod::Post,
        &["entity"],
        map(&[
            ("id", &id),
            ("entity_type", "stock_cn"),
            ("entity_table", "stock_cn"),
        ]),
    )?;
    Ok(())
}

fn insert_calendar(handler: &DataHandler<'_, DataBase>) -> Result<()> {
    request(
        handler,
        RestMethod::Post,
        &["calendar"],
        map(&[
            ("market", &SZ.to_string()),
            ("date", TRADE_DATE),
            ("is_open", "1"),
        ]),
    )?;
    Ok(())
}

fn daily_bar_body(close: &str) -> HashMap<String, String> {
    let id = stock_id();
    map(&[
        ("id", &id),
        ("date", TRADE_DATE),
        ("exchange", EXCHANGE),
        ("open", "10.00"),
        ("high", "10.50"),
        ("low", "9.80"),
        ("close", close),
        ("volume", "1000000"),
        ("amount", "10300000"),
        ("adjust", "none"),
    ])
}

fn insert_daily_bar(handler: &DataHandler<'_, DataBase>, close: &str) -> Result<()> {
    request(
        handler,
        RestMethod::Post,
        &["daily_bar"],
        daily_bar_body(close),
    )?;
    Ok(())
}

fn query_daily_bar(handler: &DataHandler<'_, DataBase>) -> Result<Vec<HashMap<String, String>>> {
    let id = stock_id();
    request(
        handler,
        RestMethod::Get,
        &[
            "daily_bar",
            "id",
            &id,
            "date",
            TRADE_DATE,
            "exchange",
            EXCHANGE,
            "*",
        ],
        HashMap::new(),
    )
}

fn setup_daily_bar_case() -> Result<DataBase> {
    let db = test_database()?;
    let handler = DataHandler::new(&db);
    insert_entity(&handler)?;
    insert_calendar(&handler)?;
    insert_daily_bar(&handler, "10.30")?;
    Ok(db)
}

#[test]
fn database_starts_with_builtin_tables() -> Result<()> {
    let db = test_database()?;
    let tables = db.tables()?;

    assert!(tables.contains(&"entity".to_string()));
    assert!(tables.contains(&"calendar".to_string()));
    assert!(tables.contains(&"daily_bar".to_string()));
    Ok(())
}

#[test]
fn rest_post_and_get_daily_bar() -> Result<()> {
    let db = setup_daily_bar_case()?;
    let handler = DataHandler::new(&db);

    let rows = query_daily_bar(&handler)?;
    let id = stock_id();

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("id").map(String::as_str), Some(id.as_str()));
    assert_eq!(rows[0].get("date").map(String::as_str), Some(TRADE_DATE));
    assert_eq!(rows[0].get("exchange").map(String::as_str), Some(EXCHANGE));
    assert_eq!(rows[0].get("close").map(String::as_str), Some("10.30"));
    Ok(())
}

#[test]
fn rest_patch_updates_existing_daily_bar() -> Result<()> {
    let db = setup_daily_bar_case()?;
    let handler = DataHandler::new(&db);
    let id = stock_id();

    request(
        &handler,
        RestMethod::Patch,
        &["daily_bar"],
        daily_bar_body("10.88"),
    )?;
    let rows = query_daily_bar(&handler)?;

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].get("close").map(String::as_str), Some("10.88"));
    Ok(())
}

#[test]
fn rest_delete_removes_daily_bar() -> Result<()> {
    let db = setup_daily_bar_case()?;
    let handler = DataHandler::new(&db);
    let id = stock_id();

    request(
        &handler,
        RestMethod::Delete,
        &[
            "daily_bar",
            "id",
            &id,
            "date",
            TRADE_DATE,
            "exchange",
            EXCHANGE,
            "*",
        ],
        HashMap::new(),
    )?;
    let rows = query_daily_bar(&handler)?;

    assert!(rows.is_empty());
    Ok(())
}

#[test]
#[ignore = "fixture CSV path is not wired yet"]
fn import_daily_bar_csv_fixture() -> Result<()> {
    let _db = test_database()?;

    // TODO: Fill this section after the test fixture file is chosen.
    // Expected fixture shape:
    // id,date,exchange,open,high,low,close,volume,amount,adjust
    //
    // Planned assertions:
    // - imported row count matches the CSV row count
    // - GET by id/date/exchange returns the imported row
    // - importing the same CSV twice is idempotent

    Ok(())
}

#[test]
#[ignore = "requires local full A-share CSV files"]
fn import_full_a_share_daily_csv_dataset() -> Result<()> {
    let _db = test_database()?;

    // TODO: Fill this section after the full CSV file locations are finalized.
    // Keep this ignored so ordinary cargo test remains fast and deterministic.
    //
    // Planned assertions:
    // - raw and qfq datasets can both be imported
    // - total inserted rows match expected counts
    // - boundary dates 2010-01-01 and 2025-12-31 are queryable
    // - repeated imports do not create duplicate rows

    Ok(())
}
