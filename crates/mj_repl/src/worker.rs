use std::sync::Arc;

use memory_jogger::{data_store::SavedItem, SavedItemMediator};
use tokio::sync::Mutex;

use crate::app::App;

pub enum IoEvent {
    GetRandomItem,
    ArchiveItem(SavedItem),
}

pub struct Worker<'a> {
    pub app: &'a Arc<Mutex<App>>,
    saved_item_mediator: SavedItemMediator<'a>,
}

impl<'a> Worker<'a> {
    pub fn new(app: &'a Arc<Mutex<App>>, saved_item_mediator: SavedItemMediator<'a>) -> Self {
        Self {
            app,
            saved_item_mediator,
        }
    }

    pub async fn handle_io_event(&mut self, io_event: IoEvent) {
        match io_event {
            IoEvent::GetRandomItem => {
                // TODO: re-architect because this is a blocking call
                let item = self
                    .saved_item_mediator
                    .saved_item_store()
                    .get_random_item(1)
                    .unwrap();
                self.app.lock().await.saved_item = item;
            }
            IoEvent::ArchiveItem(item) => {
                self.saved_item_mediator
                    .archive(item.user_id(), item.id())
                    .await
                    .unwrap();
                // TODO: show success message
            }
        }
    }
}
