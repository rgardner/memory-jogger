use std::env;

use crate::error::{PocketCleanerError, Result};

// Pocket constants
pub static POCKET_CONSUMER_KEY_ENV_VAR: &str = "POCKET_CLEANER_CONSUMER_KEY";
pub static POCKET_USER_ACCESS_TOKEN_ENV_VAR: &str = "POCKET_TEMP_USER_ACCESS_TOKEN";

// Email constants
pub static SENDGRID_API_KEY_ENV_VAR: &str = "POCKET_CLEANER_SENDGRID_API_KEY";
pub static FROM_EMAIL_ENV_VAR: &str = "POCKET_CLEANER_FROM_EMAIL";
pub static TO_EMAIL_ENV_VAR: &str = "POCKET_CLEANER_TO_EMAIL";

// Database constants
pub static DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";

pub struct AppConfig {
    pub pocket_consumer_key: String,
    pub pocket_user_access_token: String,
}

pub fn get_required_env_var(key: &str) -> Result<String> {
    env::var(key)
        .map_err(|_| PocketCleanerError::Unknown(format!("missing app config env var: {}", key)))
}
