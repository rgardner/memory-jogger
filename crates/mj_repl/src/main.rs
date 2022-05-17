use std::{io, sync::Arc, time::Duration};

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use memory_jogger::{
    data_store::{SavedItemStore, StoreFactory},
    pocket::Pocket,
    SavedItemMediator,
};
use reqwest::Url;
use structopt::StructOpt;
use tokio::sync::Mutex;
#[cfg(target_vendor = "apple")]
use tracing_oslog::OsLogger;
#[cfg(target_vendor = "apple")]
use tracing_subscriber::filter::EnvFilter;
#[cfg(target_vendor = "apple")]
use tracing_subscriber::prelude::*;
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{List, ListItem, Paragraph},
    Frame, Terminal,
};

use mj_repl::{
    app::{App, Message},
    util,
    worker::{IoEvent, Worker},
};

#[cfg(target_vendor = "apple")]
static OS_LOG_SUBSYSTEM: &str = "com.rgardner.memory-jogger";

#[derive(Debug, StructOpt)]
#[structopt(about = "Memory Jogger REPL.")]
struct CLIArgs {
    #[structopt(long, env = "MEMORY_JOGGER_DATABASE_URL")]
    database_url: String,
    #[structopt(long, env = "MEMORY_JOGGER_POCKET_CONSUMER_KEY")]
    pocket_consumer_key: String,
    #[structopt(short, long, env = "MEMORY_JOGGER_USER_ID")]
    user_id: i32,
    #[structopt(long)]
    trace: bool,
    #[structopt(long)]
    item_id: Option<i32>,
}

fn init_logging() -> Result<()> {
    if cfg!(target_vendor = "apple") {
        tracing_subscriber::registry()
            .with(EnvFilter::from_default_env())
            .with(OsLogger::new(OS_LOG_SUBSYSTEM, "default"))
            .init();
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CLIArgs::from_args();
    init_logging()?;

    let database_url = args.database_url.clone();
    let http_client = reqwest::ClientBuilder::new()
        .connection_verbose(args.trace)
        .build()?;
    if let Some(item_id) = args.item_id {
        let store_factory = StoreFactory::new(&database_url)?;
        let saved_item_store = store_factory.create_saved_item_store();
        return display_item(item_id, saved_item_store.as_ref(), &http_client).await;
    }

    let user_id = args.user_id;
    let (sync_io_tx, sync_io_rx) = std::sync::mpsc::channel::<IoEvent>();
    let app = Arc::new(Mutex::new(App::new(user_id, sync_io_tx)));
    let cloned_app = Arc::clone(&app);
    let pocket_consumer_key = args.pocket_consumer_key.clone();
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
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let res = run_app(&mut terminal, app).await;

    // restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
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

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                match (key.code, key.modifiers) {
                    (KeyCode::Char('a'), _) => {
                        // archive
                        let item = app.saved_item.clone();
                        if let Some(saved_item) = item {
                            app.dispatch(IoEvent::ArchiveItem(saved_item));
                            app.dispatch(IoEvent::GetRandomItem);
                        }
                    }
                    (KeyCode::Char('d'), _) => {
                        // delete
                        let item = app.saved_item.clone();
                        if let Some(saved_item) = item {
                            app.dispatch(IoEvent::DeleteItem(saved_item));
                            app.dispatch(IoEvent::GetRandomItem);
                        }
                    }
                    (KeyCode::Char('f'), _) => {
                        // favorite
                        let item = app.saved_item.clone();
                        if let Some(saved_item) = item {
                            app.dispatch(IoEvent::FavoriteItem(saved_item));
                        }
                    }
                    (KeyCode::Char('n'), _) => {
                        // next
                        app.dispatch(IoEvent::GetRandomItem);
                    }
                    (KeyCode::Char('q'), _) | (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                        // quit
                        return Ok(());
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
                Constraint::Min(4),    // item_info
                Constraint::Length(1), // post url
                Constraint::Length(1), // wayback url
                Constraint::Min(2),    // HN discussions
            ]
            .as_ref(),
        )
        .split(f.size());

    let help_message = vec![Span::raw("(a)rchive, (d)elete, (f)avorite, (n)ext, (q)uit")];
    let help_msg = Text::from(Spans::from(help_message));
    let help_msg = Paragraph::new(help_msg);
    f.render_widget(help_msg, chunks[0]);

    let msg_span = match &app.message {
        Some(Message::Info(msg)) => Span::styled(msg, Style::default().fg(Color::White)),
        Some(Message::Error(msg)) => Span::styled(msg, Style::default().fg(Color::Red)),
        None => Span::raw(""),
    };
    let error_msg = vec![Spans::from(msg_span)];
    let error_msg = Paragraph::new(error_msg);
    f.render_widget(error_msg, chunks[1]);

    let item_info = vec![
        Spans::from(Span::raw(
            app.saved_item
                .clone()
                .map(|item| format!("{} ({})", item.title(), item.id()))
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
    f.render_widget(item_info, chunks[2]);

    let resolved_url = vec![Spans::from(Span::raw(
        app.resolved_url.clone().unwrap_or_default(),
    ))];
    let resolved_url = Paragraph::new(resolved_url);
    f.render_widget(resolved_url, chunks[3]);

    let wayback_url = vec![Spans::from(Span::raw(
        app.wayback_url.clone().unwrap_or_default(),
    ))];
    let wayback_url = Paragraph::new(wayback_url);
    f.render_widget(wayback_url, chunks[4]);

    let hn_discussions: Vec<ListItem> = app
        .discussions
        .iter()
        .map(|hit| {
            let content = vec![Spans::from(Span::raw(format!("{}", hit)))];
            ListItem::new(content)
        })
        .collect();
    let hn_discussions = List::new(hn_discussions);
    f.render_widget(hn_discussions, chunks[5]);
}

async fn display_item(
    item_id: i32,
    saved_item_store: &dyn SavedItemStore,
    http_client: &reqwest::Client,
) -> Result<()> {
    let item = if let Some(item) = saved_item_store.get_item(item_id)? {
        item
    } else {
        println!("Item not found");
        return Ok(());
    };

    println!("{}", item.title());
    if let Some(excerpt) = item.excerpt() {
        println!("{}", excerpt);
    }
    if let Some(url) = item.url() {
        println!("{}", url);
    }
    if let Some(time_added) = item.time_added() {
        println!("{}", time_added);
    }

    let raw_url = if let Some(raw_url) = item.url() {
        raw_url
    } else {
        return Ok(());
    };
    if let Ok(url) = Url::parse(&raw_url) {
        let resolved_url = util::resolve_submission_url(url.clone(), http_client).await?;
        if let Some(resolved_url) = &resolved_url {
            println!("{} (submitted URL)", resolved_url);
        }
        let resolved_url = resolved_url
            .and_then(|url| Url::parse(&url).ok())
            .unwrap_or(url);
        let hn_hits = util::get_hn_discussions(resolved_url, http_client).await?;
        for hit in hn_hits {
            println!("{}", hit);
        }
    }
    let archive_url = util::get_wayback_url(raw_url, item.time_added(), http_client).await?;
    if let Some(archive_url) = archive_url {
        println!("{} (Wayback Machine archive)", archive_url);
    }

    Ok(())
}
