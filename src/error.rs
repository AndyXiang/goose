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
}

pub type Result<T> = std::result::Result<T, Error>;
