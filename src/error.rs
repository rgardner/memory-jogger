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
        // Note: errors caused by failing to sanitize input strings is a logic error
        Self::Logic(e.to_string())
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(e: serde_json::error::Error) -> Self {
        Self::Unknown(e.to_string())
    }
}

impl From<diesel::result::Error> for Error {
    fn from(e: diesel::result::Error) -> Self {
        Self::Unknown(e.to_string())
    }
}

impl From<diesel::result::ConnectionError> for Error {
    fn from(e: diesel::result::ConnectionError) -> Self {
        Self::Unknown(e.to_string())
    }
}

impl From<diesel_migrations::RunMigrationsError> for Error {
    fn from(e: diesel_migrations::RunMigrationsError) -> Self {
        Self::Unknown(e.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
