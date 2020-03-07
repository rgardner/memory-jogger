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

use diesel::prelude::*;
use env_logger::Env;
use pocket_cleaner::{
    config::{self, get_required_env_var},
    db,
    error::Result,
    trends::{Geo, TrendFinder},
};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(about = "Interacts with Pocket Cleaner DB and APIs.")]
enum CLIArgs {
    /// View latest trends.
    Trends,
    /// Retrieve items from the database.
    DB(DBSubcommand),
}

#[derive(Debug, StructOpt)]
enum DBSubcommand {
    Add {
        #[structopt(long)]
        pocket_id: String,
        #[structopt(long)]
        title: String,
        #[structopt(long)]
        body: String,
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

fn run_db_subcommand(cmd: &DBSubcommand) -> Result<()> {
    use db::schema::saved_items::dsl::saved_items;

    let database_url = get_required_env_var(config::DATABASE_URL_ENV_VAR)?;
    let connection = db::establish_connection(&database_url)?;

    match cmd {
        DBSubcommand::Add {
            pocket_id,
            title,
            body,
        } => {
            let saved_item = db::create_saved_item(&connection, &pocket_id, &title, &body)?;
            println!("\nSaved item {} with id {}", title, saved_item.id);
        }
        DBSubcommand::List => {
            let results = saved_items
                .limit(5)
                .load::<db::models::SavedItem>(&connection)
                .expect("Error loading saved items");

            println!("Displaying {} saved items", results.len());
            for saved_item in results {
                println!("{}", saved_item.title);
                println!("----------\n");
                println!("{}", saved_item.body);
            }
        }
    }

    Ok(())
}

async fn try_main() -> Result<()> {
    let args = CLIArgs::from_args();
    env_logger::from_env(Env::default().default_filter_or("warn")).init();
    match args {
        CLIArgs::Trends => run_trends_subcommand().await?,
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
