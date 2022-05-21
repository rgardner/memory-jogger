//! Surfaces items from your [Pocket](https://getpocket.com) library based on
//! trending headlines.

#![warn(
    clippy::all,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]

use std::{
    collections::HashMap,
    convert::TryInto,
    env,
    io::{self, Read},
    str::FromStr,
};

use anyhow::{anyhow, Context, Result};
use clap::{CommandFactory, Parser};
use clap_complete::Shell;
use env_logger::Env;
use memory_jogger::{
    data_store::{self, GetSavedItemsQuery, SavedItem, SavedItemStore, StoreFactory, UserStore},
    email::{Mail, SendGridApiClient},
    pocket::{Pocket, PocketItem, PocketItemId, PocketRetrieveQuery},
    trends::{Geo, Trend, TrendFinder},
    SavedItemMediator,
};

static USER_ID_ENV_VAR: &str = "MEMORY_JOGGER_USER_ID";
static POCKET_CONSUMER_KEY_ENV_VAR: &str = "MEMORY_JOGGER_POCKET_CONSUMER_KEY";
static SENDGRID_API_KEY_ENV_VAR: &str = "MEMORY_JOGGER_SENDGRID_API_KEY";
static MISSING_POCKET_ACCESS_TOKEN_ERROR_MSG: &str = "User does not have a Pocket access token. \
    See the README to authorize the app to access your Pocket data and save the user authorization \
    token";
static EMAIL_SUBJECT: &str = "Memory Jogger Daily Digest";
const MAX_ITEMS_PER_EMAIL: usize = 4;
const NUM_ITEMS_PER_TREND: usize = 2;

fn get_required_env_var(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("missing app config env var: {}", key))
}

#[derive(Parser, Debug)]
#[clap(about = "Finds items from your Pocket library that are relevant to trending news.")]
struct CliArgs {
    #[clap(long, env = "DATABASE_URL")]
    database_url: String,
    /// Shows trace messages, including potentially sensitive HTTP data.
    #[clap(long)]
    trace: bool,
    #[clap(subcommand)]
    cmd: CliCommand,
}

#[derive(clap::Subcommand, Debug)]
enum CliCommand {
    /// Shows relevant Pocket items for latest trends.
    Relevant(RelevantSubcommand),
    /// Shows latest trends.
    Trends,
    /// Interacts with Pocket.
    #[clap(subcommand)]
    Pocket(PocketSubcommand),
    /// Syncs and searches saved items.
    #[clap(subcommand)]
    SavedItems(SavedItemsSubcommand),
    /// Retrieves items from the database.
    #[clap(subcommand)]
    Db(DbSubcommand),
    /// Generates shell completions.
    #[clap(subcommand)]
    Completions(CompletionsSubcommand),
}

#[derive(Debug, clap::Args)]
struct RelevantSubcommand {
    #[clap(short, long, env = USER_ID_ENV_VAR)]
    user_id: i32,
    #[clap(long)]
    email: bool,
    /// From email address: only required when `--email` is supplied.
    #[clap(long, env = "MEMORY_JOGGER_FROM_EMAIL")]
    from_email: Option<String>,
    /// If specified and `--email` is specified, the email will only be
    /// displayed, not sent.
    #[clap(short, long)]
    dry_run: bool,
}

#[derive(Debug, clap::Subcommand)]
enum PocketSubcommand {
    Auth {
        #[clap(short, long, env = POCKET_CONSUMER_KEY_ENV_VAR)]
        consumer_key: String,
    },
    Retrieve {
        #[clap(short, long, env = USER_ID_ENV_VAR)]
        user_id: i32,
        #[clap(long)]
        search: Option<String>,
    },
}

#[derive(Debug, Parser)]
enum SavedItemsSubcommand {
    Search {
        #[clap()]
        query: String,
        #[clap(short, long, env = USER_ID_ENV_VAR)]
        user_id: i32,
        #[clap(long)]
        limit: Option<i32>,
    },
    Sync {
        #[clap(short, long, env = USER_ID_ENV_VAR)]
        user_id: i32,
        /// Resync all items, replacing existing data in the database.
        #[clap(long)]
        full: bool,
    },
    Archive {
        #[clap(short, long, env = USER_ID_ENV_VAR)]
        user_id: i32,
        #[clap(short, long)]
        item_id: i32,
    },
    Delete {
        #[clap(short, long, env = USER_ID_ENV_VAR)]
        user_id: i32,
        #[clap(short, long)]
        item_id: i32,
    },
    Favorite {
        #[clap(short, long, env = USER_ID_ENV_VAR)]
        user_id: i32,
        #[clap(short, long)]
        item_id: i32,
    },
}

#[derive(Debug, clap::Subcommand)]
enum DbSubcommand {
    #[clap(subcommand)]
    User(UserDbSubcommand),
    #[clap(subcommand)]
    SavedItem(SavedItemDbSubcommand),
}

#[derive(Debug, clap::Subcommand)]
enum UserDbSubcommand {
    Add {
        #[clap(long)]
        email: String,
        #[clap(long)]
        pocket_access_token: Option<String>,
    },
    List,
    Update {
        #[clap(long)]
        id: i32,
        #[clap(long)]
        email: Option<String>,
        #[clap(long)]
        pocket_access_token: Option<String>,
    },
    /// Deletes all users or just the user specified by `id`. Will prompt if
    /// deleting all users and not passing `--yes`.
    Delete {
        #[clap(long)]
        id: Option<i32>,
        /// Accepts any prompts.
        #[clap(short, long)]
        yes: bool,
    },
}

#[derive(Clone, Debug)]
enum SavedItemSortBy {
    TimeAdded,
}

impl FromStr for SavedItemSortBy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "time_added" => Ok(Self::TimeAdded),
            _ => Err(anyhow!("sort by: {}", s)),
        }
    }
}

impl From<SavedItemSortBy> for data_store::SavedItemSort {
    fn from(sort: SavedItemSortBy) -> Self {
        match sort {
            SavedItemSortBy::TimeAdded => Self::TimeAdded,
        }
    }
}

#[derive(Debug, clap::Subcommand)]
enum SavedItemDbSubcommand {
    Add {
        #[clap(short, long, env = USER_ID_ENV_VAR)]
        user_id: i32,
        #[clap(long)]
        pocket_id: PocketItemId,
        #[clap(long)]
        title: String,
    },
    List {
        #[clap(short, long, env = USER_ID_ENV_VAR)]
        user_id: i32,
        #[clap(long)]
        sort: Option<SavedItemSortBy>,
    },
    Delete {
        #[clap(short, long, env = USER_ID_ENV_VAR)]
        user_id: i32,
    },
}

#[derive(Debug, Clone, clap::Subcommand)]
enum CompletionsSubcommand {
    Bash,
    Zsh,
}

fn get_pocket_url(item: &SavedItem) -> String {
    format!("https://app.getpocket.com/read/{}", item.pocket_id())
}

fn get_pocket_fallback_url(item_title: &str) -> reqwest::Url {
    // Use `unwrap` here because only logic errors can occur.
    let base = reqwest::Url::parse("https://app.getpocket.com/search/").unwrap();
    base.join(item_title).unwrap()
}

fn get_email_body(
    relevant_items: &HashMap<Trend, Vec<SavedItem>>,
    user_id: i32,
    item_store: &dyn SavedItemStore,
) -> Result<String> {
    let mut body = String::new();
    body.push_str("<b>Timely items from your Pocket:</b>");

    if relevant_items.is_empty() {
        body.push_str("Nothing relevant found in your Pocket, returning some items you may not have seen in a while\n");
        let items = item_store.get_items(&GetSavedItemsQuery {
            user_id,
            sort_by: Some(data_store::SavedItemSort::TimeAdded),
            count: Some(3),
        })?;

        body.push_str("<ol>");
        for item in items {
            body.push_str(&format!(
                r#"<li><a href="{}">{}</a> (<a href="{}">Fallback</a>)</li>"#,
                get_pocket_url(&item),
                item.title(),
                get_pocket_fallback_url(&item.title()),
            ));
        }
    } else {
        body.push_str("<ol>");
        for (trend, items) in relevant_items {
            if !items.is_empty() {
                body.push_str(&format!(
                    r#"<li><a href="{}">Trend: {}</a><ol>"#,
                    trend.explore_link(),
                    trend.name()
                ));
                for item in items {
                    body.push_str(&format!(
                        r#"<li><a href="{}">{}</a> (<a href="{}">Fallback</a>)</li>"#,
                        get_pocket_url(item),
                        item.title(),
                        get_pocket_fallback_url(&item.title()),
                    ));
                }
                body.push_str("</ol></li>");
            }
        }
    }
    body.push_str("</ol>");

    Ok(body)
}

async fn run_relevant_subcommand(
    cmd: &RelevantSubcommand,
    database_url: &str,
    http_client: &reqwest::Client,
) -> Result<()> {
    log::info!("finding trends");
    let trend_finder = TrendFinder::new(http_client);
    // Request at least 2 days in case it's too early in the morning and there
    // aren't enough trends yet.
    let num_days = 2;
    let trends = trend_finder.daily_trends(&Geo::default(), num_days).await?;

    let store_factory = StoreFactory::new(database_url)?;
    let mut user_store = store_factory.create_user_store();
    let user = user_store.get_user(cmd.user_id)?;
    let mut saved_item_store = store_factory.create_saved_item_store();

    {
        let user_pocket_access_token = user
            .pocket_access_token()
            .ok_or_else(|| anyhow!(MISSING_POCKET_ACCESS_TOKEN_ERROR_MSG))?;

        let pocket_consumer_key = get_required_env_var(POCKET_CONSUMER_KEY_ENV_VAR)?;
        let pocket = Pocket::new(pocket_consumer_key, http_client);
        let user_pocket = pocket.for_user(user_pocket_access_token);
        let mut saved_item_mediator =
            SavedItemMediator::new(&user_pocket, saved_item_store.as_mut(), user_store.as_mut());
        log::info!("syncing database with Pocket");
        saved_item_mediator.sync(user.id()).await?;
    }

    log::info!("searching for relevant items");
    let mut items: HashMap<_, Vec<_>> = HashMap::new();
    for trend in trends {
        let relevant_items = saved_item_store.get_items_by_keyword(user.id(), &trend.name())?;
        if !relevant_items.is_empty() {
            items.insert(
                trend,
                relevant_items
                    .into_iter()
                    .take(NUM_ITEMS_PER_TREND)
                    .collect(),
            );
            if items.values().map(Vec::len).sum::<usize>() > MAX_ITEMS_PER_EMAIL {
                break;
            }
        }
    }

    if cmd.email {
        let mail = Mail {
            from_email: cmd
                .from_email
                .clone()
                .ok_or_else(|| anyhow!("--from-email is required because --email was supplied"))?,
            to_email: user.email(),
            subject: EMAIL_SUBJECT.into(),
            html_content: get_email_body(&items, user.id(), saved_item_store.as_ref())?,
        };

        if cmd.dry_run {
            println!("{}", mail);
        } else {
            let sendgrid_api_key = get_required_env_var(SENDGRID_API_KEY_ENV_VAR)?;
            let sendgrid_api_client = SendGridApiClient::new(sendgrid_api_key, http_client);
            sendgrid_api_client.send(mail).await?;
        }
    } else if items.is_empty() {
        println!("Nothing relevant found in your Pocket, returning some items you may not have seen in a while\n");
        let items = saved_item_store.get_items(&GetSavedItemsQuery {
            user_id: user.id(),
            sort_by: Some(data_store::SavedItemSort::TimeAdded),
            count: Some(3),
        })?;
        for item in items {
            println!("{}: {}", item.title(), get_pocket_url(&item));
        }
    } else {
        for (trend, items) in &items {
            if !items.is_empty() {
                println!("Trend {}: {}", trend.name(), trend.explore_link());
                for item in items {
                    println!("\t{}: {}", item.title(), get_pocket_url(item));
                }
            }
        }
    }

    Ok(())
}

async fn run_trends_subcommand(http_client: &reqwest::Client) -> Result<()> {
    let trend_finder = TrendFinder::new(http_client);
    let trends = trend_finder
        .daily_trends(&Geo::default(), 1 /*num_days*/)
        .await?;
    for trend in trends.iter().take(5) {
        println!("{}", trend);
    }

    Ok(())
}

async fn run_pocket_subcommand(
    cmd: &PocketSubcommand,
    database_url: &str,
    http_client: &reqwest::Client,
) -> Result<()> {
    match cmd {
        PocketSubcommand::Auth { consumer_key } => {
            // Get request token
            let pocket = Pocket::new(consumer_key.clone(), http_client);
            let (auth_url, request_token) = pocket.get_auth_url().await?;
            println!(
                "Follow URL to authorize application: {}\nPress enter to continue",
                auth_url
            );
            io::stdin().read_line(&mut String::new())?;
            let access_token = pocket.authorize(&request_token).await?;
            println!("{}", access_token);
        }
        PocketSubcommand::Retrieve { user_id, search } => {
            // Check required environment variables
            let pocket_consumer_key = get_required_env_var(POCKET_CONSUMER_KEY_ENV_VAR)?;

            let store_factory = StoreFactory::new(database_url)?;
            let user_store = store_factory.create_user_store();
            let user = user_store.get_user(*user_id)?;
            let user_pocket_access_token = user
                .pocket_access_token()
                .ok_or_else(|| anyhow!(MISSING_POCKET_ACCESS_TOKEN_ERROR_MSG))?;

            let pocket = Pocket::new(pocket_consumer_key, http_client);
            let user_pocket = pocket.for_user(user_pocket_access_token);
            let items_page = user_pocket
                .retrieve(&PocketRetrieveQuery {
                    search: search.as_deref(),
                    ..PocketRetrieveQuery::default()
                })
                .await?;
            for item in items_page.items {
                match item {
                    PocketItem::Unread { id, title, .. } => println!("{} - {}", id, title),
                    PocketItem::ArchivedOrDeleted { id, status } => println!("{} ({})", id, status),
                }
            }
        }
    }

    Ok(())
}

async fn run_saved_items_subcommand(
    cmd: &SavedItemsSubcommand,
    database_url: &str,
    http_client: &reqwest::Client,
) -> Result<()> {
    match cmd {
        SavedItemsSubcommand::Search {
            query,
            user_id,
            limit,
        } => {
            let store_factory = StoreFactory::new(database_url)?;
            let saved_item_store = store_factory.create_saved_item_store();
            let results = saved_item_store.get_items_by_keyword(*user_id, query)?;
            if let Some(limit) = limit {
                for result in results.iter().take((*limit).try_into().unwrap()) {
                    println!("{}", result.title());
                }
            } else {
                for result in results {
                    println!("{} - {}", result.id(), result.title());
                }
            }
        }
        SavedItemsSubcommand::Sync { user_id, full } => {
            // Check required environment variables
            let pocket_consumer_key = get_required_env_var(POCKET_CONSUMER_KEY_ENV_VAR)?;

            let store_factory = StoreFactory::new(database_url)?;
            let mut user_store = store_factory.create_user_store();
            let user = user_store.get_user(*user_id)?;
            let user_pocket_access_token = user
                .pocket_access_token()
                .ok_or_else(|| anyhow!(MISSING_POCKET_ACCESS_TOKEN_ERROR_MSG))?;

            let pocket_manager = Pocket::new(pocket_consumer_key, http_client);
            let user_pocket = pocket_manager.for_user(user_pocket_access_token);

            let mut saved_item_store = store_factory.create_saved_item_store();
            let mut saved_item_mediator = SavedItemMediator::new(
                &user_pocket,
                saved_item_store.as_mut(),
                user_store.as_mut(),
            );

            if *full {
                saved_item_mediator.sync_full(*user_id).await?;
            } else {
                saved_item_mediator.sync(*user_id).await?;
            }
        }
        SavedItemsSubcommand::Archive { user_id, item_id } => {
            // Check required environment variables
            let pocket_consumer_key = get_required_env_var(POCKET_CONSUMER_KEY_ENV_VAR)?;

            let store_factory = StoreFactory::new(database_url)?;
            let mut user_store = store_factory.create_user_store();
            let user = user_store.get_user(*user_id)?;
            let user_pocket_access_token = user
                .pocket_access_token()
                .ok_or_else(|| anyhow!(MISSING_POCKET_ACCESS_TOKEN_ERROR_MSG))?;

            let pocket_manager = Pocket::new(pocket_consumer_key, http_client);
            let user_pocket = pocket_manager.for_user(user_pocket_access_token);

            let mut saved_item_store = store_factory.create_saved_item_store();
            let mut saved_item_mediator = SavedItemMediator::new(
                &user_pocket,
                saved_item_store.as_mut(),
                user_store.as_mut(),
            );

            saved_item_mediator.archive(*user_id, *item_id).await?;
        }
        SavedItemsSubcommand::Delete { user_id, item_id } => {
            // Check required environment variables
            let pocket_consumer_key = get_required_env_var(POCKET_CONSUMER_KEY_ENV_VAR)?;

            let store_factory = StoreFactory::new(database_url)?;
            let mut user_store = store_factory.create_user_store();
            let user = user_store.get_user(*user_id)?;
            let user_pocket_access_token = user
                .pocket_access_token()
                .ok_or_else(|| anyhow!(MISSING_POCKET_ACCESS_TOKEN_ERROR_MSG))?;

            let pocket_manager = Pocket::new(pocket_consumer_key, http_client);
            let user_pocket = pocket_manager.for_user(user_pocket_access_token);

            let mut saved_item_store = store_factory.create_saved_item_store();
            let mut saved_item_mediator = SavedItemMediator::new(
                &user_pocket,
                saved_item_store.as_mut(),
                user_store.as_mut(),
            );

            saved_item_mediator.delete(*user_id, *item_id).await?;
        }
        SavedItemsSubcommand::Favorite { user_id, item_id } => {
            // Check required environment variables
            let pocket_consumer_key = get_required_env_var(POCKET_CONSUMER_KEY_ENV_VAR)?;

            let store_factory = StoreFactory::new(database_url)?;
            let mut user_store = store_factory.create_user_store();
            let user = user_store.get_user(*user_id)?;
            let user_pocket_access_token = user
                .pocket_access_token()
                .ok_or_else(|| anyhow!(MISSING_POCKET_ACCESS_TOKEN_ERROR_MSG))?;

            let pocket_manager = Pocket::new(pocket_consumer_key, http_client);
            let user_pocket = pocket_manager.for_user(user_pocket_access_token);

            let mut saved_item_store = store_factory.create_saved_item_store();
            let mut saved_item_mediator = SavedItemMediator::new(
                &user_pocket,
                saved_item_store.as_mut(),
                user_store.as_mut(),
            );

            saved_item_mediator.favorite(*item_id).await?;
        }
    }

    Ok(())
}

/// Asks the `question` on stdin.
fn ask(question: &str) -> Result<bool> {
    println!("{} y/[n]", question);
    let mut original_answer = String::new();
    io::stdin().read_to_string(&mut original_answer)?;
    let answer = original_answer.trim().to_lowercase();
    match answer.as_str() {
        "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        _ => Err(anyhow!("Unknown answer: {}", original_answer)),
    }
}

fn run_user_db_subcommand(cmd: &UserDbSubcommand, user_store: &mut dyn UserStore) -> Result<()> {
    match cmd {
        UserDbSubcommand::Add {
            email,
            pocket_access_token,
        } => {
            let user = user_store.create_user(email, pocket_access_token.as_deref())?;
            println!("id: {}", user.id());
        }
        UserDbSubcommand::List => {
            let results = user_store.filter_users(5)?;
            println!("Displaying {} users", results.len());
            for user in results {
                println!(
                    "{}. {} ({})",
                    user.id(),
                    user.email(),
                    user.pocket_access_token().unwrap_or_else(|| "none".into())
                );
            }
        }
        UserDbSubcommand::Update {
            id,
            email,
            pocket_access_token,
        } => {
            user_store.update_user(*id, email.as_deref(), pocket_access_token.as_deref())?;
            println!("Updated user with id {}", id);
        }
        UserDbSubcommand::Delete { id, yes } => {
            if let Some(id) = id {
                user_store.delete_user(*id)?;
                println!("Successfully deleted user with id {}", id);
            } else if *yes || ask("Delete all users?")? {
                user_store.delete_all_users()?;
                println!("Successfully deleted all users");
            }
        }
    }
    Ok(())
}

fn run_saved_item_db_subcommand(
    cmd: &SavedItemDbSubcommand,
    saved_item_store: &mut dyn SavedItemStore,
) -> Result<()> {
    match cmd {
        SavedItemDbSubcommand::Add {
            user_id,
            pocket_id,
            title,
        } => {
            let saved_item = saved_item_store.create_saved_item(*user_id, pocket_id, title)?;
            println!("\nSaved item {} with id {}", title, saved_item.id());
        }
        SavedItemDbSubcommand::List { user_id, sort } => {
            let results = saved_item_store.get_items(&GetSavedItemsQuery {
                user_id: *user_id,
                sort_by: sort.clone().map(Into::into),
                count: Some(5),
            })?;
            println!("Displaying {} saved items", results.len());
            for saved_item in results {
                println!(
                    "{} {}",
                    saved_item.title(),
                    saved_item
                        .time_added()
                        .map_or_else(|| "none".into(), |t| t.to_string())
                );
            }
        }
        SavedItemDbSubcommand::Delete { user_id } => {
            saved_item_store.delete_all(*user_id)?;
        }
    }
    Ok(())
}

fn run_db_subcommand(cmd: &DbSubcommand, database_url: &str) -> Result<()> {
    let store_factory = StoreFactory::new(database_url)?;
    match cmd {
        DbSubcommand::User(sub) => {
            run_user_db_subcommand(sub, store_factory.create_user_store().as_mut())
        }

        DbSubcommand::SavedItem(sub) => {
            run_saved_item_db_subcommand(sub, store_factory.create_saved_item_store().as_mut())
        }
    }
}

fn run_completions_subcommand(cmd: &CompletionsSubcommand, buf: &mut impl io::Write) {
    let shell = match cmd {
        CompletionsSubcommand::Bash => Shell::Bash,
        CompletionsSubcommand::Zsh => Shell::Zsh,
    };
    clap_complete::generate(shell, &mut CliArgs::command(), "memory_jogger", buf);
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = CliArgs::parse();

    let default_log_level = if args.trace { "trace" } else { "info" };
    let mut log_builder =
        env_logger::Builder::from_env(Env::default().default_filter_or(default_log_level));
    if args.trace {
        log_builder.filter_module("reqwest", log::LevelFilter::Trace);
    }
    log_builder.init();

    let http_client = reqwest::ClientBuilder::new()
        .connection_verbose(args.trace)
        .build()?;

    match args.cmd {
        CliCommand::Relevant(cmd) => {
            run_relevant_subcommand(&cmd, &args.database_url, &http_client).await?;
        }
        CliCommand::Trends => run_trends_subcommand(&http_client).await?,
        CliCommand::Pocket(cmd) => {
            run_pocket_subcommand(&cmd, &args.database_url, &http_client).await?;
        }
        CliCommand::SavedItems(cmd) => {
            run_saved_items_subcommand(&cmd, &args.database_url, &http_client).await?;
        }
        CliCommand::Db(cmd) => run_db_subcommand(&cmd, &args.database_url)?,
        CliCommand::Completions(cmd) => run_completions_subcommand(&cmd, &mut io::stdout()),
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    use reqwest::Url;

    #[test]
    fn test_pocket_fallback_url_returns_percent_encoded_query() {
        // Note embedded double quotes and special quotes in `item_title`
        let item_title = r#"C++Now 2017: Bryce Lelbach “C++17 Features""#;

        let actual_url = get_pocket_fallback_url(&item_title);

        let expected_url = "https://app.getpocket.com/search/C++Now%202017:%20Bryce%20Lelbach%20%E2%80%9CC++17%20Features%22";
        let expected_url = Url::parse(expected_url).unwrap();
        assert_eq!(actual_url, expected_url);
    }

    #[test]
    fn test_completions_subcommand_when_called_with_bash_returns_nonempty_completions() {
        let cmd = CompletionsSubcommand::Bash;
        let mut buf = Vec::new();
        run_completions_subcommand(&cmd, &mut buf);
        assert!(!buf.is_empty());
    }

    #[test]
    fn test_completions_subcommand_when_called_with_zsh_returns_nonempty_completions() {
        let cmd = CompletionsSubcommand::Zsh;
        let mut buf = Vec::new();
        run_completions_subcommand(&cmd, &mut buf);
        assert!(!buf.is_empty());
    }
}
