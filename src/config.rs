use std::env;

use crate::error::{Error, Result};

// Pocket constants
pub static POCKET_CONSUMER_KEY_ENV_VAR: &str = "MEMORY_JOGGER_POCKET_CONSUMER_KEY";

// Email constants
pub static SENDGRID_API_KEY_ENV_VAR: &str = "MEMORY_JOGGER_SENDGRID_API_KEY";
pub static FROM_EMAIL_ENV_VAR: &str = "MEMORY_JOGGER_FROM_EMAIL";

pub fn get_required_env_var(key: &str) -> Result<String> {
    env::var(key).map_err(|_| Error::Unknown(format!("missing app config env var: {}", key)))
}
