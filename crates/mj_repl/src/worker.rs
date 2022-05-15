use std::sync::Arc;

use anyhow::Result;
use tokio::sync::{oneshot, Mutex};

use crate::{
    app::App,
    db::{DbEvent, DbResponse},
};

pub enum IoEvent {
    GetRandomItem,
}

pub struct Worker<'a> {
    pub app: &'a Arc<Mutex<App>>,
    db_tx: tokio::sync::mpsc::Sender<(DbEvent, oneshot::Sender<Result<DbResponse>>)>,
}

impl<'a> Worker<'a> {
    pub fn new(
        app: &'a Arc<Mutex<App>>,
        db_tx: tokio::sync::mpsc::Sender<(DbEvent, oneshot::Sender<Result<DbResponse>>)>,
    ) -> Self {
        Self { app, db_tx }
    }

    pub async fn handle_io_event(&mut self, io_event: IoEvent) {
        match io_event {
            IoEvent::GetRandomItem => {
                let (resp_tx, resp_rx) = oneshot::channel();
                self.db_tx
                    .send((DbEvent::GetRandomItem, resp_tx))
                    .await
                    .ok()
                    .unwrap();
                let res = resp_rx.await.unwrap();
                match res {
                    Ok(DbResponse::GetRandomItem(Ok(item))) => {
                        self.app.lock().await.saved_item = Some(item);
                    }
                    _ => {}
                }
            }
        }
    }
}
