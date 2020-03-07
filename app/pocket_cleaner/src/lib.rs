extern crate openssl;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

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
