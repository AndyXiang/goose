use goose::data::{DateBar, Ohlc, PriceAdjust};

#[test]
fn date_bar_new_accepts_valid_and_partial_prices() {
    let ohlc = Ohlc::new(
        PriceAdjust::Raw,
        Some("10".parse().unwrap()),
        Some("11".parse().unwrap()),
        Some("9".parse().unwrap()),
        Some("10.5".parse().unwrap()),
    )
    .unwrap();
    let bar = DateBar::new(
        "AAPL",
        "2026-06-15".parse().unwrap(),
        ohlc,
        Some("1000".parse().unwrap()),
        Some("10500".parse().unwrap()),
    )
    .unwrap();

    assert_eq!(bar.symbol, "AAPL");
    assert_eq!(bar.ohlc.close.unwrap().to_string(), "10.5000");
    assert_eq!(bar.volume.unwrap().to_string(), "1000.0000");
    assert_eq!(bar.amount.unwrap().to_string(), "10500.0000");

    assert!(
        DateBar::new(
            "MSFT",
            "2026-06-15".parse().unwrap(),
            Ohlc::new(PriceAdjust::Raw, None, None, None, None).unwrap(),
            None,
            None,
        )
        .is_ok()
    );
}

#[test]
fn date_bar_new_rejects_empty_symbols() {
    let date = "2026-06-15".parse().unwrap();
    let ohlc = Ohlc::new(PriceAdjust::Raw, None, None, None, None).unwrap();

    assert!(DateBar::new("  ", date, ohlc, None, None).is_err());
}

#[test]
fn ohlc_new_rejects_invalid_ohlc() {
    assert!(
        Ohlc::new(
            PriceAdjust::Raw,
            Some("10".parse().unwrap()),
            Some("9".parse().unwrap()),
            Some("11".parse().unwrap()),
            Some("10".parse().unwrap()),
        )
        .is_err()
    );
    assert!(
        Ohlc::new(
            PriceAdjust::Raw,
            Some("12".parse().unwrap()),
            Some("11".parse().unwrap()),
            Some("9".parse().unwrap()),
            Some("10".parse().unwrap()),
        )
        .is_err()
    );
}
