use std::sync::mpsc::Sender;

use memory_jogger::data_store::SavedItem;

use crate::worker::IoEvent;

#[derive(Default)]
pub struct App {
    pub io_tx: Option<Sender<IoEvent>>,
    pub input: String,
    pub error: String,
    pub saved_item: Option<SavedItem>,
    pub discussions: Vec<String>,
}

impl App {
    pub fn new(io_tx: Sender<IoEvent>) -> Self {
        Self {
            io_tx: Some(io_tx),
            ..Default::default()
        }
    }

    pub fn dispatch(&mut self, action: IoEvent) {
        if let Some(io_tx) = &self.io_tx {
            if let Err(e) = io_tx.send(action) {
                eprintln!("Error from dispatch {}", e);
                // TODO: handle error
            };
        }
    }
}
