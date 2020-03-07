// As of Rust 1.34.0, these dependencies need to be declared in this order using
// `extern crate` in your `main.rs` file. See
// https://github.com/emk/rust-musl-builder/issues/69.
extern crate openssl;
#[macro_use]
extern crate diesel;

use std::env;

use crate::error::{PocketCleanerError, Result};

pub mod config;
pub mod db;
pub mod email;
pub mod error;
pub mod pocket;
pub mod trends;
pub mod view;

pub fn get_required_env_var(key: &str) -> Result<String> {
    env::var(key)
        .map_err(|_| PocketCleanerError::Unknown(format!("missing app config env var: {}", key)))
}
