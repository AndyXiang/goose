use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Error converting file path to string.")]
    InvalidPath(PathBuf)
}

impl From<rusqlite::Error> for Error {
    fn from(error: rusqlite::Error) -> Error {
        use rusqlite::Error::*;
        match error {
            InvalidPath(p) => Error::InvalidPath(p),
            _ => panic!("unimplemented rusqlite error transformation!"),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

