use std::{io, sync::Arc, time::Duration};

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use memory_jogger::{data_store::StoreFactory, pocket::Pocket, SavedItemMediator};
use structopt::StructOpt;
use tokio::sync::Mutex;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;
use worker::IoEvent;

use crate::worker::Worker;

mod app;
mod util;
mod worker;

#[derive(Debug, StructOpt)]
#[structopt(about = "Memory Jogger REPL.")]
struct CLIArgs {
    #[structopt(long, env = "MEMORY_JOGGER_DATABASE_URL")]
    database_url: String,
    #[structopt(long, env = "MEMORY_JOGGER_POCKET_CONSUMER_KEY")]
    pocket_consumer_key: String,
    #[structopt(short, long, env = "MEMORY_JOGGER_USER_ID")]
    user_id: i32,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CLIArgs::from_args();

    let (sync_io_tx, sync_io_rx) = std::sync::mpsc::channel::<IoEvent>();
    let app = Arc::new(Mutex::new(App::new(sync_io_tx)));
    let cloned_app = Arc::clone(&app);
    let database_url = args.database_url.clone();
    let pocket_consumer_key = args.pocket_consumer_key.clone();
    let http_client = reqwest::ClientBuilder::new().build()?;
    let user_id = args.user_id;
    std::thread::spawn(move || {
        let store_factory = StoreFactory::new(&database_url).unwrap();
        let pocket = Pocket::new(pocket_consumer_key, &http_client);
        let mut user_store = store_factory.create_user_store();
        let mut saved_item_store = store_factory.create_saved_item_store();
        let user = user_store.get_user(user_id).unwrap();
        let user_pocket_access_token = user.pocket_access_token().unwrap();
        let user_pocket = pocket.for_user(user_pocket_access_token);
        let mediator =
            SavedItemMediator::new(&user_pocket, saved_item_store.as_mut(), user_store.as_mut());
        let mut worker = Worker::new(&app, mediator, &http_client);
        start_tokio(sync_io_rx, &mut worker);
    });
    // The UI must run in the "main" thread
    start_ui(&cloned_app).await?;

    Ok(())
}

#[tokio::main]
async fn start_tokio(io_rx: std::sync::mpsc::Receiver<IoEvent>, worker: &mut Worker) {
    while let Ok(io_event) = io_rx.recv() {
        worker.handle_io_event(io_event).await;
    }
}

async fn start_ui(app: &Arc<Mutex<App>>) -> Result<()> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let res = run_app(&mut terminal, app).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{:?}", err)
    }

    Ok(())
}

async fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &Arc<Mutex<App>>) -> io::Result<()> {
    let mut is_first_render = true;
    loop {
        let mut app = app.lock().await;
        terminal.draw(|f| ui(f, &app))?;

        if event::poll(Duration::from_millis(250)).unwrap() {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter => {
                        // TODO: change error to enum Message { Info(String), Error(String) }
                        app.error.clear();
                        if app.input.is_empty() {
                            // ignore
                        } else if "archive".starts_with(&app.input) {
                            let item = app.saved_item.clone();
                            if let Some(saved_item) = item {
                                app.dispatch(IoEvent::ArchiveItem(saved_item));
                                app.dispatch(IoEvent::GetRandomItem);
                            }
                        } else if "delete".starts_with(&app.input) {
                            let item = app.saved_item.clone();
                            if let Some(saved_item) = item {
                                app.dispatch(IoEvent::DeleteItem(saved_item));
                                app.dispatch(IoEvent::GetRandomItem);
                            }
                        } else if "favorite".starts_with(&app.input) {
                            let item = app.saved_item.clone();
                            if let Some(saved_item) = item {
                                app.dispatch(IoEvent::FavoriteItem(saved_item));
                            }
                        } else if "next".starts_with(&app.input) {
                            app.dispatch(IoEvent::GetRandomItem);
                        } else if "quit".starts_with(&app.input) {
                            return Ok(());
                        } else {
                            app.error = format!("Unknown command: {}", app.input);
                        }
                        app.input.clear();
                    }
                    KeyCode::Char(c) => {
                        app.input.push(c);
                    }
                    KeyCode::Backspace => {
                        app.input.pop();
                    }
                    _ => {}
                }
            }
        }

        if is_first_render {
            app.dispatch(IoEvent::GetRandomItem);
            is_first_render = false;
        }
    }
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(1), // Help message
                Constraint::Length(1), // Error message
                Constraint::Length(3), // Command prompt
                Constraint::Min(4),    // item_info
                Constraint::Length(1), // post url
                Constraint::Length(1), // wayback url
                Constraint::Min(2),    // HN discussions
            ]
            .as_ref(),
        )
        .split(f.size());

    let help_message = vec![Span::raw(
        "(a)rchive, (d)elete, (f)avorite, (n)ext, (q)uit, (Enter) to submit",
    )];
    let help_msg = Text::from(Spans::from(help_message));
    let help_msg = Paragraph::new(help_msg);
    f.render_widget(help_msg, chunks[0]);

    let error_msg = vec![Spans::from(Span::raw(app.error.clone()))];
    let error_msg = Paragraph::new(error_msg).style(Style::default().fg(Color::Red));
    f.render_widget(error_msg, chunks[1]);

    let input = Paragraph::new(app.input.as_ref())
        .block(Block::default().borders(Borders::ALL).title("Command"));
    f.render_widget(input, chunks[2]);
    // Make the cursor visible and ask tui-rs to put it at the specified coordinates after rendering
    f.set_cursor(
        // Put cursor past the end of the input text
        chunks[2].x + app.input.width() as u16 + 1,
        // Move one line down, from the border to the input line
        chunks[2].y + 1,
    );

    let item_info = vec![
        Spans::from(Span::raw(
            app.saved_item
                .clone()
                .map(|item| item.title())
                .unwrap_or_default(),
        )),
        // TODO: wrap the excerpt
        Spans::from(Span::raw(
            app.saved_item
                .clone()
                .map(|item| item.excerpt().unwrap_or_default())
                .unwrap_or_default(),
        )),
        Spans::from(Span::raw(
            app.saved_item
                .clone()
                .map(|item| item.url().unwrap_or_default())
                .unwrap_or_default(),
        )),
        Spans::from(Span::raw(
            app.saved_item
                .clone()
                .map(|item| {
                    item.time_added()
                        .map(|dt| dt.to_string())
                        .unwrap_or_default()
                })
                .unwrap_or_default(),
        )),
    ];
    let item_info = Paragraph::new(item_info);
    f.render_widget(item_info, chunks[3]);

    let resolved_url = vec![Spans::from(Span::raw(
        app.resolved_url.clone().unwrap_or_default(),
    ))];
    let resolved_url = Paragraph::new(resolved_url);
    f.render_widget(resolved_url, chunks[4]);

    let wayback_url = vec![Spans::from(Span::raw(
        app.wayback_url.clone().unwrap_or_default(),
    ))];
    let wayback_url = Paragraph::new(wayback_url);
    f.render_widget(wayback_url, chunks[5]);

    let hn_discussions: Vec<ListItem> = app
        .discussions
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let content = vec![Spans::from(Span::raw(format!("{}: {}", i, m)))];
            ListItem::new(content)
        })
        .collect();
    let hn_discussions = List::new(hn_discussions);
    f.render_widget(hn_discussions, chunks[6]);
}
