use goose::data::{CsvBarFetcher, CsvCalendarFetcher, Fetcher, PriceAdjust};
use goose::error::{Error, FetchError};
use std::io::Cursor;

const CSV: &str = "symbol,date,price_adjust,open,high,low,close,volume,amount
AAPL,2026-06-12,raw,10,11,9.5,10.5,1000,10500
MSFT,2026-06-12,qfq,20,21,19.5,20.5,2000,41000
GOOG,2026-06-15,hfq,,,,,,
";

#[test]
fn csv_fetcher_returns_validated_batches_until_eof() {
    let mut fetcher = CsvBarFetcher::from_reader(Cursor::new(CSV), 2).unwrap();

    let first = fetcher.fetch().unwrap().unwrap();
    assert_eq!(first.len(), 2);
    assert_eq!(first[0].symbol, "AAPL");
    assert_eq!(first[0].ohlc.close.unwrap().to_string(), "10.5000");
    assert_eq!(first[0].volume.unwrap().to_string(), "1000.0000");
    assert_eq!(first[0].amount.unwrap().to_string(), "10500.0000");
    assert_eq!(first[1].ohlc.price_adjust, PriceAdjust::Qfq);

    let second = fetcher.fetch().unwrap().unwrap();
    assert_eq!(second.len(), 1);
    assert_eq!(second[0].symbol, "GOOG");
    assert!(second[0].ohlc.open.is_none());
    assert!(fetcher.fetch().unwrap().is_none());
}

#[test]
fn csv_fetcher_rejects_invalid_headers_and_batch_size() {
    let invalid_headers = "date,symbol,price_adjust,open,high,low,close,volume,amount\n";

    let error = match CsvBarFetcher::from_reader(Cursor::new(invalid_headers), 1) {
        Ok(_) => panic!("invalid headers were accepted"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        Error::Fetch(FetchError::InvalidHeaders { .. })
    ));
}

#[test]
#[should_panic(expected = "CSV fetcher batch size must be greater than zero")]
fn csv_bar_fetcher_rejects_zero_batch_size() {
    let _ = CsvBarFetcher::from_reader(Cursor::new(CSV), 0);
}

#[test]
fn csv_fetcher_rejects_invalid_domain_values() {
    let invalid_date = "symbol,date,price_adjust,open,high,low,close,volume,amount
AAPL,2026-02-29,raw,10,11,9,10,1000,10000
";
    let invalid_ohlc = "symbol,date,price_adjust,open,high,low,close,volume,amount
AAPL,2026-06-12,raw,10,9,11,10,1000,10000
";

    let mut date_fetcher = CsvBarFetcher::from_reader(Cursor::new(invalid_date), 1).unwrap();
    assert!(date_fetcher.fetch().is_err());

    let mut ohlc_fetcher = CsvBarFetcher::from_reader(Cursor::new(invalid_ohlc), 1).unwrap();
    assert!(ohlc_fetcher.fetch().is_err());
}

#[test]
fn csv_calendar_fetcher_returns_batches_and_parses_booleans() {
    let csv = "date,is_open
2026-06-12,true
2026-06-13,FALSE
2026-06-14,1
2026-06-15,0
";
    let mut fetcher = CsvCalendarFetcher::from_reader(Cursor::new(csv), 3).unwrap();

    let first = fetcher.fetch().unwrap().unwrap();
    assert_eq!(first.len(), 3);
    assert_eq!(first[0].date.to_string(), "2026-06-12");
    assert!(first[0].is_open);
    assert!(!first[1].is_open);
    assert!(first[2].is_open);

    let second = fetcher.fetch().unwrap().unwrap();
    assert_eq!(second.len(), 1);
    assert!(!second[0].is_open);
    assert!(fetcher.fetch().unwrap().is_none());
}

#[test]
fn csv_calendar_fetcher_rejects_invalid_input() {
    let invalid_headers = "is_open,date\ntrue,2026-06-12\n";
    let invalid_date = "date,is_open\n2026-02-29,true\n";
    let invalid_boolean = "date,is_open\n2026-06-12,open\n";

    assert!(CsvCalendarFetcher::from_reader(Cursor::new(invalid_headers), 1).is_err());

    let mut date_fetcher = CsvCalendarFetcher::from_reader(Cursor::new(invalid_date), 1).unwrap();
    assert!(date_fetcher.fetch().is_err());

    let mut bool_fetcher =
        CsvCalendarFetcher::from_reader(Cursor::new(invalid_boolean), 1).unwrap();
    assert!(bool_fetcher.fetch().is_err());
}

#[test]
#[should_panic(expected = "CSV fetcher batch size must be greater than zero")]
fn csv_calendar_fetcher_rejects_zero_batch_size() {
    let _ = CsvCalendarFetcher::from_reader(Cursor::new("date,is_open\n"), 0);
}
