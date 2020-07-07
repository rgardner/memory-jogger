use std::rc::Rc;

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::error::Result;

#[cfg(feature = "postgres")]
mod pg;

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
}

#[derive(Clone, Debug)]
pub struct SavedItem {
    id: i32,
    user_id: i32,
    pocket_id: String,
    title: String,
    // TODO: remove unused body field
    body: Option<String>,
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
    pub fn body(&self) -> Option<String> {
        self.body.clone()
    }
    pub fn excerpt(&self) -> Option<String> {
        self.excerpt.clone()
    }
    pub fn time_added(&self) -> Option<NaiveDateTime> {
        self.time_added
    }
}

pub struct StoreFactory {
    pg_conn: Rc<PgConnection>,
}

impl StoreFactory {
    pub fn new() -> Result<Self> {
        let conn = pg::initialize_db()?;
        Ok(StoreFactory {
            pg_conn: Rc::new(conn),
        })
    }

    pub fn create_user_store(&self) -> Box<dyn UserStore> {
        Box::new(pg::PgUserStore::new(&self.pg_conn))
    }

    pub fn create_saved_item_store(&self) -> Box<dyn SavedItemStore> {
        Box::new(pg::PgSavedItemStore::new(&self.pg_conn))
    }
}
