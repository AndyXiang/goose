use goose::data::Quantity;
use rust_decimal::Decimal;

#[test]
fn quantity_parses_and_has_canonical_scale() {
    let quantity: Quantity = "12.34".parse().unwrap();

    assert_eq!(quantity.to_string(), "12.3400");
    assert_eq!(Decimal::from(quantity), Decimal::new(123_400, 4));
}

#[test]
fn quantity_rejects_excess_precision_and_out_of_range_values() {
    assert!("12.34567".parse::<Quantity>().is_err());
    assert!("922337203685477.5808".parse::<Quantity>().is_err());
}

#[test]
fn quantity_reports_its_sign() {
    let negative: Quantity = "-0.0001".parse().unwrap();
    let zero: Quantity = "0.0000".parse().unwrap();
    let positive: Quantity = "0.0001".parse().unwrap();

    assert!(negative.is_negative());
    assert!(!negative.is_positive());
    assert!(!zero.is_negative());
    assert!(!zero.is_positive());
    assert!(positive.is_positive());
    assert!(!positive.is_negative());
}

#[test]
fn quantity_arithmetic_preserves_valid_values() {
    let mut quantity: Quantity = "10.0000".parse().unwrap();
    let adjustment: Quantity = "2.5000".parse().unwrap();

    quantity += adjustment;
    assert_eq!(quantity.to_string(), "12.5000");

    quantity -= "0.5000".parse().unwrap();
    assert_eq!(quantity.to_string(), "12.0000");

    assert_eq!((quantity * Decimal::new(15, 1)).to_string(), "18.0000");
    assert_eq!((quantity / Decimal::new(2, 0)).to_string(), "6.0000");
}

#[test]
fn checked_operations_report_invalid_results() {
    let max: Quantity = "922337203685477.5807".parse().unwrap();
    let smallest: Quantity = "0.0001".parse().unwrap();
    let one: Quantity = "1.0000".parse().unwrap();

    assert!(max.checked_add(smallest).is_none());
    assert!(one.checked_div(Decimal::new(3, 0)).is_none());
    assert!(one.checked_div(Decimal::ZERO).is_none());
}

#[test]
fn dividing_quantities_returns_a_ratio() {
    let numerator: Quantity = "12.0000".parse().unwrap();
    let denominator: Quantity = "8.0000".parse().unwrap();

    assert_eq!(numerator / denominator, Decimal::new(15, 1));
}
