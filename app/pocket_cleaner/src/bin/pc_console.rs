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

use env_logger::Env;
use pocket_cleaner::{
    config::{self, get_required_env_var},
    data_store::{SavedItemStore, StoreFactory, UserStore},
    error::{PocketCleanerError, Result},
    pocket::PocketManager,
    trends::{Geo, TrendFinder},
    SavedItemMediator,
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Interacts with Pocket Cleaner DB and APIs.")]
enum CLIArgs {
    /// View latest trends.
    Trends,
    SyncSavedItems {
        #[structopt(long)]
        user_id: i32,
    },
    /// Retrieve items from the database.
    DB(DBSubcommand),
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
}

#[derive(Debug, StructOpt)]
enum SavedItemDBSubcommand {
    Add {
        #[structopt(long)]
        user_id: i32,
        #[structopt(long)]
        pocket_id: String,
        #[structopt(long)]
        title: String,
    },
    List,
}

async fn run_trends_subcommand() -> Result<()> {
    let trend_finder = TrendFinder::new();
    let trends = trend_finder.daily_trends(&Geo::default()).await?;
    for trend in trends.iter().take(5) {
        println!("{}", trend);
    }

    Ok(())
}

async fn run_sync_saved_items_subcommand(user_id: i32) -> Result<()> {
    // Check required environment variables
    let pocket_consumer_key = get_required_env_var(config::POCKET_CONSUMER_KEY_ENV_VAR)?;

    let store_factory = StoreFactory::new()?;
    let mut user_store = store_factory.create_user_store();
    let user = user_store.get_user(user_id)?;
    let user_pocket_access_token = user.pocket_access_token().ok_or_else(|| {
        PocketCleanerError::Unknown("Main user does not have Pocket access token".into())
    })?;

    let pocket_manager = PocketManager::new(pocket_consumer_key);
    let user_pocket = pocket_manager.for_user(&user_pocket_access_token);

    let mut saved_item_store = store_factory.create_saved_item_store();
    let mut saved_item_mediator =
        SavedItemMediator::new(&user_pocket, &mut saved_item_store, &mut user_store);
    saved_item_mediator.sync(user_id).await?;
    Ok(())
}

fn run_user_db_subcommand(cmd: &UserDBSubcommand, user_store: &mut UserStore) -> Result<()> {
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
    }
    Ok(())
}

fn run_saved_item_db_subcommand(
    cmd: &SavedItemDBSubcommand,
    saved_item_store: &mut SavedItemStore,
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
        SavedItemDBSubcommand::List => {
            let results = saved_item_store.filter_saved_items(5)?;
            println!("Displaying {} saved items", results.len());
            for saved_item in results {
                println!("{}", saved_item.title());
                println!("----------\n");
                println!("{}", saved_item.body().unwrap_or_else(|| "none".into()));
            }
        }
    }
    Ok(())
}

fn run_db_subcommand(cmd: &DBSubcommand) -> Result<()> {
    let store_factory = StoreFactory::new()?;
    match cmd {
        DBSubcommand::User(sub) => {
            run_user_db_subcommand(sub, &mut store_factory.create_user_store())
        }

        DBSubcommand::SavedItem(sub) => {
            run_saved_item_db_subcommand(sub, &mut store_factory.create_saved_item_store())
        }
    }
}

async fn try_main() -> Result<()> {
    let args = CLIArgs::from_args();
    env_logger::from_env(Env::default().default_filter_or("warn")).init();
    match args {
        CLIArgs::Trends => run_trends_subcommand().await?,
        CLIArgs::SyncSavedItems { user_id } => run_sync_saved_items_subcommand(user_id).await?,
        CLIArgs::DB(cmd) => run_db_subcommand(&cmd)?,
    }

    Ok(())
}

#[actix_rt::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}