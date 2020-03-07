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

fn run_user_db_subcommand(cmd: &UserDBSubcommand, db_conn: &PgConnection) -> Result<()> {
    use db::schema::users::dsl::users;
    match cmd {
        UserDBSubcommand::Add {
            email,
            pocket_access_token,
        } => {
            let user = db::create_user(db_conn, &email, pocket_access_token.as_deref())?;
            println!("\nSaved user {} with id {}", user.email, user.id);
        }
        UserDBSubcommand::List => {
            let results = users
                .limit(5)
                .load::<db::models::User>(db_conn)
                .expect("Error loading users");

            println!("Displaying {} users", results.len());
            for user in results {
                println!("{}", user.email);
            }
        }
    }
    Ok(())
}

fn run_saved_item_db_subcommand(cmd: &SavedItemDBSubcommand, db_conn: &PgConnection) -> Result<()> {
    use db::schema::saved_items::dsl::saved_items;
    match cmd {
        SavedItemDBSubcommand::Add {
            user_id,
            pocket_id,
            title,
            body,
        } => {
            let saved_item = db::create_saved_item(db_conn, *user_id, &pocket_id, &title, &body)?;
            println!("\nSaved item {} with id {}", title, saved_item.id);
        }
        SavedItemDBSubcommand::List => {
            let results = saved_items
                .limit(5)
                .load::<db::models::SavedItem>(db_conn)
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
fn run_db_subcommand(cmd: &DBSubcommand) -> Result<()> {
    let db_conn = db::initialize_db()?;
    match cmd {
        DBSubcommand::User(sub) => run_user_db_subcommand(sub, &db_conn),
        DBSubcommand::SavedItem(sub) => run_saved_item_db_subcommand(sub, &db_conn),
    }
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
