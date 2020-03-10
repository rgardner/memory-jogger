use std::rc::Rc;

use diesel::pg::PgConnection;
use diesel::prelude::*;

use crate::{
    db,
    error::{PocketCleanerError, Result},
};

pub struct User(db::models::User);

pub struct UserStore {
    db_conn: Rc<PgConnection>,
}

impl User {
    pub fn id(&self) -> i32 {
        self.0.id
    }
    pub fn email(&self) -> String {
        self.0.email.clone()
    }
    pub fn pocket_access_token(&self) -> Option<String> {
        self.0.pocket_access_token.clone()
    }
    pub fn last_pocket_sync_time(&self) -> Option<i64> {
        self.0.last_pocket_sync_time.clone()
    }
}

impl From<db::models::User> for User {
    fn from(model: db::models::User) -> Self {
        Self(model)
    }
}

impl UserStore {
    fn new(conn: &Rc<PgConnection>) -> Self {
        UserStore {
            db_conn: Rc::clone(conn),
        }
    }

    pub fn create_user<'a>(
        &mut self,
        email: &'a str,
        pocket_access_token: Option<&'a str>,
    ) -> Result<User> {
        db::create_user(&self.db_conn, &email, pocket_access_token.as_deref()).map(|u| u.into())
    }

    pub fn get_user(&self, id: i32) -> Result<User> {
        db::get_user(&self.db_conn, id).map(|u| u.into())
    }

    pub fn filter_users(&self, count: i32) -> Result<Vec<User>> {
        use db::schema::users::dsl::users;
        Ok(users
            .limit(count.into())
            .load::<db::models::User>(&*self.db_conn)
            .map_err(|e| PocketCleanerError::Unknown(format!("Failed to users from DB: {}", e)))?
            .into_iter()
            .map(|u| u.into())
            .collect())
    }

    pub fn update_user<'a>(
        &mut self,
        id: i32,
        email: Option<&'a str>,
        pocket_access_token: Option<&'a str>,
    ) -> Result<()> {
        db::update_user(&self.db_conn, id, email, pocket_access_token, None)
    }

    pub fn update_user_last_pocket_sync_time(&mut self, id: i32, value: Option<i64>) -> Result<()> {
        db::update_user(&self.db_conn, id, None, None, value)
    }
}

pub struct SavedItem(db::models::SavedItem);

pub struct SavedItemStore {
    db_conn: Rc<PgConnection>,
}

impl SavedItem {
    pub fn id(&self) -> i32 {
        self.0.id
    }
    pub fn user_id(&self) -> i32 {
        self.0.user_id
    }
    pub fn pocket_id(&self) -> String {
        self.0.pocket_id.clone()
    }
    pub fn title(&self) -> String {
        self.0.title.clone()
    }
    pub fn body(&self) -> Option<String> {
        self.0.body.clone()
    }
    pub fn excerpt(&self) -> Option<String> {
        self.0.excerpt.clone()
    }
}

impl From<db::models::SavedItem> for SavedItem {
    fn from(model: db::models::SavedItem) -> Self {
        SavedItem(model)
    }
}

pub struct UpsertSavedItem {
    pub user_id: i32,
    pub pocket_id: String,
    pub title: String,
    pub excerpt: String,
    pub url: String,
}

impl SavedItemStore {
    pub fn new(conn: &Rc<PgConnection>) -> Self {
        Self {
            db_conn: Rc::clone(conn),
        }
    }

    pub fn create_saved_item<'a>(
        &mut self,
        user_id: i32,
        pocket_id: &'a str,
        title: &'a str,
    ) -> Result<SavedItem> {
        db::create_saved_item(&self.db_conn, user_id, pocket_id, title).map(|item| item.into())
    }

    pub fn upsert_items(&mut self, items: &[UpsertSavedItem]) -> Result<()> {
        use db::schema::saved_items::dsl::*;
        let db_upserts = items
            .iter()
            .map(|upsert| db::models::NewSavedItem {
                user_id: upsert.user_id,
                pocket_id: &upsert.pocket_id,
                title: &upsert.title,
                body: None,
                excerpt: Some(&upsert.excerpt),
                url: Some(&upsert.url),
            })
            .collect::<Vec<_>>();

        for upsert in &db_upserts {
            diesel::insert_into(saved_items)
                .values(upsert)
                .on_conflict(pocket_id)
                .do_update()
                .set(upsert)
                .execute(&*self.db_conn)
                .map(|_| ())
                .map_err(|e| {
                    PocketCleanerError::Unknown(format!(
                        "Failed to upsert saved items in DB: {}",
                        e
                    ))
                })?;
        }

        Ok(())
    }

    pub fn filter_saved_items(&self, count: i32) -> Result<Vec<SavedItem>> {
        use db::schema::saved_items::dsl::saved_items;
        Ok(saved_items
            .limit(count.into())
            .load::<db::models::SavedItem>(&*self.db_conn)
            .map_err(|e| {
                PocketCleanerError::Unknown(format!("Failed to get saved items from DB: {}", e))
            })?
            .into_iter()
            .map(|u| u.into())
            .collect())
    }
}

pub struct StoreFactory {
    db_conn: Rc<PgConnection>,
}

impl StoreFactory {
    pub fn new() -> Result<Self> {
        let conn = db::initialize_db()?;
        Ok(StoreFactory {
            db_conn: Rc::new(conn),
        })
    }

    pub fn create_user_store(&self) -> UserStore {
        UserStore::new(&self.db_conn)
    }

    pub fn create_saved_item_store(&self) -> SavedItemStore {
        SavedItemStore::new(&self.db_conn)
    }
}
