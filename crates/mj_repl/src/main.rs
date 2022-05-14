//! Interactive REPL for memory jogger.

use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use std::{error::Error, io};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use unicode_width::UnicodeWidthStr;

/// App holds the state of the application
struct App {
    /// Current value of the input box
    input: String,
    /// Error message if any
    error: String,
    /// History of recorded messages
    messages: Vec<String>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            input: String::new(),
            error: String::new(),
            messages: vec!["link1".into(), "link2".into()],
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::default();
    let res = run_app(&mut terminal, app);

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

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &app))?;

        if let Event::Key(key) = event::read()? {
            match key.code {
                KeyCode::Enter => {
                    app.error.clear();
                    if app.input.is_empty() {
                        // ignore
                    } else if "quit".starts_with(&app.input) {
                        return Ok(());
                    } else if "archive".starts_with(&app.input) {
                        // archive
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
    let text = Text::from(Spans::from(help_message));
    let help_message = Paragraph::new(text);
    f.render_widget(help_message, chunks[0]);

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
        Spans::from(Span::raw("Title")),
        Spans::from(Span::raw("Excerpt")),
        Spans::from(Span::raw("Saved URL")),
        Spans::from(Span::raw("Added")),
    ];
    let item_info = Paragraph::new(item_info);
    f.render_widget(item_info, chunks[3]);

    let resolved_url = vec![Spans::from(Span::raw("Actual URL"))];
    let resolved_url = Paragraph::new(resolved_url);
    f.render_widget(resolved_url, chunks[4]);

    let wayback_url = vec![Spans::from(Span::raw("Wayback URL"))];
    let wayback_url = Paragraph::new(wayback_url);
    f.render_widget(wayback_url, chunks[5]);

    let hn_discussions: Vec<ListItem> = app
        .messages
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
