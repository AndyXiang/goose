use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Invalid path `{0}` to database")]
    InvaildPath(PathBuf),
    #[error("Fail connecting to `{0}`")]
    FailConnectingDatabase(PathBuf),
    #[error("invalid date: {0}")]
    InvalidDate(String),
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error("No trading calendar data for {0}")]
    MissingCalendarDate(String),
    #[error("No trading day after {0}")]
    MissingTradingDayAfter(String),
    #[error("No trading day before {0}")]
    MissingTradingDayBefore(String),
    #[error("Invalid date interval: start {start} is after end {end}")]
    InvalidDateInterval { start: String, end: String },
    #[error("Database error: {0}")]
    Database(#[from] diesel::result::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
