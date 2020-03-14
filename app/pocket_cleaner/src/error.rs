//! A module for working with Pocket Cleaner errors.

use std::io;

use actix_http::ResponseBuilder;
use actix_web::{
    http::{header, StatusCode},
    HttpResponse,
};
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

impl actix_web::error::ResponseError for PocketCleanerError {
    fn error_response(&self) -> HttpResponse {
        ResponseBuilder::new(self.status_code())
            .set_header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(self.to_string())
    }

    fn status_code(&self) -> StatusCode {
        match *self {
            Self::UserValidation { .. } | Self::InvalidArgument(_) => StatusCode::BAD_REQUEST,
            Self::Logic(_) | Self::Io(_) | Self::Unknown(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

pub type Result<T> = std::result::Result<T, PocketCleanerError>;
