//! Create, read, update, and delete operations on users and saved items.
//!
//! Backend and InferConnection code originated from the diesel_cli crate.
//! Dual-licensed under Apache License, Version 2.0 and MIT.
//! https://github.com/diesel-rs/diesel/blob/fa826f0c97e1f47eef34f37cb5b60056855a2b9a/diesel_cli/src/database.rs#L20-L124

use std::rc::Rc;

use anyhow::Result;
use chrono::NaiveDateTime;
use diesel::prelude::*;

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
    fn create_user<'a>(
        &mut self,
        email: &'a str,
        pocket_access_token: Option<&'a str>,
    ) -> Result<User>;

    fn get_user(&self, id: i32) -> Result<User>;

    fn filter_users(&self, count: i32) -> Result<Vec<User>>;

    fn update_user<'a>(
        &mut self,
        id: i32,
        email: Option<&'a str>,
        pocket_access_token: Option<&'a str>,
    ) -> Result<()>;

    fn update_user_last_pocket_sync_time(&mut self, id: i32, value: Option<i64>) -> Result<()>;

    fn delete_user(&mut self, id: i32) -> Result<()>;

    fn delete_all_users(&mut self) -> Result<()>;
}

#[derive(Clone, Debug)]
pub struct SavedItem {
    id: i32,
    user_id: i32,
    pocket_id: String,
    title: String,
    excerpt: Option<String>,
    url: Option<String>,
    time_added: Option<NaiveDateTime>,
}

pub struct UpsertSavedItem<'a> {
    pub user_id: i32,
    pub pocket_id: &'a str,
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
        pocket_id: &'a str,
        title: &'a str,
    ) -> Result<SavedItem>;

    /// Creates or updates the saved item in the database.
    fn upsert_item(&mut self, item: &UpsertSavedItem) -> Result<()>;

    /// Retrieves a single item.
    fn get_item(&self, id: i32) -> Result<Option<SavedItem>>;

    fn get_items(&self, query: &GetSavedItemsQuery) -> Result<Vec<SavedItem>>;

    fn get_items_by_keyword(&self, user_id: i32, keyword: &str) -> Result<Vec<SavedItem>>;

    /// Deletes the saved item from the database if the saved item exists.
    fn delete_item(&mut self, user_id: i32, pocket_id: &str) -> Result<()>;

    /// Deletes all saved items from the database for the given user.
    fn delete_all(&mut self, user_id: i32) -> Result<()>;
}

impl User {
    pub fn id(&self) -> i32 {
        self.id
    }
    pub fn email(&self) -> String {
        self.email.clone()
    }
    pub fn pocket_access_token(&self) -> Option<String> {
        self.pocket_access_token.clone()
    }
    pub fn last_pocket_sync_time(&self) -> Option<i64> {
        self.last_pocket_sync_time
    }
}

impl SavedItem {
    pub fn id(&self) -> i32 {
        self.id
    }
    pub fn user_id(&self) -> i32 {
        self.user_id
    }
    pub fn pocket_id(&self) -> String {
        self.pocket_id.clone()
    }
    pub fn title(&self) -> String {
        self.title.clone()
    }
    pub fn excerpt(&self) -> Option<String> {
        self.excerpt.clone()
    }
    pub fn time_added(&self) -> Option<NaiveDateTime> {
        self.time_added
    }
}

pub struct StoreFactory {
    db_conn: InferConnection,
}

impl StoreFactory {
    pub fn new(database_url: &str) -> Result<Self> {
        let db_conn = match Backend::for_url(database_url) {
            #[cfg(feature = "postgres")]
            Backend::Pg => {
                pg::initialize_db(database_url).map(|conn| InferConnection::Pg(Rc::new(conn)))?
            }
            #[cfg(feature = "sqlite")]
            Backend::Sqlite => sqlite::initialize_db(database_url)
                .map(|conn| InferConnection::Sqlite(Rc::new(conn)))?,
        };

        Ok(StoreFactory { db_conn })
    }

    pub fn create_user_store(&self) -> Box<dyn UserStore> {
        match &self.db_conn {
            #[cfg(feature = "postgres")]
            InferConnection::Pg(conn) => Box::new(pg::PgUserStore::new(&conn)),
            #[cfg(feature = "sqlite")]
            InferConnection::Sqlite(conn) => Box::new(sqlite::SqliteUserStore::new(&conn)),
        }
    }

    pub fn create_saved_item_store(&self) -> Box<dyn SavedItemStore> {
        match &self.db_conn {
            #[cfg(feature = "postgres")]
            InferConnection::Pg(conn) => Box::new(pg::PgSavedItemStore::new(&conn)),
            #[cfg(feature = "sqlite")]
            InferConnection::Sqlite(conn) => Box::new(sqlite::SqliteSavedItemStore::new(&conn)),
        }
    }
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
                    Backend::Pg
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
            _ => Backend::Sqlite,
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
