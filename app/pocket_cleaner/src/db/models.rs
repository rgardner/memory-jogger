use crate::db::schema::{saved_items, users};

#[derive(Queryable)]
pub(crate) struct User {
    pub id: i32,
    pub email: String,
    pub pocket_access_token: Option<String>,
}

#[derive(Insertable)]
#[table_name = "users"]
pub(crate) struct NewUser<'a> {
    pub email: &'a str,
    pub pocket_access_token: Option<&'a str>,
}

#[derive(AsChangeset)]
#[table_name = "users"]
pub(crate) struct UpdateUser<'a> {
    pub email: Option<&'a str>,
    pub pocket_access_token: Option<&'a str>,
}

#[derive(Queryable)]
pub(crate) struct SavedItem {
    pub id: i32,
    pub user_id: i32,
    pub pocket_id: String,
    pub title: String,
    pub body: String,
}

#[derive(Insertable)]
#[table_name = "saved_items"]
pub(crate) struct NewSavedItem<'a> {
    pub user_id: i32,
    pub pocket_id: &'a str,
    pub title: &'a str,
    pub body: &'a str,
}
