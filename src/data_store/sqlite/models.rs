use chrono::NaiveDateTime;

use crate::pocket::PocketItemId;

use super::schema::{saved_items, users};

#[derive(Queryable)]
pub struct User {
    pub id: i32,
    pub email: String,
    pub pocket_access_token: Option<String>,
    pub last_pocket_sync_time: Option<i64>,
}

#[derive(Insertable)]
#[table_name = "users"]
pub struct NewUser<'a> {
    pub email: &'a str,
    pub pocket_access_token: Option<&'a str>,
}

#[derive(AsChangeset)]
#[table_name = "users"]
pub struct UpdateUser<'a> {
    pub email: Option<&'a str>,
    pub pocket_access_token: Option<&'a str>,
    pub last_pocket_sync_time: Option<i64>,
}

#[derive(Queryable, Clone)]
pub struct SavedItem {
    pub id: i32,
    pub user_id: i32,
    pub pocket_id: PocketItemId,
    pub title: String,
    pub excerpt: Option<String>,
    pub url: Option<String>,
    pub time_added: Option<NaiveDateTime>,
}

#[derive(Insertable, AsChangeset)]
#[table_name = "saved_items"]
pub struct NewSavedItem<'a> {
    pub user_id: i32,
    pub pocket_id: &'a PocketItemId,
    pub title: &'a str,
    pub excerpt: Option<&'a str>,
    pub url: Option<&'a str>,
    pub time_added: Option<&'a NaiveDateTime>,
}
