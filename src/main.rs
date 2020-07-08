//! Surfaces items from your [Pocket](https://getpocket.com) library based on
//! trending headlines.

#![deny(
    clippy::all,
    missing_debug_implementations,
    missing_copy_implementations,
    trivial_casts,
    trivial_numeric_casts,
    unsafe_code,
    unused_import_braces,
    unused_qualifications
)]

use std::{convert::TryInto, io, str::FromStr};

use env_logger::Env;
use memory_jogger::{
    config::{self, get_required_env_var},
    data_store::{self, GetSavedItemsQuery, SavedItem, SavedItemStore, StoreFactory, UserStore},
    email::{Mail, SendGridAPIClient},
    error::{PocketCleanerError, Result},
    pocket::{PocketItem, PocketManager, PocketRetrieveQuery},
    trends::{Geo, Trend, TrendFinder},
    SavedItemMediator,
};
use structopt::StructOpt;

// Email constants
static EMAIL_SUBJECT: &str = "Pocket Cleaner Daily Digest";
const MAX_ITEMS_PER_EMAIL: usize = 4;
const NUM_ITEMS_PER_TREND: usize = 2;
const MAIN_USER_ID: i32 = 1;

#[derive(StructOpt, Debug)]
#[structopt(about = "Finds items from your Pocket library that are relevant to trending news.")]
struct CLIArgs {
    #[structopt(long, env = "DATABASE_URL")]
    database_url: String,
    #[structopt(subcommand)]
    cmd: CLICommand,
}

#[derive(StructOpt, Debug)]
enum CLICommand {
    /// Shows relevant Pocket items for latest trends.
    Relevant(RelevantSubcommand),
    /// Shows latest trends.
    Trends,
    /// Interacts with Pocket.
    Pocket(PocketSubcommand),
    /// Syncs and searches saved items.
    SavedItems(SavedItemsSubcommand),
    /// Retrieves items from the database.
    DB(DBSubcommand),
}

#[derive(Debug, StructOpt)]
struct RelevantSubcommand {
    #[structopt(long)]
    email: bool,
    /// If specified and `email` is true, the email will only be displayed,
    /// not sent.
    #[structopt(short, long)]
    dry_run: bool,
}

#[derive(Debug, StructOpt)]
enum PocketSubcommand {
    Auth,
    Retrieve {
        #[structopt(short, long)]
        user_id: i32,
        #[structopt(long)]
        search: Option<String>,
    },
}

#[derive(Debug, StructOpt)]
enum SavedItemsSubcommand {
    Search {
        #[structopt()]
        query: String,
        #[structopt(short, long)]
        user_id: i32,
        #[structopt(long)]
        limit: Option<i32>,
    },
    Sync {
        #[structopt(short, long)]
        user_id: i32,
        /// Resync all items, replacing existing data in the database.
        #[structopt(long)]
        full: bool,
    },
}

#[derive(Debug, StructOpt)]
enum DBSubcommand {
    User(UserDBSubcommand),
    SavedItem(SavedItemDBSubcommand),
}

#[derive(Debug, StructOpt)]
enum UserDBSubcommand {
    Add {
        #[structopt(long)]
        email: String,
        #[structopt(long)]
        pocket_access_token: Option<String>,
    },
    List,
    Update {
        #[structopt(long)]
        id: i32,
        #[structopt(long)]
        email: Option<String>,
        #[structopt(long)]
        pocket_access_token: Option<String>,
    },
    Delete {
        #[structopt(long)]
        id: i32,
    },
}

#[derive(Clone, Debug)]
enum SavedItemSortBy {
    TimeAdded,
}

impl FromStr for SavedItemSortBy {
    type Err = PocketCleanerError;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "time_added" => Ok(Self::TimeAdded),
            _ => Err(PocketCleanerError::InvalidArgument(format!(
                "sort by: {}",
                s
            ))),
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

#[derive(Debug, StructOpt)]
enum SavedItemDBSubcommand {
    Add {
        #[structopt(short, long)]
        user_id: i32,
        #[structopt(long)]
        pocket_id: String,
        #[structopt(long)]
        title: String,
    },
    List {
        #[structopt(short, long)]
        user_id: i32,
        #[structopt(long)]
        sort: Option<SavedItemSortBy>,
    },
    Delete {
        #[structopt(short, long)]
        user_id: i32,
    },
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
    relevant_items: &[RelevantItem],
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

        body.push_str("<ol>\n");
        for item in items {
            body.push_str(&format!(
                r#"<li><a href="{}">{}</a> (<a href="{}">Fallback</a>)</li>"#,
                get_pocket_url(&item),
                item.title(),
                get_pocket_fallback_url(&item.title()),
            ));
        }
        body.push_str("</ol>");
    } else {
        body.push_str("<ol>");
        for item in relevant_items {
            body.push_str(&format!(
                r#"<li><a href="{}">{}</a> (<a href="{}">Fallback</a>) (Why: <a href="{}">{}</a>)</li>"#,
                get_pocket_url(&item.pocket_item),
                item.pocket_item.title(),
                get_pocket_fallback_url(&item.pocket_item.title()),
                item.trend.explore_link(),
                item.trend.name(),
            ));
        }
        body.push_str("</ol>");
    }

    Ok(body)
}

struct RelevantItem {
    pub pocket_item: SavedItem,
    pub trend: Trend,
}

async fn run_relevant_subcommand(cmd: &RelevantSubcommand, database_url: &str) -> Result<()> {
    log::info!("finding trends");
    let trend_finder = TrendFinder::new();
    // Request at least 2 days in case it's too early in the morning and there
    // aren't enough trends yet.
    let num_days = 2;
    let trends = trend_finder.daily_trends(&Geo::default(), num_days).await?;

    let store_factory = StoreFactory::new(database_url)?;
    let mut user_store = store_factory.create_user_store();
    let user = user_store.get_user(MAIN_USER_ID)?;
    let mut saved_item_store = store_factory.create_saved_item_store();

    {
        let user_pocket_access_token = user.pocket_access_token().ok_or_else(|| {
            PocketCleanerError::Unknown("Main user does not have Pocket access token".into())
        })?;

        let pocket_consumer_key = get_required_env_var(config::POCKET_CONSUMER_KEY_ENV_VAR)?;
        let user_pocket =
            PocketManager::new(pocket_consumer_key).for_user(&user_pocket_access_token);
        let mut saved_item_mediator =
            SavedItemMediator::new(&user_pocket, saved_item_store.as_mut(), user_store.as_mut());
        log::info!("syncing database with Pocket");
        saved_item_mediator.sync(MAIN_USER_ID).await?;
    }

    log::info!("searching for relevant items");
    let mut items = Vec::new();
    for trend in trends {
        let relevant_items = saved_item_store.get_items_by_keyword(user.id(), &trend.name())?;
        items.extend(
            relevant_items
                .into_iter()
                .take(NUM_ITEMS_PER_TREND)
                .map(|item| RelevantItem {
                    pocket_item: item,
                    trend: trend.clone(),
                }),
        );
        if items.len() > MAX_ITEMS_PER_EMAIL {
            break;
        }
    }

    if cmd.email {
        let from_email = get_required_env_var(config::FROM_EMAIL_ENV_VAR)?;
        let mail = Mail {
            from_email,
            to_email: user.email(),
            subject: EMAIL_SUBJECT.into(),
            html_content: get_email_body(&items, user.id(), saved_item_store.as_ref())?,
        };
        if cmd.dry_run {
            println!("{}", mail);
        } else {
            let sendgrid_api_key = get_required_env_var(config::SENDGRID_API_KEY_ENV_VAR)?;
            let sendgrid_api_client = SendGridAPIClient::new(sendgrid_api_key);
            sendgrid_api_client.send(&mail).await?;
        }
    } else {
        for item in &items {
            println!(
                "{} ({}), Why: {} ({})",
                item.pocket_item.title(),
                get_pocket_url(&item.pocket_item),
                item.trend.name(),
                item.trend.explore_link(),
            );
        }
    }

    Ok(())
}

async fn run_trends_subcommand() -> Result<()> {
    let trend_finder = TrendFinder::new();
    let trends = trend_finder
        .daily_trends(&Geo::default(), 1 /*num_days*/)
        .await?;
    for trend in trends.iter().take(5) {
        println!("{}", trend);
    }

    Ok(())
}

async fn run_pocket_subcommand(cmd: &PocketSubcommand, database_url: &str) -> Result<()> {
    match cmd {
        PocketSubcommand::Auth => {
            // Check required environment variables
            let pocket_consumer_key = get_required_env_var(config::POCKET_CONSUMER_KEY_ENV_VAR)?;

            // Get request token
            let pocket = PocketManager::new(pocket_consumer_key);
            let (auth_url, request_token) = pocket.get_auth_url().await?;
            println!("Follow URL to authorize application: {}", auth_url);
            let _ = io::stdin().read_line(&mut String::new());
            for _ in 0..3 {
                match pocket.authorize(&request_token).await {
                    Ok(access_token) => {
                        println!("{}", access_token);
                        break;
                    }
                    Err(PocketCleanerError::UserPocketAuth) => continue,
                    Err(e) => return Err(e),
                }
            }
        }
        PocketSubcommand::Retrieve { user_id, search } => {
            // Check required environment variables
            let pocket_consumer_key = get_required_env_var(config::POCKET_CONSUMER_KEY_ENV_VAR)?;

            let store_factory = StoreFactory::new(database_url)?;
            let user_store = store_factory.create_user_store();
            let user = user_store.get_user(*user_id)?;
            let user_pocket_access_token = user.pocket_access_token().ok_or_else(|| {
                PocketCleanerError::Unknown("Main user does not have Pocket access token".into())
            })?;

            let pocket = PocketManager::new(pocket_consumer_key);
            let user_pocket = pocket.for_user(&user_pocket_access_token);
            let items_page = user_pocket
                .retrieve(&PocketRetrieveQuery {
                    search: search.as_deref(),
                    ..Default::default()
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

async fn run_saved_items_subcommand(cmd: &SavedItemsSubcommand, database_url: &str) -> Result<()> {
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
                    println!("{}", result.title());
                }
            }
        }
        SavedItemsSubcommand::Sync { user_id, full } => {
            // Check required environment variables
            let pocket_consumer_key = get_required_env_var(config::POCKET_CONSUMER_KEY_ENV_VAR)?;

            let store_factory = StoreFactory::new(database_url)?;
            let mut user_store = store_factory.create_user_store();
            let user = user_store.get_user(*user_id)?;
            let user_pocket_access_token = user.pocket_access_token().ok_or_else(|| {
                PocketCleanerError::Unknown("Main user does not have Pocket access token".into())
            })?;

            let pocket_manager = PocketManager::new(pocket_consumer_key);
            let user_pocket = pocket_manager.for_user(&user_pocket_access_token);

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
    }

    Ok(())
}

fn run_user_db_subcommand(cmd: &UserDBSubcommand, user_store: &mut dyn UserStore) -> Result<()> {
    match cmd {
        UserDBSubcommand::Add {
            email,
            pocket_access_token,
        } => {
            let user = user_store.create_user(&email, pocket_access_token.as_deref())?;
            println!("\nSaved user {} with id {}", user.email(), user.id());
        }
        UserDBSubcommand::List => {
            let results = user_store.filter_users(5)?;
            println!("Displaying {} users", results.len());
            for user in results {
                println!(
                    "{} ({})",
                    user.email(),
                    user.pocket_access_token().unwrap_or_else(|| "none".into())
                );
            }
        }
        UserDBSubcommand::Update {
            id,
            email,
            pocket_access_token,
        } => {
            user_store.update_user(*id, email.as_deref(), pocket_access_token.as_deref())?;
            println!("Updated user with id {}", id);
        }
        UserDBSubcommand::Delete { id } => {
            user_store.delete_user(*id)?;
            println!("Successfully deleted user with id {}", id);
        }
    }
    Ok(())
}

fn run_saved_item_db_subcommand(
    cmd: &SavedItemDBSubcommand,
    saved_item_store: &mut dyn SavedItemStore,
) -> Result<()> {
    match cmd {
        SavedItemDBSubcommand::Add {
            user_id,
            pocket_id,
            title,
        } => {
            let saved_item = saved_item_store.create_saved_item(*user_id, &pocket_id, &title)?;
            println!("\nSaved item {} with id {}", title, saved_item.id());
        }
        SavedItemDBSubcommand::List { user_id, sort } => {
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
                        .map(|t| t.to_string())
                        .unwrap_or_else(|| "none".into())
                );
            }
        }
        SavedItemDBSubcommand::Delete { user_id } => {
            saved_item_store.delete_all(*user_id)?;
        }
    }
    Ok(())
}

fn run_db_subcommand(cmd: &DBSubcommand, database_url: &str) -> Result<()> {
    let store_factory = StoreFactory::new(database_url)?;
    match cmd {
        DBSubcommand::User(sub) => {
            run_user_db_subcommand(sub, store_factory.create_user_store().as_mut())
        }

        DBSubcommand::SavedItem(sub) => {
            run_saved_item_db_subcommand(sub, store_factory.create_saved_item_store().as_mut())
        }
    }
}

async fn try_main() -> Result<()> {
    let args = CLIArgs::from_args();
    env_logger::from_env(Env::default().default_filter_or("info")).init();
    match args.cmd {
        CLICommand::Relevant(cmd) => run_relevant_subcommand(&cmd, &args.database_url).await?,
        CLICommand::Trends => run_trends_subcommand().await?,
        CLICommand::Pocket(cmd) => run_pocket_subcommand(&cmd, &args.database_url).await?,
        CLICommand::SavedItems(cmd) => run_saved_items_subcommand(&cmd, &args.database_url).await?,
        CLICommand::DB(cmd) => run_db_subcommand(&cmd, &args.database_url)?,
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
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
}
