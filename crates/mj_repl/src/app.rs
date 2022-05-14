use std::sync::mpsc::Sender;

use crate::worker::IoEvent;

pub struct App {
    pub io_tx: Option<Sender<IoEvent>>,
    pub input: String,
    pub error: String,
    pub discussions: Vec<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            io_tx: None,
            input: String::new(),
            error: String::new(),
            discussions: Vec::new(),
        }
    }
}

impl App {
    pub fn new(io_tx: Sender<IoEvent>) -> Self {
        Self {
            io_tx: Some(io_tx),
            ..Default::default()
        }
    }
}
