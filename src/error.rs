//! Memory Jogger error types.

use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("user did not authorize application")]
    UserPocketAuth,
    #[error("faulty logic: {0}")]
    Logic(String),
    #[error("unknown IO error")]
    Io(#[from] io::Error),
    #[error("unknown error: {0}")]
    Unknown(String),
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Self::Unknown(e.to_string())
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Self::Logic(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
