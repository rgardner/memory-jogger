use std::sync::Arc;

use tokio::sync::Mutex;

use crate::app::App;

pub enum IoEvent {}

pub struct Worker<'a> {
    pub app: &'a Arc<Mutex<App>>,
}

impl<'a> Worker<'a> {
    pub fn new(app: &'a Arc<Mutex<App>>) -> Self {
        Self { app }
    }

    pub async fn handle_io_event(&mut self, io_event: IoEvent) {
        match io_event {}
    }
}
