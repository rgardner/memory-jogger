use std::sync::mpsc::Sender;

use memory_jogger::data_store::SavedItem;

use crate::{util::HnHit, worker::IoEvent};

pub enum Message {
    Info(String),
    Error(String),
}

#[derive(Default)]
pub struct App {
    // common
    pub user_id: i32,
    pub io_tx: Option<Sender<IoEvent>>,
    pub message: Option<Message>,
    // normal
    pub saved_item: Option<SavedItem>,
    pub resolved_url: Option<String>,
    pub wayback_url: Option<String>,
    pub discussions: Vec<HnHit>,
    // wayback prompt
    pub input: String,
    pub show_wayback_prompt: bool,
    pub wayback_prompt_url: Option<String>,
}

impl App {
    pub fn new(user_id: i32, io_tx: Sender<IoEvent>) -> Self {
        Self {
            user_id,
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

    pub fn reset_state(&mut self) {
        self.saved_item = None;
        self.resolved_url = None;
        self.wayback_url = None;
        self.discussions.clear();
    }
}
