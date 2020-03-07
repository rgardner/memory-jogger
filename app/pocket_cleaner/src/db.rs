//! A module for interacting with Pocket Cleaner's Database.

use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::{
    db::models::{NewSavedItem, SavedItem},
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

pub fn create_saved_item<'a>(
    conn: &PgConnection,
    pocket_id: &'a str,
    title: &'a str,
    body: &'a str,
) -> Result<SavedItem> {
    use crate::db::schema::saved_items;

    let new_post = NewSavedItem {
        pocket_id,
        title,
        body,
    };

    diesel::insert_into(saved_items::table)
        .values(&new_post)
        .get_result(conn)
        .map_err(|e| PocketCleanerError::Unknown(format!("Error saving new saved item: {}", e)))
}
