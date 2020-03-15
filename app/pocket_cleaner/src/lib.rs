#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use crate::{
    data_store::{SavedItemStore, UpsertSavedItem, UserStore},
    error::Result,
    pocket::{PocketPage, PocketRetrieveQuery, UserPocketManager},
};

pub mod config;
pub mod data_store;
mod db;
pub mod email;
pub mod error;
pub mod pocket;
pub mod trends;
pub mod view;

const ITEMS_PER_PAGE: u32 = 100;

pub struct SavedItemMediator<'a> {
    pocket: &'a UserPocketManager,
    saved_item_store: &'a mut SavedItemStore,
    user_store: &'a mut UserStore,
}

impl<'a> SavedItemMediator<'a> {
    pub fn new(
        pocket: &'a UserPocketManager,
        saved_item_store: &'a mut SavedItemStore,
        user_store: &'a mut UserStore,
    ) -> Self {
        Self {
            pocket,
            saved_item_store,
            user_store,
        }
    }

    /// Syncs any new or updated items from the user's Pocket collection to the
    /// database.
    ///
    /// This performs a delta sync of any items that have been added or changed
    /// since the last sync. If the database schema has not changed, use this
    /// function.
    ///
    /// To perform a full sync of all items in the user's Pocket collection, use
    /// [sync_full](struct.SavedItemMediator.html#method.sync_full).
    pub async fn sync(&mut self, user_id: i32) -> Result<()> {
        let user = self.user_store.get_user(user_id)?;
        let last_sync_time = user.last_pocket_sync_time();
        self.sync_impl(user_id, last_sync_time).await
    }

    /// Re-syncs all items from the user's Pocket collection to the database.
    ///
    /// This requests all items from the user's Pocket collection and potentially
    /// involves significant more network traffic. This should only be used when
    /// new database columns have been added.
    ///
    /// To perform a delta sync of only new or changed items, use
    /// [sync](struct.SavedItemMediator.html#method.sync).
    pub async fn sync_full(&mut self, user_id: i32) -> Result<()> {
        self.sync_impl(user_id, None /*last_sync_time*/).await
    }

    async fn sync_impl(&mut self, user_id: i32, last_sync_time: Option<i64>) -> Result<()> {
        let mut page = 0;
        let mut offset = 0;
        let new_last_sync_time = loop {
            page += 1;

            let PocketPage { items, since } = self
                .pocket
                .retrieve(&PocketRetrieveQuery {
                    count: Some(ITEMS_PER_PAGE),
                    offset: Some(offset),
                    since: last_sync_time,
                    ..Default::default()
                })
                .await?;
            let store_items: Vec<_> = items
                .into_iter()
                .map(|item| UpsertSavedItem {
                    user_id,
                    pocket_id: item.id(),
                    title: item.title(),
                    excerpt: item.excerpt(),
                    url: item.url(),
                    time_added: item.time_added(),
                })
                .collect();
            self.saved_item_store.upsert_items(&store_items)?;
            log::debug!("Synced {} items to DB (page {})", store_items.len(), page);
            let num_stored_items = store_items.len() as u32;
            offset += num_stored_items;
            if num_stored_items < ITEMS_PER_PAGE {
                break since;
            }
        };

        self.user_store
            .update_user_last_pocket_sync_time(user_id, Some(new_last_sync_time))?;

        Ok(())
    }
}
