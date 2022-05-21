use std::sync::Arc;

use chrono::NaiveDateTime;
use memory_jogger::{data_store::SavedItem, SavedItemMediator};
use reqwest::Url;
use tokio::sync::Mutex;

use crate::{
    app::{App, Message},
    util,
};

pub enum IoEvent {
    GetRandomItem,
    ArchiveItem(SavedItem),
    DeleteItem(SavedItem),
    FavoriteItem(SavedItem),
    GetHnDiscussions(Url),
    ResolveUrl(Url),
    GetWaybackUrl(String, Option<NaiveDateTime>),
    GetWaybackPromptUrl(String, Option<NaiveDateTime>),
}

pub struct Worker<'a> {
    pub app: &'a Arc<Mutex<App>>,
    saved_item_mediator: SavedItemMediator<'a>,
    http_client: &'a reqwest::Client,
}

impl<'a> Worker<'a> {
    #[must_use]
    pub fn new(
        app: &'a Arc<Mutex<App>>,
        saved_item_mediator: SavedItemMediator<'a>,
        http_client: &'a reqwest::Client,
    ) -> Self {
        Self {
            app,
            saved_item_mediator,
            http_client,
        }
    }

    pub async fn handle_io_event(&mut self, io_event: IoEvent) {
        match io_event {
            IoEvent::GetRandomItem => {
                let user_id = self.app.lock().await.user_id;
                let item = self
                    .saved_item_mediator
                    .saved_item_store()
                    .get_random_item(user_id);
                let mut app = self.app.lock().await;
                let item = match item {
                    Ok(item) => item,
                    Err(e) => {
                        app.saved_item = None;
                        app.message = Message::Error(format!("Failed to get items: {}", e)).into();
                        return;
                    }
                };

                app.reset_state();
                app.saved_item = item.clone();
                if let Some(item) = item {
                    if let Some(url) = item.url() {
                        if let Ok(parsed_url) = Url::parse(&url) {
                            app.dispatch(IoEvent::ResolveUrl(parsed_url.clone()));
                            app.dispatch(IoEvent::GetHnDiscussions(parsed_url));
                        }
                        app.dispatch(IoEvent::GetWaybackUrl(url, item.time_added()));
                    }
                }
            }
            IoEvent::ArchiveItem(item) => {
                let res = self
                    .saved_item_mediator
                    .archive(item.user_id(), item.id())
                    .await;
                let msg = match res {
                    Ok(()) => Message::Info("Item archived".into()).into(),
                    Err(e) => Message::Error(format!("Error archiving item: {}", e)).into(),
                };
                self.app.lock().await.message = msg;
            }
            IoEvent::DeleteItem(item) => {
                let res = self
                    .saved_item_mediator
                    .delete(item.user_id(), item.id())
                    .await;
                let msg = match res {
                    Ok(()) => Message::Info("Item deleted".into()).into(),
                    Err(e) => Message::Error(format!("Error deleting item: {}", e)).into(),
                };
                self.app.lock().await.message = msg;
            }
            IoEvent::FavoriteItem(item) => {
                let res = self.saved_item_mediator.favorite(item.id()).await;
                let msg = match res {
                    Ok(()) => Message::Info("Item favorited".into()).into(),
                    Err(e) => Message::Error(format!("Error favoriting item: {}", e)).into(),
                };
                self.app.lock().await.message = msg;
            }
            IoEvent::GetHnDiscussions(url) => {
                let discussions = util::get_hn_discussions(url, self.http_client).await;
                if let Ok(discussions) = discussions {
                    self.app.lock().await.discussions = discussions;
                }
            }
            IoEvent::ResolveUrl(url) => {
                let res = util::resolve_submission_url(url, self.http_client).await;
                let mut app = self.app.lock().await;
                match res {
                    Ok(url) => app.resolved_url = url,
                    Err(e) => {
                        app.message =
                            Message::Error(format!("Error getting submission url: {}", e)).into();
                    }
                }
            }
            IoEvent::GetWaybackUrl(url, time) => {
                let res = util::get_wayback_url(url, time, self.http_client).await;
                let mut app = self.app.lock().await;
                match res {
                    Ok(url) => app.wayback_url = url,
                    Err(e) => {
                        app.message =
                            Message::Error(format!("Error getting wayback url: {}", e)).into();
                    }
                }
            }
            IoEvent::GetWaybackPromptUrl(url, time) => {
                let res = util::get_wayback_url(url, time, self.http_client).await;
                let mut app = self.app.lock().await;
                match res {
                    Ok(url) => app.wayback_prompt_url = url,
                    Err(e) => {
                        app.message =
                            Message::Error(format!("Error getting wayback url: {}", e)).into();
                    }
                }
            }
        }
    }
}
