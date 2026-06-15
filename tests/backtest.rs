use goose::{data::Price, engine::Account};

fn price(value: &str) -> Price {
    value.parse().unwrap()
}

#[test]
fn apply_fill_updates_cash_and_position_for_buys_and_sells() {
    let mut account = Account::new(price("100.0000"));

    assert!(
        account
            .apply_fill("AAPL".into(), price("10.0000"), 5, price("1.0000"))
            .unwrap()
    );
    assert_eq!(account.cash(), price("49.0000"));
    assert_eq!(account.position("AAPL"), 5);

    assert!(
        account
            .apply_fill("AAPL".into(), price("10.0000"), -2, price("1.0000"))
            .unwrap()
    );
    assert_eq!(account.cash(), price("68.0000"));
    assert_eq!(account.position("AAPL"), 3);
}

#[test]
fn apply_fill_discards_an_unaffordable_order_without_mutation() {
    let mut account = Account::new(price("100.0000"));

    assert!(
        !account
            .apply_fill("AAPL".into(), price("20.0000"), 5, price("1.0000"))
            .unwrap()
    );
    assert_eq!(account.cash(), price("100.0000"));
    assert_eq!(account.position("AAPL"), 0);
}
