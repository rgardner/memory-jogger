use anyhow::{anyhow, Result};
use data_store::DataStore;
#[macro_use]
extern crate diesel;

use crate::{
    data_store::UpsertSavedItem,
    pocket::{PocketItem, PocketPage, PocketRetrieveItemState, PocketRetrieveQuery, UserPocket},
};

pub mod data_store;
pub mod email;
mod http;
pub mod pocket;
pub mod trends;

const ITEMS_PER_PAGE: u32 = 100;

pub struct SavedItemMediator<'a> {
    pocket: &'a UserPocket<'a>,
    data_store: &'a mut dyn DataStore,
}

impl<'a> SavedItemMediator<'a> {
    pub fn new(pocket: &'a UserPocket, data_store: &'a mut dyn DataStore) -> Self {
        Self { pocket, data_store }
    }

    #[must_use]
    pub fn data_store_mut(&mut self) -> &mut dyn DataStore {
        self.data_store
    }

    /// Syncs any new or updated items from the user's Pocket collection to the
    /// database.
    ///
    /// This performs a delta sync of any items that have been added or changed
    /// since the last sync. If the database schema has not changed, use this
    /// function.
    ///
    /// To perform a full sync of all items in the user's Pocket collection, use
    /// [`Self::sync_full`].
    ///
    /// # Errors
    ///
    /// Fails if the user's Pocket access token is not set or has expired, or
    /// if a network error occurs.
    pub async fn sync(&mut self, user_id: i32) -> Result<()> {
        let user = self.data_store.get_user(user_id)?;
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
    /// [`Self::sync`].
    ///
    /// # Errors
    ///
    /// Fails if the user's Pocket access token is not set or has expired, or
    /// if a network error occurs.
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
                    ..PocketRetrieveQuery::default()
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
                        self.data_store.upsert_item(&UpsertSavedItem {
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
                        self.data_store.delete_item(user_id, id)?;
                    }
                }
            }

            log::debug!("Synced {} items to DB (page {})", items.len(), page);
            let num_stored_items: u32 = items
                .len()
                .try_into()
                .expect("more than 2^32 items returned");
            offset += num_stored_items;
            if num_stored_items < ITEMS_PER_PAGE {
                break since;
            }
        };

        self.data_store
            .update_user_last_pocket_sync_time(user_id, Some(new_last_sync_time))?;

        Ok(())
    }

    /// Marks item as read, updating database and Pocket.
    ///
    /// # Errors
    ///
    /// Fails if the Pocket API returns an error.
    pub async fn archive(&mut self, user_id: i32, item_id: i32) -> Result<()> {
        let item = self
            .data_store
            .get_item(item_id)?
            .ok_or_else(|| anyhow!("item {} does not exist", item_id))?;
        self.pocket.archive(item.pocket_id()).await?;
        self.sync(user_id).await?;
        Ok(())
    }

    /// Deletes item, updating database and Pocket.
    ///
    /// # Errors
    ///
    /// Fails if the Pocket API returns an error.
    pub async fn delete(&mut self, user_id: i32, item_id: i32) -> Result<()> {
        let item = self
            .data_store
            .get_item(item_id)?
            .ok_or_else(|| anyhow!("item {} does not exist", item_id))?;
        self.pocket.delete(item.pocket_id()).await?;
        self.sync(user_id).await?;
        Ok(())
    }

    /// Favorites item, updating database and Pocket.
    ///
    /// # Errors
    ///
    /// Fails if the Pocket API returns an error.
    pub async fn favorite(&mut self, item_id: i32) -> Result<()> {
        let item = self
            .data_store
            .get_item(item_id)?
            .ok_or_else(|| anyhow!("item {} does not exist", item_id))?;
        self.pocket.favorite(item.pocket_id()).await?;
        Ok(())
    }
}
