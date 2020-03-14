use std::{cmp::Ordering, rc::Rc};

use chrono::NaiveDateTime;
use diesel::{pg::PgConnection, prelude::*};

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
        self.0.last_pocket_sync_time
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
    pub time_added: NaiveDateTime,
}

pub enum SavedItemSort {
    Default,
    TimeAdded,
}

impl Default for SavedItemSort {
    fn default() -> Self {
        Self::Default
    }
}

#[derive(Default)]
pub struct GetSavedItemsQuery {
    pub sort_by: SavedItemSort,
    pub count: Option<i64>,
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
                time_added: Some(&upsert.time_added),
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

    pub fn get_items(&self, _user_id: i32, _query: &GetSavedItemsQuery) -> Result<Vec<SavedItem>> {
        todo!()
    }

    pub fn get_items_by_keyword(&self, user_id: i32, keyword: &str) -> Result<Vec<SavedItem>> {
        // Find most relevant items by tf-idf.
        //
        // tf-idf stands for term frequency-inverse document frequency, which
        // rewards documents that contain more usage of uncommon terms in the
        // search query. https://en.wikipedia.org/wiki/Tf%E2%80%93idf
        //
        // This implementation uses tf(t, d) = count of t in d and idf(t, d, D)
        // = log_10(|D|/|{d in D : t in D}|).

        let user_saved_items = db::get_saved_items_by_user(&self.db_conn, user_id)?;
        let keyword_terms = keyword
            .split_whitespace()
            .map(str::to_lowercase)
            .collect::<Vec<_>>();

        // [[1, 2, 3], [0, 5, 1], ...]
        // For each doc (aka saved item), store the raw count of each word in
        // the doc.
        let mut term_freqs_by_doc = vec![vec![0; keyword_terms.len()]; user_saved_items.len()];
        // For each term, store the number of documents containing the term.
        let mut doc_freqs = vec![0; keyword_terms.len()];

        for (doc_i, saved_item) in user_saved_items.iter().enumerate() {
            // Calculate term-frequency for title.
            for word in saved_item.title.split_whitespace().map(str::to_lowercase) {
                for (term_i, term) in keyword_terms.iter().enumerate() {
                    if *term == word {
                        if term_freqs_by_doc[doc_i][term_i] == 0 {
                            doc_freqs[term_i] += 1;
                        }
                        term_freqs_by_doc[doc_i][term_i] += 1;
                    }
                }
            }

            // Calculate term-frequency for excerpt.
            if let Some(doc_excerpt) = &saved_item.excerpt {
                for word in doc_excerpt.split_whitespace().map(str::to_lowercase) {
                    for (term_i, term) in keyword_terms.iter().enumerate() {
                        if *term == word {
                            if term_freqs_by_doc[doc_i][term_i] == 0 {
                                doc_freqs[term_i] += 1;
                            }
                            term_freqs_by_doc[doc_i][term_i] += 1;
                        }
                    }
                }
            }

            // Calculate term-frequency for URL.
            if let Some(url) = &saved_item.url {
                let lower_url = url.to_lowercase();
                for (term_i, term) in keyword_terms.iter().enumerate() {
                    let count = lower_url.matches(term).count();
                    if count > 0 {
                        if term_freqs_by_doc[doc_i][term_i] == 0 {
                            doc_freqs[term_i] += 1;
                        }
                        term_freqs_by_doc[doc_i][term_i] += count;
                    }
                }
            }
        }

        let mut scores = term_freqs_by_doc
            .iter()
            .enumerate()
            .filter_map(|(doc_i, doc_term_counts)| {
                let score = doc_term_counts
                    .iter()
                    .enumerate()
                    .map(|(term_i, term_frequency)| {
                        *term_frequency as f64
                            * (user_saved_items.len() as f64 / (1.0 + doc_freqs[term_i] as f64))
                                .log10()
                    })
                    .sum::<f64>();

                if score.is_normal() {
                    Some((doc_i, score))
                } else {
                    // NaN, 0, subnormal scores get filtered out
                    None
                }
            })
            .collect::<Vec<_>>();
        scores.sort_unstable_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(Ordering::Equal));
        Ok(scores
            .iter()
            .map(|(i, _)| user_saved_items[*i].clone().into())
            .collect())
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
