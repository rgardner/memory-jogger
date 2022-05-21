//! Create, read, update, and delete operations on users and saved items.
//!
//! Backend and [`InferConnection`] code originated from the
//! [`diesel_cli`](https://github.com/diesel-rs/diesel/tree/master/diesel_cli)
//! crate. [Dual-licensed under Apache License, Version 2.0 and
//! MIT](https://github.com/diesel-rs/diesel/blob/fa826f0c97e1f47eef34f37cb5b60056855a2b9a/diesel_cli/src/database.rs#L20-L124).

use std::rc::Rc;

use anyhow::Result;
use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::pocket::PocketItemId;

#[cfg(feature = "postgres")]
mod pg;
#[cfg(feature = "sqlite")]
mod sqlite;

pub struct User {
    id: i32,
    email: String,
    pocket_access_token: Option<String>,
    last_pocket_sync_time: Option<i64>,
}

pub trait UserStore {
    /// Create a new user.
    ///
    /// # Errors
    ///
    /// Fails if a user with the given `email` already exists or the connection
    /// to the database fails.
    fn create_user<'a>(
        &mut self,
        email: &'a str,
        pocket_access_token: Option<&'a str>,
    ) -> Result<User>;

    /// Gets a user by their ID.
    ///
    /// # Errors
    ///
    /// Fails if the user does not exist or the connection to the database fails.
    fn get_user(&mut self, id: i32) -> Result<User>;

    /// Returns `count` number users.
    ///
    /// # Errors
    ///
    /// Fails if the connection to the database fails.
    fn filter_users(&mut self, count: i32) -> Result<Vec<User>>;

    /// Updates the `email` and or `pocket_access_token` of a user.
    ///
    /// # Errors
    ///
    /// Fails if the user does not exist or the connection to the database fails.
    fn update_user<'a>(
        &mut self,
        id: i32,
        email: Option<&'a str>,
        pocket_access_token: Option<&'a str>,
    ) -> Result<()>;

    /// Updates the `last_pocket_sync_time` of a user returned by the Pocket
    /// API.
    ///
    /// # Errors
    ///
    /// Fails if the user does not exist or the connection to the database fails.
    fn update_user_last_pocket_sync_time(&mut self, id: i32, value: Option<i64>) -> Result<()>;

    /// Deletes a user.
    ///
    /// # Errors
    ///
    /// Fails if the user does not exist or the connection to the database fails.
    fn delete_user(&mut self, id: i32) -> Result<()>;

    /// Deletes all users.
    ///
    /// # Errors
    ///
    /// Fails if the connection to the database fails.
    fn delete_all_users(&mut self) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct SavedItem {
    id: i32,
    user_id: i32,
    pocket_id: PocketItemId,
    title: String,
    excerpt: Option<String>,
    url: Option<String>,
    time_added: Option<NaiveDateTime>,
}

pub struct UpsertSavedItem<'a> {
    pub user_id: i32,
    pub pocket_id: &'a PocketItemId,
    pub title: &'a str,
    pub excerpt: &'a str,
    pub url: &'a str,
    pub time_added: &'a NaiveDateTime,
}

pub enum SavedItemSort {
    TimeAdded,
}

#[derive(Default)]
pub struct GetSavedItemsQuery {
    pub user_id: i32,
    pub sort_by: Option<SavedItemSort>,
    pub count: Option<i64>,
}

pub trait SavedItemStore {
    fn create_saved_item<'a>(
        &mut self,
        user_id: i32,
        pocket_id: &'a PocketItemId,
        title: &'a str,
    ) -> Result<SavedItem>;

    /// Creates or updates the saved item in the database.
    fn upsert_item(&mut self, item: &UpsertSavedItem) -> Result<()>;

    /// Retrieves a single item.
    fn get_item(&mut self, id: i32) -> Result<Option<SavedItem>>;

    fn get_items(&mut self, query: &GetSavedItemsQuery) -> Result<Vec<SavedItem>>;

    fn get_items_by_keyword(&mut self, user_id: i32, keyword: &str) -> Result<Vec<SavedItem>>;

    fn get_random_item(&mut self, user_id: i32) -> Result<Option<SavedItem>>;

    /// Deletes the saved item from the database if the saved item exists.
    fn delete_item(&mut self, user_id: i32, pocket_id: &PocketItemId) -> Result<()>;

    /// Deletes all saved items from the database for the given user.
    fn delete_all(&mut self, user_id: i32) -> Result<()>;
}

impl User {
    #[must_use]
    pub fn id(&self) -> i32 {
        self.id
    }
    #[must_use]
    pub fn email(&self) -> String {
        self.email.clone()
    }
    #[must_use]
    pub fn pocket_access_token(&self) -> Option<String> {
        self.pocket_access_token.clone()
    }
    #[must_use]
    pub fn last_pocket_sync_time(&self) -> Option<i64> {
        self.last_pocket_sync_time
    }
}

impl SavedItem {
    #[must_use]
    pub const fn id(&self) -> i32 {
        self.id
    }
    #[must_use]
    pub const fn user_id(&self) -> i32 {
        self.user_id
    }
    #[must_use]
    pub fn pocket_id(&self) -> PocketItemId {
        self.pocket_id.clone()
    }
    #[must_use]
    pub fn title(&self) -> String {
        self.title.clone()
    }
    #[must_use]
    pub fn excerpt(&self) -> Option<String> {
        self.excerpt.clone()
    }
    #[must_use]
    pub fn url(&self) -> Option<String> {
        self.url.clone()
    }
    #[must_use]
    pub const fn time_added(&self) -> Option<NaiveDateTime> {
        self.time_added
    }
}

pub trait DataStore: UserStore + SavedItemStore {}

pub fn create_store(database_url: &str) -> Result<Box<dyn DataStore>> {
    let store: Box<dyn DataStore> = match Backend::for_url(database_url) {
        #[cfg(feature = "postgres")]
        Backend::Pg => Box::new(pg::DataStore::new(pg::initialize_db(database_url)?)),
        #[cfg(feature = "sqlite")]
        Backend::Sqlite => Box::new(sqlite::DataStore::new(sqlite::initialize_db(database_url)?)),
    };
    Ok(store)
}

enum Backend {
    #[cfg(feature = "postgres")]
    Pg,
    #[cfg(feature = "sqlite")]
    Sqlite,
}

impl Backend {
    fn for_url(database_url: &str) -> Self {
        match database_url {
            _ if database_url.starts_with("postgres://")
                || database_url.starts_with("postgresql://") =>
            {
                #[cfg(feature = "postgres")]
                {
                    Self::Pg
                }
                #[cfg(not(feature = "postgres"))]
                {
                    panic!(
                        "Database url `{}` requires the `postgres` feature but it's not enabled.",
                        database_url
                    );
                }
            }
            #[cfg(feature = "sqlite")]
            _ => Self::Sqlite,
            #[cfg(not(feature = "sqlite"))]
            _ => {
                if database_url.starts_with("sqlite://") {
                    panic!(
                        "Database url `{}` requires the `sqlite` feature but it's not enabled.",
                        database_url
                    );
                }

                panic!(
                    "`{}` is not a valid database URL. It should start with postgres, or maybe you meant to use the `sqlite` feature which is not enabled.",
                    database_url,
                );
            }
            #[cfg(not(any(feature = "sqlite", feature = "postgres")))]
            _ => compile_error!(
                "At least one backend must be specified for use with this crate. \
                 You may omit the unneeded dependencies in the following command. \n\n \
                 ex. `cargo install memory_jogger --no-default-features --features postgres sqlite` \n"
            ),
        }
    }
}

pub enum InferConnection {
    #[cfg(feature = "postgres")]
    Pg(Rc<PgConnection>),
    #[cfg(feature = "sqlite")]
    Sqlite(Rc<SqliteConnection>),
}
