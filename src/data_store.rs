use std::{cmp::Ordering, rc::Rc};

use chrono::NaiveDateTime;
use diesel::prelude::*;

use crate::error::{PocketCleanerError, Result};

mod pg;

pub struct User(pg::models::User);

pub struct UserStore {
    pg_conn: Rc<PgConnection>,
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

impl From<pg::models::User> for User {
    fn from(model: pg::models::User) -> Self {
        Self(model)
    }
}

impl UserStore {
    fn new(conn: &Rc<PgConnection>) -> Self {
        UserStore {
            pg_conn: Rc::clone(conn),
        }
    }

    pub fn create_user<'a>(
        &mut self,
        email: &'a str,
        pocket_access_token: Option<&'a str>,
    ) -> Result<User> {
        pg::create_user(&self.pg_conn, &email, pocket_access_token.as_deref()).map(|u| u.into())
    }

    pub fn get_user(&self, id: i32) -> Result<User> {
        pg::get_user(&self.pg_conn, id).map(|u| u.into())
    }

    pub fn filter_users(&self, count: i32) -> Result<Vec<User>> {
        use pg::schema::users::dsl::users;
        Ok(users
            .limit(count.into())
            .load::<pg::models::User>(&*self.pg_conn)
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
        pg::update_user(&self.pg_conn, id, email, pocket_access_token, None)
    }

    pub fn update_user_last_pocket_sync_time(&mut self, id: i32, value: Option<i64>) -> Result<()> {
        pg::update_user(&self.pg_conn, id, None, None, value)
    }
}

pub struct SavedItem(pg::models::SavedItem);

pub struct SavedItemStore {
    pg_conn: Rc<PgConnection>,
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
    pub fn time_added(&self) -> Option<NaiveDateTime> {
        self.0.time_added
    }
}

impl From<pg::models::SavedItem> for SavedItem {
    fn from(model: pg::models::SavedItem) -> Self {
        SavedItem(model)
    }
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

impl SavedItemStore {
    pub fn new(conn: &Rc<PgConnection>) -> Self {
        Self {
            pg_conn: Rc::clone(conn),
        }
    }

    pub fn create_saved_item<'a>(
        &mut self,
        user_id: i32,
        pocket_id: &'a str,
        title: &'a str,
    ) -> Result<SavedItem> {
        pg::create_saved_item(&self.pg_conn, user_id, pocket_id, title).map(|item| item.into())
    }

    /// Creates or updates the saved item in the database.
    pub fn upsert_item(&mut self, item: &UpsertSavedItem) -> Result<()> {
        use pg::schema::saved_items::dsl;

        let pg_upsert = pg::models::NewSavedItem {
            user_id: item.user_id,
            pocket_id: &item.pocket_id,
            title: &item.title,
            body: None,
            excerpt: Some(&item.excerpt),
            url: Some(&item.url),
            time_added: Some(&item.time_added),
        };

        diesel::insert_into(dsl::saved_items)
            .values(&pg_upsert)
            .on_conflict(dsl::pocket_id)
            .do_update()
            .set(&pg_upsert)
            .execute(&*self.pg_conn)
            .map(|_| ())
            .map_err(|e| {
                PocketCleanerError::Unknown(format!("Failed to upsert saved item in DB: {}", e))
            })?;

        Ok(())
    }

    pub fn get_items(&self, query: &GetSavedItemsQuery) -> Result<Vec<SavedItem>> {
        use pg::schema::saved_items::dsl;

        let pg_query = dsl::saved_items.filter(dsl::user_id.eq(query.user_id));
        let pg_query = if let Some(count) = query.count {
            pg_query.limit(count).into_boxed()
        } else {
            pg_query.into_boxed()
        };
        let pg_query = match query.sort_by {
            Some(SavedItemSort::TimeAdded) => pg_query.order(dsl::time_added),
            None => pg_query,
        };
        Ok(pg_query
            .load::<pg::models::SavedItem>(&*self.pg_conn)
            .map_err(|e| {
                PocketCleanerError::Unknown(format!("Failed to get saved items from DB: {}", e))
            })?
            .into_iter()
            .map(|u| u.into())
            .collect())
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

        let user_saved_items = pg::get_saved_items_by_user(&self.pg_conn, user_id)?;
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
        scores.sort_unstable_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(Ordering::Equal));
        Ok(scores
            .iter()
            .map(|(i, _)| user_saved_items[*i].clone().into())
            .collect())
    }

    /// Deletes the saved item from the database if the saved item exists.
    pub fn delete_item(&mut self, user_id: i32, pocket_id: &str) -> Result<()> {
        use pg::schema::saved_items::dsl;

        diesel::delete(
            dsl::saved_items
                .filter(dsl::user_id.eq(user_id))
                .filter(dsl::pocket_id.eq(pocket_id)),
        )
        .execute(&*self.pg_conn)
        .map(|_| ())
        .map_err(|e| {
            PocketCleanerError::Unknown(format!("Failed to delete saved item in DB: {}", e))
        })
    }

    /// Deletes all saved items from the database for the given user.
    pub fn delete_all(&mut self, user_id: i32) -> Result<()> {
        use pg::schema::saved_items::dsl;

        diesel::delete(dsl::saved_items.filter(dsl::user_id.eq(user_id)))
            .execute(&*self.pg_conn)
            .map(|_| ())
            .map_err(|e| {
                PocketCleanerError::Unknown(format!("Failed to delete saved item in DB: {}", e))
            })
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

    pub fn create_user_store(&self) -> UserStore {
        UserStore::new(&self.pg_conn)
    }

    pub fn create_saved_item_store(&self) -> SavedItemStore {
        SavedItemStore::new(&self.pg_conn)
    }
}
