use goose::data::{DateBar, PriceAdjust};

#[test]
fn date_bar_new_accepts_valid_and_partial_prices() {
    let bar = DateBar::new(
        "AAPL",
        "2026-06-15".parse().unwrap(),
        PriceAdjust::Raw,
        Some("10".parse().unwrap()),
        Some("11".parse().unwrap()),
        Some("9".parse().unwrap()),
        Some("10.5".parse().unwrap()),
    )
    .unwrap();

    assert_eq!(bar.symbol, "AAPL");
    assert_eq!(bar.close.unwrap().to_string(), "10.5000");

    assert!(
        DateBar::new(
            "MSFT",
            "2026-06-15".parse().unwrap(),
            PriceAdjust::Raw,
            None,
            None,
            None,
            None,
        )
        .is_ok()
    );
}

#[test]
fn date_bar_new_rejects_empty_symbols_and_invalid_ohlc() {
    let date = "2026-06-15".parse().unwrap();

    assert!(DateBar::new("  ", date, PriceAdjust::Raw, None, None, None, None).is_err());
    assert!(
        DateBar::new(
            "AAPL",
            date,
            PriceAdjust::Raw,
            Some("10".parse().unwrap()),
            Some("9".parse().unwrap()),
            Some("11".parse().unwrap()),
            Some("10".parse().unwrap()),
        )
        .is_err()
    );
    assert!(
        DateBar::new(
            "AAPL",
            date,
            PriceAdjust::Raw,
            Some("12".parse().unwrap()),
            Some("11".parse().unwrap()),
            Some("9".parse().unwrap()),
            Some("10".parse().unwrap()),
        )
        .is_err()
    );
}
