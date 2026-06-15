use goose::{
    data::Price,
    error::{BacktestError, Error},
};

#[test]
fn backtest_errors_convert_to_the_crate_error() {
    let error: Error = BacktestError::PositionOverflow {
        symbol: "AAPL".into(),
    }
    .into();

    assert!(matches!(
        error,
        Error::Backtest(BacktestError::PositionOverflow { symbol }) if symbol == "AAPL"
    ));
}

#[test]
fn backtest_errors_include_relevant_values() {
    let error = BacktestError::InsufficientCash {
        required: "100.0000".parse::<Price>().unwrap(),
        available: "80.0000".parse::<Price>().unwrap(),
    };

    assert_eq!(
        error.to_string(),
        "insufficient cash: required 100.0000, available 80.0000"
    );
}
