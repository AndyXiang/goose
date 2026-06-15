use diesel::{Connection, SqliteConnection, connection::SimpleConnection};
use goose::data::{DataBase, Date, PriceAdjust};
use goose::error::Error;

fn database_with_calendar() -> DataBase {
    let mut conn = SqliteConnection::establish(":memory:").unwrap();
    conn.batch_execute(include_str!(
        "../migrations/2026-06-12-081652-0000_create_bar_calendar/up.sql"
    ))
    .unwrap();
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
            (symbol, date, is_adjust, open, high, low, close)
         VALUES
            ('AAPL', '2026-06-12', 'raw', 100000, 110000, 95000, 105000),
            ('AAPL', '2026-06-12', 'qfq',  90000, 100000, 85000,  95000),
            ('MSFT', '2026-06-12', 'raw', 190000, 205000, 185000, 200000),
            ('MSFT', '2026-06-15', 'raw', 200000, 210000, 195000, 205000),
            ('AAPL', '2026-06-16', 'raw', 106000, 112000, 101000, 110000);",
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
                "INSERT INTO daily_bars (symbol, date, is_adjust)
                 VALUES ('AAPL', '2026-06-15', 'raw');",
            )
            .is_err()
    );
}

#[test]
fn is_trading_day_returns_calendar_status() {
    let mut database = database_with_calendar();

    assert!(
        database
            .is_trading_day("2026-06-15".parse().unwrap())
            .unwrap()
    );
    assert!(
        !database
            .is_trading_day("2026-06-14".parse().unwrap())
            .unwrap()
    );
}

#[test]
fn is_trading_day_errors_when_calendar_data_is_missing() {
    let mut database = database_with_calendar();
    let date: Date = "2026-06-11".parse().unwrap();

    let error = database.is_trading_day(date).unwrap_err();

    assert!(matches!(
        error,
        Error::MissingCalendarDate(value) if value == "2026-06-11"
    ));
}

#[test]
fn get_trading_day_returns_open_dates_in_inclusive_interval() {
    let mut database = database_with_calendar();

    let dates = database
        .get_trading_days("2026-06-12".parse().unwrap(), "2026-06-15".parse().unwrap())
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
        .get_trading_days("2026-06-13".parse().unwrap(), "2026-06-14".parse().unwrap())
        .unwrap();

    assert!(dates.is_empty());
}

#[test]
fn get_trading_day_rejects_reversed_interval() {
    let mut database = database_with_calendar();

    let error = database
        .get_trading_days("2026-06-15".parse().unwrap(), "2026-06-12".parse().unwrap())
        .unwrap_err();

    assert!(matches!(
        error,
        Error::InvalidDateInterval { start, end }
            if start == "2026-06-15" && end == "2026-06-12"
    ));
}

#[test]
fn first_trading_day_after_uses_a_strict_boundary() {
    let mut database = database_with_calendar();

    let date = database
        .first_trading_day_after("2026-06-12".parse().unwrap())
        .unwrap();

    assert_eq!(date.to_string(), "2026-06-15");
}

#[test]
fn first_trading_day_after_errors_when_none_exists() {
    let mut database = database_with_calendar();

    let error = database
        .first_trading_day_after("2026-06-16".parse().unwrap())
        .unwrap_err();

    assert!(matches!(
        error,
        Error::MissingTradingDayAfter(value) if value == "2026-06-16"
    ));
}

#[test]
fn last_trading_day_before_uses_a_strict_boundary() {
    let mut database = database_with_calendar();

    let date = database
        .last_trading_day_before("2026-06-15".parse().unwrap())
        .unwrap();

    assert_eq!(date.to_string(), "2026-06-12");
}

#[test]
fn last_trading_day_before_errors_when_none_exists() {
    let mut database = database_with_calendar();

    let error = database
        .last_trading_day_before("2026-06-12".parse().unwrap())
        .unwrap_err();

    assert!(matches!(
        error,
        Error::MissingTradingDayBefore(value) if value == "2026-06-12"
    ));
}

#[test]
fn available_symbols_returns_sorted_distinct_values() {
    let mut database = database_with_calendar();

    let symbols = database.available_symbols().unwrap();

    assert_eq!(symbols, vec!["AAPL", "MSFT"]);
}

#[test]
fn get_bar_returns_one_symbol_date_and_adjustment() {
    let mut database = database_with_calendar();

    let bar = database
        .get_bar("AAPL", "2026-06-12".parse().unwrap(), PriceAdjust::Raw)
        .unwrap();

    assert_eq!(bar.symbol, "AAPL");
    assert_eq!(bar.date.to_string(), "2026-06-12");
    assert_eq!(bar.is_adjust, PriceAdjust::Raw);
    assert_eq!(bar.close.unwrap().to_string(), "10.5000");
}

#[test]
fn get_cross_section_returns_all_symbols_for_one_date() {
    let mut database = database_with_calendar();

    let bars = database
        .get_cross_section("2026-06-12".parse().unwrap(), PriceAdjust::Raw)
        .unwrap();

    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].symbol, "AAPL");
    assert_eq!(bars[1].symbol, "MSFT");
    assert!(bars.iter().all(|bar| bar.is_adjust == PriceAdjust::Raw));
}

#[test]
fn get_history_returns_one_symbols_ordered_history() {
    let mut database = database_with_calendar();

    let bars = database
        .get_history(
            "AAPL",
            "2026-06-12".parse().unwrap(),
            "2026-06-16".parse().unwrap(),
            PriceAdjust::Raw,
        )
        .unwrap();

    assert_eq!(bars.len(), 2);
    assert_eq!(bars[0].date.to_string(), "2026-06-12");
    assert_eq!(bars[1].date.to_string(), "2026-06-16");
    assert!(bars.iter().all(|bar| bar.symbol == "AAPL"));
}

#[test]
fn get_history_rejects_reversed_interval() {
    let mut database = database_with_calendar();

    let error = database
        .get_history(
            "AAPL",
            "2026-06-15".parse().unwrap(),
            "2026-06-12".parse().unwrap(),
            PriceAdjust::Raw,
        )
        .unwrap_err();

    assert!(matches!(error, Error::InvalidDateInterval { .. }));
}
