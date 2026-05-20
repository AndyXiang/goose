#[derive(Debug)]
pub enum ErrorKind {
    Db,
    Data,
}

#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    msg: String,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.msg)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;

impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error {
            kind: ErrorKind::Db,
            msg: err.to_string(),
        }
    }
}

impl From<rust_decimal::Error> for Error {
    fn from(err: rust_decimal::Error) -> Self {
        Error {
            kind: ErrorKind::Data,
            msg: err.to_string(),
        }
    }
}
