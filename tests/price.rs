use goose::data::{Price, Quantity};
use rust_decimal::Decimal;

#[test]
fn price_parses_and_has_canonical_scale() {
    let price: Price = "12.34".parse().unwrap();

    assert_eq!(price.to_string(), "12.3400");
    assert_eq!(Decimal::from(price), Decimal::new(123_400, 4));
}

#[test]
fn price_rejects_excess_precision_and_out_of_range_values() {
    assert!("12.34567".parse::<Price>().is_err());
    assert!("922337203685477.5808".parse::<Price>().is_err());
}

#[test]
fn price_arithmetic_preserves_valid_prices() {
    let mut price: Price = "10.0000".parse().unwrap();
    let adjustment: Price = "2.5000".parse().unwrap();

    price += adjustment;
    assert_eq!(price.to_string(), "12.5000");

    price -= "0.5000".parse().unwrap();
    assert_eq!(price.to_string(), "12.0000");

    assert_eq!((price * Decimal::new(15, 1)).to_string(), "18.0000");
    assert_eq!((price / Decimal::new(2, 0)).to_string(), "6.0000");
}

#[test]
fn checked_operations_report_invalid_results() {
    let max: Price = "922337203685477.5807".parse().unwrap();
    let smallest: Price = "0.0001".parse().unwrap();
    let one: Price = "1.0000".parse().unwrap();

    assert!(max.checked_add(smallest).is_none());
    assert!(one.checked_div(Decimal::new(3, 0)).is_none());
    assert!(one.checked_div(Decimal::ZERO).is_none());
}

#[test]
fn dividing_prices_returns_a_ratio() {
    let numerator: Price = "12.0000".parse().unwrap();
    let denominator: Price = "8.0000".parse().unwrap();

    assert_eq!(numerator / denominator, Decimal::new(15, 1));
}

#[test]
fn price_and_quantity_arithmetic_returns_price_values() {
    let price: Price = "12.5000".parse().unwrap();
    let quantity: Quantity = "3.0000".parse().unwrap();
    let total: Price = "37.5000".parse().unwrap();

    assert_eq!(price * quantity, total);
    assert_eq!(quantity * price, total);
    assert_eq!(total / quantity, price);
}

#[test]
fn checked_price_quantity_operations_report_invalid_results() {
    let max: Price = "922337203685477.5807".parse().unwrap();
    let two: Quantity = "2.0000".parse().unwrap();
    let one: Price = "1.0000".parse().unwrap();
    let zero: Quantity = "0.0000".parse().unwrap();

    assert!(max.checked_mul_quantity(two).is_none());
    assert!(two.checked_mul_price(max).is_none());
    assert!(one.checked_div_quantity(zero).is_none());
}
