use std::sync::Arc;

use memory_jogger::{data_store::SavedItem, SavedItemMediator};
use reqwest::Url;
use tokio::sync::Mutex;

use crate::{app::App, util};

pub enum IoEvent {
    GetRandomItem,
    ArchiveItem(SavedItem),
    DeleteItem(SavedItem),
    FavoriteItem(SavedItem),
    GetHnDiscussions(String),
    ResolveUrl(String),
    GetWaybackUrl(String),
}

pub struct Worker<'a> {
    pub app: &'a Arc<Mutex<App>>,
    saved_item_mediator: SavedItemMediator<'a>,
    http_client: &'a reqwest::Client,
}

impl<'a> Worker<'a> {
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
                // TODO: re-architect because this is a blocking call
                let item = self
                    .saved_item_mediator
                    .saved_item_store()
                    .get_random_item(1)
                    .unwrap();
                let mut app = self.app.lock().await;
                app.saved_item = item;
                // TODO: is the clone necessary?
                if let Some(url) = &app.saved_item.clone().and_then(|item| item.url()) {
                    app.dispatch(IoEvent::GetHnDiscussions(url.clone()));
                    app.dispatch(IoEvent::ResolveUrl(url.clone()));
                    app.dispatch(IoEvent::GetWaybackUrl(url.clone()));
                }
            }
            IoEvent::ArchiveItem(item) => {
                self.saved_item_mediator
                    .archive(item.user_id(), item.id())
                    .await
                    .unwrap();
                // TODO: show success message
            }
            IoEvent::DeleteItem(item) => {
                self.saved_item_mediator
                    .delete(item.user_id(), item.id())
                    .await
                    .unwrap();
                // TODO: show success message
            }
            IoEvent::FavoriteItem(item) => {
                self.saved_item_mediator.favorite(item.id()).await.unwrap();
                // TODO: show success message
            }
            IoEvent::GetHnDiscussions(url) => {
                if let Ok(url) = Url::parse(&url) {
                    let discussions = util::get_hn_discussions(url, self.http_client).await;
                    if let Ok(discussions) = discussions {
                        self.app.lock().await.discussions = discussions;
                    }
                }
            }
            IoEvent::ResolveUrl(url) => {
                if let Ok(url) = Url::parse(&url) {
                    let resolved_url = util::resolve_submission_url(url, self.http_client)
                        .await
                        .unwrap();
                    self.app.lock().await.resolved_url = resolved_url;
                }
            }
            IoEvent::GetWaybackUrl(url) => {
                let wayback_url = util::get_wayback_url(url, self.http_client).await.unwrap();
                self.app.lock().await.wayback_url = wayback_url;
            }
        }
    }
}
