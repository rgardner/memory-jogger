// Use old macro_use syntax for diesel because diesel does not support Rust
// 2018 syntax.
// https://www.reddit.com/r/rust/comments/b9t3c0/is_it_possible_to_use_diesel_schema_macros_with/
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
