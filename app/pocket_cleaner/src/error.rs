//! A module for working with Pocket Cleaner errors.

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PocketCleanerError {
    #[error("faulty logic: {0}")]
    Logic(String),
    #[error("unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, PocketCleanerError>;
