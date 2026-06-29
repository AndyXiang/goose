use goose::data::{DateBar, Ohlc, Price, Quantity};

fn price(value: &str) -> Price {
    value.parse().unwrap()
}

fn quantity(value: &str) -> Quantity {
    value.parse().unwrap()
}

#[test]
fn date_bar_new_accepts_valid_prices() {
    let ohlc = Ohlc::new(price("10"), price("11"), price("9"), price("10.5")).unwrap();
    let bar = DateBar::new(
        "AAPL",
        "2026-06-15".parse().unwrap(),
        ohlc,
        quantity("1000"),
        price("10500"),
    )
    .unwrap();

    assert_eq!(bar.symbol, "AAPL");
    assert_eq!(bar.ohlc.close.to_string(), "10.5000");
    assert_eq!(bar.volume.to_string(), "1000.0000");
    assert_eq!(bar.amount.to_string(), "10500.0000");
}

#[test]
fn date_bar_new_rejects_empty_symbols() {
    let date = "2026-06-15".parse().unwrap();
    let ohlc = Ohlc::new(price("10"), price("11"), price("9"), price("10.5")).unwrap();

    assert!(DateBar::new("  ", date, ohlc, quantity("1000"), price("10500")).is_err());
}

#[test]
fn ohlc_new_rejects_invalid_ohlc() {
    assert!(Ohlc::new(price("10"), price("9"), price("11"), price("10")).is_err());
    assert!(Ohlc::new(price("12"), price("11"), price("9"), price("10")).is_err());
}
