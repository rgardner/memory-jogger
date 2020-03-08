#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_migrations;

use crate::{data_store::SavedItemStore, error::Result, pocket::UserPocketManager};

pub mod config;
pub mod data_store;
mod db;
pub mod email;
pub mod error;
pub mod pocket;
pub mod trends;
pub mod view;

pub struct SavedItemMediator<'a> {
    pocket: &'a UserPocketManager,
    store: &'a SavedItemStore,
}

impl<'a> SavedItemMediator<'a> {
    pub fn new(pocket: &'a UserPocketManager, store: &'a SavedItemStore) -> Self {
        Self { pocket, store }
    }

    pub fn sync(&mut self, user_id: i32) -> Result<()> {
        todo!()
    }
}
