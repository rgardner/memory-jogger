//! Memory Jogger error types.

use std::io;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum PocketCleanerError {
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("validation error on field: {}", reason)]
    UserValidation { reason: String },
    #[error("faulty logic: {0}")]
    Logic(String),
    #[error("unknown IO error")]
    Io(#[from] io::Error),
    #[error("unknown error: {0}")]
    Unknown(String),
}

pub type Result<T> = std::result::Result<T, PocketCleanerError>;
