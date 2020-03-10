#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use crate::{
    data_store::{SavedItemStore, UpsertSavedItem, UserStore},
    error::Result,
    pocket::{PocketPage, UserPocketManager},
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

    pub async fn sync(&mut self, user_id: i32) -> Result<()> {
        let user = self.user_store.get_user(user_id)?;
        let last_sync_time = user.last_pocket_sync_time();

        let mut page = 0;
        let mut offset = 0;
        let new_last_sync_time = loop {
            page += 1;

            let PocketPage { items, since } = self
                .pocket
                .get_items_paginated(ITEMS_PER_PAGE, offset, last_sync_time)
                .await?;
            let store_items: Vec<_> = items
                .into_iter()
                .map(|item| UpsertSavedItem {
                    user_id,
                    pocket_id: item.id(),
                    title: item.title(),
                    excerpt: item.excerpt(),
                    url: item.url(),
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
