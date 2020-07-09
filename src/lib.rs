#[macro_use]
extern crate diesel;

use crate::{
    data_store::{SavedItemStore, UpsertSavedItem, UserStore},
    error::Result,
    pocket::{PocketItem, PocketPage, PocketRetrieveItemState, PocketRetrieveQuery, UserPocket},
};

pub mod data_store;
pub mod email;
pub mod error;
mod http;
pub mod pocket;
pub mod trends;

const ITEMS_PER_PAGE: u32 = 100;

pub struct SavedItemMediator<'a> {
    pocket: &'a UserPocket<'a>,
    saved_item_store: &'a mut dyn SavedItemStore,
    user_store: &'a mut dyn UserStore,
}

impl<'a> SavedItemMediator<'a> {
    pub fn new(
        pocket: &'a UserPocket,
        saved_item_store: &'a mut dyn SavedItemStore,
        user_store: &'a mut dyn UserStore,
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
                    state: Some(PocketRetrieveItemState::All),
                    count: Some(ITEMS_PER_PAGE),
                    offset: Some(offset),
                    since: last_sync_time,
                    ..Default::default()
                })
                .await?;

            for item in &items {
                match item {
                    PocketItem::Unread {
                        id,
                        title,
                        excerpt,
                        url,
                        time_added,
                    } => {
                        // Create or update the item
                        self.saved_item_store.upsert_item(&UpsertSavedItem {
                            user_id,
                            pocket_id: id,
                            title,
                            excerpt,
                            url,
                            time_added,
                        })?;
                    }
                    PocketItem::ArchivedOrDeleted { id, .. } => {
                        // Delete the item if it exists
                        self.saved_item_store.delete_item(user_id, &id)?;
                    }
                }
            }

            log::debug!("Synced {} items to DB (page {})", items.len(), page);
            let num_stored_items = items.len() as u32;
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
