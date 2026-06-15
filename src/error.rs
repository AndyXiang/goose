use crate::data::{Date, Price, PriceAdjust};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Database(#[from] DatabaseError),
    #[error(transparent)]
    Validation(#[from] ValidationError),
    #[error(transparent)]
    Fetch(#[from] FetchError),
    #[error(transparent)]
    Lookup(#[from] LookupError),
    #[error(transparent)]
    Backtest(#[from] BacktestError),
}

#[derive(thiserror::Error, Debug)]
pub enum DatabaseError {
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum ValidationError {
    #[error("invalid date: {value}")]
    Date { value: String },
    #[error("invalid price: {reason}")]
    Price { reason: String },
    #[error("invalid price adjustment: {value}")]
    PriceAdjust { value: String },
    #[error("symbol must not be empty")]
    EmptySymbol,
    #[error("invalid OHLC data: {reason}")]
    Ohlc { reason: String },
}

#[derive(thiserror::Error, Debug)]
pub enum FetchError {
    #[error("CSV error: {0}")]
    Csv(#[from] csv::Error),
    #[error("fetcher returned an empty batch; return None when exhausted")]
    EmptyBatch,
    #[error("invalid CSV headers: expected {expected}, found {actual}")]
    InvalidHeaders { expected: String, actual: String },
    #[error("invalid CSV record at row {row}: {source}")]
    InvalidRecord {
        row: u64,
        #[source]
        source: ValidationError,
    },
    #[error("CSV row {row} is missing field `{field}`")]
    MissingField { row: u64, field: &'static str },
    #[error("CSV row {row} has invalid `{field}` value: {value}")]
    InvalidField {
        row: u64,
        field: &'static str,
        value: String,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum LookupError {
    #[error("no calendar data for {date}")]
    CalendarDate { date: Date },
    #[error("no bar for {symbol} on {date} using {adjustment}")]
    Bar {
        symbol: String,
        date: Date,
        adjustment: PriceAdjust,
    },
}

#[derive(thiserror::Error, Debug)]
pub enum BacktestError {
    #[error("initial cash must not be negative: {cash}")]
    NegativeInitialCash { cash: Price },
    #[error("order symbol must not be empty")]
    EmptyOrderSymbol,
    #[error("order quantity must be greater than zero")]
    ZeroOrderQuantity,
    #[error("cash arithmetic overflow")]
    CashOverflow,
    #[error("position arithmetic overflow for {symbol}")]
    PositionOverflow { symbol: String },
    #[error("insufficient cash: required {required}, available {available}")]
    InsufficientCash { required: Price, available: Price },
    #[error(
        "insufficient position for {symbol}: requested {requested} shares, available {available}"
    )]
    InsufficientPosition {
        symbol: String,
        requested: u32,
        available: u32,
    },
}

impl From<diesel::result::Error> for Error {
    fn from(error: diesel::result::Error) -> Self {
        DatabaseError::from(error).into()
    }
}

impl From<csv::Error> for Error {
    fn from(error: csv::Error) -> Self {
        FetchError::from(error).into()
    }
}

pub type Result<T> = std::result::Result<T, Error>;
