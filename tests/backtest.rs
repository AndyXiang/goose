use goose::{
    data::{Price, Quantity},
    engine::Account,
};

fn price(value: &str) -> Price {
    value.parse().unwrap()
}

fn quantity(value: &str) -> Quantity {
    value.parse().unwrap()
}

#[test]
fn apply_fill_updates_cash_and_position_for_buys_and_sells() {
    let mut account = Account::new(price("100.0000"));

    account
        .apply_fill(
            "AAPL".into(),
            price("10.0000"),
            quantity("5.0000"),
            price("1.0000"),
        )
        .unwrap();
    assert_eq!(account.cash(), price("49.0000"));
    assert_eq!(account.position("AAPL"), quantity("5.0000"));

    account
        .apply_fill(
            "AAPL".into(),
            price("10.0000"),
            quantity("-2.0000"),
            price("1.0000"),
        )
        .unwrap();
    assert_eq!(account.cash(), price("68.0000"));
    assert_eq!(account.position("AAPL"), quantity("3.0000"));
}

#[test]
fn apply_fill_allows_negative_cash_for_broker_approved_fills() {
    let mut account = Account::new(price("100.0000"));

    account
        .apply_fill(
            "AAPL".into(),
            price("20.0000"),
            quantity("5.0000"),
            price("1.0000"),
        )
        .unwrap();

    assert_eq!(account.cash(), price("-1.0000"));
    assert_eq!(account.position("AAPL"), quantity("5.0000"));
}
