//! A module for interacting with Pocket Cleaner's Database.

use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::{
    config,
    db::models::{NewSavedItem, NewUser, SavedItem, User},
    error::{PocketCleanerError, Result},
};

pub mod models;
pub mod schema;

embed_migrations!();

pub fn establish_connection(database_url: &str) -> Result<PgConnection> {
    PgConnection::establish(&database_url).map_err(|e| {
        PocketCleanerError::Unknown(format!("Error connecting to {}: {}", database_url, e))
    })
}

pub fn run_migrations(connection: &PgConnection) -> Result<()> {
    embedded_migrations::run_with_output(connection, &mut std::io::stdout())
        .map_err(|e| PocketCleanerError::Unknown(format!("Failed to run migrations: {}", e)))
}

/// Connect to the database and run migrations.
pub fn initialize_db() -> Result<PgConnection> {
    let database_url = config::get_required_env_var(config::DATABASE_URL_ENV_VAR)?;
    let conn = establish_connection(&database_url)?;
    run_migrations(&conn)?;
    Ok(conn)
}

pub fn create_user<'a>(
    conn: &PgConnection,
    email: &'a str,
    pocket_access_token: Option<&'a str>,
) -> Result<User> {
    use crate::db::schema::users;

    let new_user = NewUser {
        email,
        pocket_access_token,
    };

    diesel::insert_into(users::table)
        .values(&new_user)
        .get_result(conn)
        .map_err(|e| PocketCleanerError::Unknown(format!("Error saving new saved item: {}", e)))
}

pub fn create_saved_item<'a>(
    conn: &PgConnection,
    user_id: i32,
    pocket_id: &'a str,
    title: &'a str,
    body: &'a str,
) -> Result<SavedItem> {
    use crate::db::schema::saved_items;

    let new_post = NewSavedItem {
        user_id,
        pocket_id,
        title,
        body,
    };

    diesel::insert_into(saved_items::table)
        .values(&new_post)
        .get_result(conn)
        .map_err(|e| PocketCleanerError::Unknown(format!("Error saving new saved item: {}", e)))
}
