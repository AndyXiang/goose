use std::collections::VecDeque;

use diesel::{Connection, SqliteConnection, connection::SimpleConnection};
use goose::data::{
    CalendarEntry, DataBase, Date, DateBar, Fetcher, Ohlc, Persistable, Price, Quantity,
};
use goose::error::{Error, FetchError, LookupError, Result};

struct BatchFetcher<T> {
    batches: VecDeque<Vec<T>>,
}

impl<T> BatchFetcher<T> {
    fn new(batches: Vec<Vec<T>>) -> Self {
        Self {
            batches: batches.into(),
        }
    }
}

impl<T: Persistable> Fetcher for BatchFetcher<T> {
    type Item = T;

    fn fetch(&mut self) -> Result<Option<Vec<Self::Item>>> {
        Ok(self.batches.pop_front())
    }
}

fn database_with_calendar() -> DataBase {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    conn.batch_execute(include_str!(
        "../migrations/2026-06-12-081652-0000_create_bar_calendar/up.sql"
    ))
    .unwrap();
    conn.batch_execute("PRAGMA foreign_keys = ON;").unwrap();
    conn.batch_execute(
        "INSERT INTO calendar (date, is_open) VALUES
            ('2026-06-12', TRUE),
            ('2026-06-13', FALSE),
            ('2026-06-14', FALSE),
            ('2026-06-15', TRUE),
            ('2026-06-16', TRUE);",
    )
    .unwrap();
    conn.batch_execute(
        "INSERT INTO daily_bars
            (symbol, date, open, high, low, close, volume, amount)
         VALUES
            ('AAPL', '2026-06-12', 100000, 110000, 95000, 105000, 10000000, 1050000000),
            ('MSFT', '2026-06-12', 190000, 205000, 185000, 200000, 20000000, 4000000000),
            ('MSFT', '2026-06-15', 200000, 210000, 195000, 205000, 15000000, 3075000000),
            ('AAPL', '2026-06-16', 106000, 112000, 101000, 110000, 12000000, 1320000000);",
    )
    .unwrap();
    DataBase { conn }
}

#[test]
fn new_runs_migrations_and_enables_foreign_keys() {
    let mut database = DataBase::new(":memory:");

    assert!(database.available_symbols().unwrap().is_empty());
    assert!(
        database
            .conn
            .batch_execute(
                "INSERT INTO daily_bars (symbol, date)
                 VALUES ('AAPL', '2026-06-15');",
            )
            .is_err()
    );
}

fn bar(symbol: &str, date: &str, close: &str) -> DateBar {
    let volume: Quantity = "1000".parse().unwrap();
    DateBar {
        date: date.parse().unwrap(),
        ohlc: Ohlc {
            open: close.parse().unwrap(),
            high: close.parse().unwrap(),
            low: close.parse().unwrap(),
            close: close.parse().unwrap(),
        },
        volume,
        amount: close.parse::<Price>().unwrap() * volume,
        symbol: symbol.into(),
    }
}

fn date(value: &str) -> Date {
    value.parse().unwrap()
}

#[test]
fn insert_and_upsert_calendar_entries() {
    let mut database = DataBase::new(":memory:");
    let date: Date = "2026-06-15".parse().unwrap();

    assert_eq!(
        database
            .insert_calendar(&[CalendarEntry {
                date,
                is_open: false,
            }])
            .unwrap(),
        1
    );
    assert!(!database.is_trading_day(&date).unwrap());

    assert_eq!(
        database
            .upsert_calendar(&[CalendarEntry {
                date,
                is_open: true,
            }])
            .unwrap(),
        1
    );
    assert!(database.is_trading_day(&date).unwrap());
}

#[test]
fn insert_calendar_rejects_duplicate_dates() {
    let mut database = DataBase::new(":memory:");
    let entry = CalendarEntry {
        date: "2026-06-15".parse().unwrap(),
        is_open: true,
    };

    database
        .insert_calendar(std::slice::from_ref(&entry))
        .unwrap();

    assert!(database.insert_calendar(&[entry]).is_err());
}

#[test]
fn insert_and_upsert_bars() {
    let mut database = DataBase::new(":memory:");
    let date: Date = "2026-06-15".parse().unwrap();
    database
        .insert_calendar(&[CalendarEntry {
            date,
            is_open: true,
        }])
        .unwrap();

    assert_eq!(
        database
            .insert_bars(&[bar("AAPL", "2026-06-15", "10")])
            .unwrap(),
        1
    );
    assert_eq!(
        database
            .upsert_bars(&[bar("AAPL", "2026-06-15", "12.5")])
            .unwrap(),
        1
    );

    let stored = database.get_bar("AAPL", &date).unwrap();
    assert_eq!(stored.ohlc.close.to_string(), "12.5000");
    assert_eq!(stored.volume.to_string(), "1000.0000");
    assert_eq!(stored.amount.to_string(), "12500.0000");
}

#[test]
fn insert_bars_rejects_duplicate_business_keys() {
    let mut database = DataBase::new(":memory:");
    database
        .insert_calendar(&[CalendarEntry {
            date: "2026-06-15".parse().unwrap(),
            is_open: true,
        }])
        .unwrap();
    let value = bar("AAPL", "2026-06-15", "10");

    database.insert_bars(std::slice::from_ref(&value)).unwrap();

    assert!(database.insert_bars(&[value]).is_err());
}

#[test]
fn delete_bar_removes_one_symbol_date() {
    let mut database = database_with_calendar();

    assert_eq!(database.delete_bar("AAPL", &date("2026-06-12")).unwrap(), 1);
    assert!(database.get_bar("AAPL", &date("2026-06-12")).is_err());
    assert!(database.get_bar("MSFT", &date("2026-06-12")).is_ok());
    assert_eq!(database.delete_bar("AAPL", &date("2026-06-12")).unwrap(), 0);
}

#[test]
fn delete_history_removes_one_symbols_interval() {
    let mut database = database_with_calendar();

    assert_eq!(
        database
            .delete_history("AAPL", &date("2026-06-12"), &date("2026-06-16"))
            .unwrap(),
        2
    );

    assert!(
        database
            .get_history("AAPL", &date("2026-06-12"), &date("2026-06-16"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        database
            .get_history("MSFT", &date("2026-06-12"), &date("2026-06-16"))
            .unwrap()
            .len(),
        2
    );
}

#[test]
#[should_panic(expected = "start date 2026-06-15 is after end date 2026-06-12")]
fn delete_history_rejects_reversed_interval() {
    let mut database = database_with_calendar();

    let _ = database
        .delete_history("AAPL", &date("2026-06-15"), &date("2026-06-12"))
        .unwrap();
}

#[test]
fn delete_section_removes_all_bars_on_one_date() {
    let mut database = database_with_calendar();

    assert_eq!(database.delete_section(&date("2026-06-12")).unwrap(), 2);
    assert!(
        database
            .get_cross_section(&date("2026-06-12"))
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        database
            .get_cross_section(&date("2026-06-15"))
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn delete_calendar_removes_unreferenced_calendar_entries() {
    let mut database = database_with_calendar();

    assert_eq!(database.delete_calendar(&date("2026-06-13")).unwrap(), 1);
    assert!(database.is_trading_day(&date("2026-06-13")).is_err());
    assert_eq!(database.delete_calendar(&date("2026-06-13")).unwrap(), 0);
}

#[test]
fn delete_calendar_rejects_dates_referenced_by_bars() {
    let mut database = database_with_calendar();

    assert!(database.delete_calendar(&date("2026-06-12")).is_err());
    assert!(database.is_trading_day(&date("2026-06-12")).unwrap());
}

#[test]
fn empty_insert_and_upsert_batches_are_noops() {
    let mut database = DataBase::new(":memory:");

    assert_eq!(database.insert_calendar(&[]).unwrap(), 0);
    assert_eq!(database.upsert_calendar(&[]).unwrap(), 0);
    assert_eq!(database.insert_bars(&[]).unwrap(), 0);
    assert_eq!(database.upsert_bars(&[]).unwrap(), 0);
}

#[test]
fn insert_from_persists_all_fetched_batches() {
    let mut database = DataBase::new(":memory:");
    let mut fetcher = BatchFetcher::new(vec![
        vec![CalendarEntry {
            date: "2026-06-15".parse().unwrap(),
            is_open: true,
        }],
        vec![CalendarEntry {
            date: "2026-06-16".parse().unwrap(),
            is_open: false,
        }],
    ]);

    assert_eq!(database.insert_from(&mut fetcher).unwrap(), 2);
    assert!(database.is_trading_day(&date("2026-06-15")).unwrap());
    assert!(!database.is_trading_day(&date("2026-06-16")).unwrap());
}

#[test]
fn upsert_from_uses_the_items_conflict_policy() {
    let mut database = DataBase::new(":memory:");
    let date: Date = "2026-06-15".parse().unwrap();
    database
        .insert_calendar(&[CalendarEntry {
            date,
            is_open: true,
        }])
        .unwrap();
    database
        .insert_bars(&[bar("AAPL", "2026-06-15", "10")])
        .unwrap();
    let mut fetcher = BatchFetcher::new(vec![vec![bar("AAPL", "2026-06-15", "12.5")]]);

    assert_eq!(database.upsert_from(&mut fetcher).unwrap(), 1);
    assert_eq!(
        database
            .get_bar("AAPL", &date)
            .unwrap()
            .ohlc
            .close
            .to_string(),
        "12.5000"
    );
}

#[test]
fn persist_from_rejects_an_empty_fetch_batch() {
    let mut database = DataBase::new(":memory:");
    let mut fetcher = BatchFetcher::<CalendarEntry>::new(vec![vec![]]);

    let error = database.insert_from(&mut fetcher).unwrap_err();

    assert!(matches!(error, Error::Fetch(FetchError::EmptyBatch)));
}

struct FailingFetcher {
    fetch_count: usize,
}

impl Fetcher for FailingFetcher {
    type Item = CalendarEntry;

    fn fetch(&mut self) -> Result<Option<Vec<Self::Item>>> {
        self.fetch_count += 1;
        if self.fetch_count == 1 {
            return Ok(Some(vec![CalendarEntry {
                date: "2026-06-15".parse().unwrap(),
                is_open: true,
            }]));
        }

        Err(FetchError::InvalidField {
            row: 2,
            field: "is_open",
            value: "invalid".into(),
        }
        .into())
    }
}

#[test]
fn persist_from_keeps_batches_committed_before_a_later_fetch_error() {
    let mut database = DataBase::new(":memory:");
    let mut fetcher = FailingFetcher { fetch_count: 0 };

    assert!(database.insert_from(&mut fetcher).is_err());
    assert!(database.is_trading_day(&date("2026-06-15")).unwrap());
}

#[test]
fn is_trading_day_returns_calendar_status() {
    let mut database = database_with_calendar();

    assert!(database.is_trading_day(&date("2026-06-15")).unwrap());
    assert!(!database.is_trading_day(&date("2026-06-14")).unwrap());
}

#[test]
fn is_trading_day_errors_when_calendar_data_is_missing() {
    let mut database = database_with_calendar();
    let date: Date = "2026-06-11".parse().unwrap();

    let error = database.is_trading_day(&date).unwrap_err();

    assert!(matches!(
        error,
        Error::Lookup(LookupError::CalendarDate { date })
            if date.to_string() == "2026-06-11"
    ));
}

#[test]
fn get_trading_day_returns_open_dates_in_inclusive_interval() {
    let mut database = database_with_calendar();

    let dates = database
        .get_trading_days(&date("2026-06-12"), &date("2026-06-15"))
        .unwrap();

    assert_eq!(
        dates,
        vec!["2026-06-12".parse().unwrap(), "2026-06-15".parse().unwrap(),]
    );
}

#[test]
fn get_trading_day_returns_empty_when_no_open_dates_exist() {
    let mut database = database_with_calendar();

    let dates = database
        .get_trading_days(&date("2026-06-13"), &date("2026-06-14"))
        .unwrap();

    assert!(dates.is_empty());
}

#[test]
#[should_panic(expected = "start date 2026-06-15 is after end date 2026-06-12")]
fn get_trading_day_rejects_reversed_interval() {
    let mut database = database_with_calendar();

    let _ = database
        .get_trading_days(&date("2026-06-15"), &date("2026-06-12"))
        .unwrap();
}

#[test]
fn next_trading_day_uses_a_strict_boundary() {
    let mut database = database_with_calendar();

    let date = database
        .next_trading_day(&date("2026-06-12"))
        .unwrap()
        .unwrap();

    assert_eq!(date.to_string(), "2026-06-15");
}

#[test]
fn next_trading_day_returns_none_when_none_exists() {
    let mut database = database_with_calendar();

    let date = database.next_trading_day(&date("2026-06-16")).unwrap();

    assert!(date.is_none());
}

#[test]
fn previous_trading_day_uses_a_strict_boundary() {
    let mut database = database_with_calendar();

    let date = database
        .previous_trading_day(&date("2026-06-15"))
        .unwrap()
        .unwrap();

    assert_eq!(date.to_string(), "2026-06-12");
}

#[test]
fn previous_trading_day_returns_none_when_none_exists() {
    let mut database = database_with_calendar();

    let date = database.previous_trading_day(&date("2026-06-12")).unwrap();

    assert!(date.is_none());
}

#[test]
fn available_symbols_returns_sorted_distinct_values() {
    let mut database = database_with_calendar();

    let symbols = database.available_symbols().unwrap();

    assert_eq!(symbols, vec!["AAPL", "MSFT"]);
}

#[test]
fn get_bar_returns_one_symbol_date() {
    let mut database = database_with_calendar();

    let bar = database.get_bar("AAPL", &date("2026-06-12")).unwrap();

    assert_eq!(bar.symbol, "AAPL");
    assert_eq!(bar.date.to_string(), "2026-06-12");
    assert_eq!(bar.ohlc.close.to_string(), "10.5000");
}

#[test]
fn get_bar_returns_a_typed_lookup_error_when_missing() {
    let mut database = database_with_calendar();

    let error = database.get_bar("GOOG", &date("2026-06-12")).unwrap_err();

    assert!(matches!(
        error,
        Error::Lookup(LookupError::Bar {
            symbol,
            date,
        }) if symbol == "GOOG" && date.to_string() == "2026-06-12"
    ));
}

#[test]
fn get_cross_section_returns_all_symbols_for_one_date() {
    let mut database = database_with_calendar();

    let bars = database.get_cross_section(&date("2026-06-12")).unwrap();

    assert_eq!(bars.len(), 2);
    assert_eq!(bars["AAPL"].symbol, "AAPL");
    assert_eq!(bars["MSFT"].symbol, "MSFT");
}

#[test]
fn get_history_returns_one_symbols_ordered_history() {
    let mut database = database_with_calendar();

    let bars = database
        .get_history("AAPL", &date("2026-06-12"), &date("2026-06-16"))
        .unwrap();

    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].date.to_string(), "2026-06-12");
    assert_eq!(bars[1].date.to_string(), "2026-06-16");
    assert!(bars.iter().all(|bar| bar.symbol == "AAPL"));
}

#[test]
#[should_panic(expected = "start date 2026-06-15 is after end date 2026-06-12")]
fn get_history_rejects_reversed_interval() {
    let mut database = database_with_calendar();

    let _ = database
        .get_history("AAPL", &date("2026-06-15"), &date("2026-06-12"))
        .unwrap();
}
