use anyhow::Result;

use memory_jogger::data_store::SavedItem;

#[derive(Debug)]
pub enum DbEvent {
    GetRandomItem,
}

#[derive(Debug)]
pub enum DbResponse {
    GetRandomItem(Result<SavedItem>),
}
