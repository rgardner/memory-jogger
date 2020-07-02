use std::env;

use crate::error::{PocketCleanerError, Result};

// Pocket constants
pub static POCKET_CONSUMER_KEY_ENV_VAR: &str = "POCKET_CLEANER_CONSUMER_KEY";

// Email constants
pub static SENDGRID_API_KEY_ENV_VAR: &str = "POCKET_CLEANER_SENDGRID_API_KEY";
pub static FROM_EMAIL_ENV_VAR: &str = "POCKET_CLEANER_FROM_EMAIL";

// Database constants
pub static DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";

pub struct AppConfig {
    pub pocket_consumer_key: String,
}

pub fn get_required_env_var(key: &str) -> Result<String> {
    env::var(key)
        .map_err(|_| PocketCleanerError::Unknown(format!("missing app config env var: {}", key)))
}
