#![allow(unused)]
use goose::{
    data::{DataBase, DataHandler, ExchangeCN::*, RestMethod, RestRequest, StockCN, TimeStamp},
    error::{Error, Result},
};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use uuid::Uuid;

const CSV_IMPORT_SMOKE_ROWS: usize = 16;

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

fn csv_root() -> Result<PathBuf> {
    let path = std::env::var("CSV_PATH")
        .map_err(|_| Error::data("missing CSV_PATH environment variable"))?;

    if let Some(rest) = path.strip_prefix("~/") {
        let home =
            std::env::var("HOME").map_err(|_| Error::data("missing HOME environment variable"))?;
        Ok(PathBuf::from(home).join(rest))
    } else {
        Ok(PathBuf::from(path))
    }
}

fn csv_files() -> Result<Vec<PathBuf>> {
    let root = csv_root()?;
    let mut files = fs::read_dir(&root)
        .map_err(|err| Error::data(format!("failed to read CSV_PATH: {err}")))?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|err| Error::data(format!("failed to read CSV entry: {err}")))?;

    files.retain(|path| path.extension().is_some_and(|extension| extension == "csv"));
    files.sort();
    Ok(files)
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

fn exchange_from_ts_code(ts_code: &str) -> Result<goose::data::ExchangeCN> {
    match ts_code.rsplit_once('.') {
        Some((_, "SZ")) => Ok(SZ),
        Some((_, "SH")) => Ok(SH),
        Some((_, "BJ")) => Ok(BJ),
        _ => Err(Error::data(format!("unsupported ts_code: {ts_code}"))),
    }
}

fn stock_from_ts_code(ts_code: &str, list_date: &str) -> Result<StockCN> {
    let (code, _) = ts_code
        .split_once('.')
        .ok_or_else(|| Error::data(format!("invalid ts_code: {ts_code}")))?;
    let exchange = exchange_from_ts_code(ts_code)?;
    let list_date: TimeStamp = list_date.parse()?;

    Ok(StockCN::new(exchange, code, code, list_date))
}

fn insert_entity_for_stock(handler: &DataHandler<'_, DataBase>, stock: &StockCN) -> Result<()> {
    request(
        handler,
        RestMethod::Post,
        &["entity"],
        map(&[
            ("id", &stock.id.to_string()),
            ("entity_type", "stock_cn"),
            ("entity_table", "stock_cn"),
        ]),
    )?;
    Ok(())
}

fn insert_calendar_day(
    handler: &DataHandler<'_, DataBase>,
    exchange: &goose::data::ExchangeCN,
    date: &str,
) -> Result<()> {
    request(
        handler,
        RestMethod::Post,
        &["calendar"],
        map(&[
            ("market", &exchange.to_string()),
            ("date", date),
            ("is_open", "1"),
        ]),
    )?;
    Ok(())
}

fn qfq_daily_bar_body(stock: &StockCN, fields: &[&str]) -> Result<HashMap<String, String>> {
    if fields.len() != 16 {
        return Err(Error::data(format!(
            "unexpected qfq CSV column count: {}",
            fields.len()
        )));
    }

    Ok(map(&[
        ("id", &stock.id.to_string()),
        ("date", fields[1]),
        ("exchange", &stock.exchange.to_string()),
        ("open", fields[12]),
        ("high", fields[13]),
        ("low", fields[14]),
        ("close", fields[15]),
        ("volume", fields[9]),
        ("amount", fields[10]),
        ("adjust", "qfq"),
    ]))
}

fn import_qfq_csv_file(
    handler: &DataHandler<'_, DataBase>,
    path: &Path,
    row_limit: Option<usize>,
) -> Result<usize> {
    let content = fs::read_to_string(path)
        .map_err(|err| Error::data(format!("failed to read qfq CSV {:?}: {err}", path)))?;
    let mut lines = content.lines();
    let header = lines
        .next()
        .ok_or_else(|| Error::data(format!("empty qfq CSV: {:?}", path)))?;
    let expected_header = "ts_code,trade_date,open,high,low,close,pre_close,change,pct_chg,vol,amount,adj_factor,open_adj,high_adj,low_adj,close_adj";
    if header != expected_header {
        return Err(Error::data(format!("unexpected qfq CSV header: {header}")));
    }

    let mut imported = 0;
    let mut stock = None;
    for line in lines.take(row_limit.unwrap_or(usize::MAX)) {
        let fields = line.split(',').collect::<Vec<_>>();
        if fields.len() != 16 {
            return Err(Error::data(format!("invalid qfq CSV row: {line}")));
        }

        let stock = match &stock {
            Some(stock) => stock,
            None => {
                stock = Some(stock_from_ts_code(fields[0], fields[1])?);
                let stock = stock.as_ref().expect("stock was just initialized");
                insert_entity_for_stock(handler, stock)?;
                stock
            }
        };
        insert_calendar_day(handler, &stock.exchange, fields[1])?;
        request(
            handler,
            RestMethod::Post,
            &["daily_bar"],
            qfq_daily_bar_body(stock, &fields)?,
        )?;
        imported += 1;
    }

    Ok(imported)
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
fn csv_path_contains_qfq_csv_files() -> Result<()> {
    let files = csv_files()?;

    assert!(!files.is_empty());
    assert!(files.iter().all(|path| path.extension().unwrap() == "csv"));
    Ok(())
}

#[test]
fn import_one_qfq_csv_file_smoke() -> Result<()> {
    let db = test_database()?;
    let handler = DataHandler::new(&db);
    let file = csv_files()?
        .into_iter()
        .next()
        .ok_or_else(|| Error::data("CSV_PATH has no CSV files"))?;

    let imported = import_qfq_csv_file(&handler, &file, Some(CSV_IMPORT_SMOKE_ROWS))?;
    let rows = request(
        &handler,
        RestMethod::Get,
        &["daily_bar", "*"],
        HashMap::new(),
    )?;

    assert_eq!(imported, CSV_IMPORT_SMOKE_ROWS);
    assert_eq!(rows.len(), CSV_IMPORT_SMOKE_ROWS);
    assert!(rows.iter().all(|row| {
        row.get("adjust").map(String::as_str) == Some("qfq")
            && row.contains_key("open")
            && row.contains_key("high")
            && row.contains_key("low")
            && row.contains_key("close")
    }));
    Ok(())
}

#[test]
fn importing_same_qfq_rows_is_idempotent() -> Result<()> {
    let db = test_database()?;
    let handler = DataHandler::new(&db);
    let file = csv_files()?
        .into_iter()
        .next()
        .ok_or_else(|| Error::data("CSV_PATH has no CSV files"))?;

    import_qfq_csv_file(&handler, &file, Some(CSV_IMPORT_SMOKE_ROWS))?;
    import_qfq_csv_file(&handler, &file, Some(CSV_IMPORT_SMOKE_ROWS))?;
    let rows = request(
        &handler,
        RestMethod::Get,
        &["daily_bar", "*"],
        HashMap::new(),
    )?;

    assert_eq!(rows.len(), CSV_IMPORT_SMOKE_ROWS);
    Ok(())
}

#[test]
#[ignore = "imports every CSV under CSV_PATH and can be slow"]
fn import_full_a_share_qfq_csv_dataset() -> Result<()> {
    let db = test_database()?;
    let handler = DataHandler::new(&db);
    let files = csv_files()?;
    let mut imported = 0;

    for file in &files {
        imported += import_qfq_csv_file(&handler, file, None)?;
    }

    let rows = request(
        &handler,
        RestMethod::Get,
        &["daily_bar", "*"],
        HashMap::new(),
    )?;

    assert!(!files.is_empty());
    assert_eq!(rows.len(), imported);
    Ok(())
}
