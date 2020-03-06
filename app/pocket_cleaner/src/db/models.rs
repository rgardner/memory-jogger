use crate::db::schema::saved_items;

#[derive(Queryable)]
pub struct SavedItem {
    pub id: i32,
    pub pocket_id: String,
    pub title: String,
    pub body: String,
}

#[derive(Insertable)]
#[table_name = "saved_items"]
pub struct NewSavedItem<'a> {
    pub pocket_id: &'a str,
    pub title: &'a str,
    pub body: &'a str,
}
