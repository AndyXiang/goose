use goose::data::PriceAdjust;
use std::collections::HashSet;

#[test]
fn price_adjust_parses_and_formats_database_values() {
    for (value, expected) in [
        ("raw", PriceAdjust::Raw),
        ("qfq", PriceAdjust::Qfq),
        ("hfq", PriceAdjust::Hfq),
    ] {
        let adjustment: PriceAdjust = value.parse().unwrap();

        assert_eq!(adjustment, expected);
        assert_eq!(adjustment.to_string(), value);
    }
}

#[test]
fn price_adjust_rejects_unknown_values() {
    assert!("split-adjusted".parse::<PriceAdjust>().is_err());
}

#[test]
fn price_adjust_supports_hash_collections() {
    let adjustments = HashSet::from([
        PriceAdjust::Raw,
        PriceAdjust::Qfq,
        PriceAdjust::Hfq,
        PriceAdjust::Raw,
    ]);

    assert_eq!(adjustments.len(), 3);
}
