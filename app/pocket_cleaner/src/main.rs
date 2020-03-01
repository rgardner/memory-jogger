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

use std::env;

use anyhow::{Context, Result};
use env_logger::Env;

use crate::{
    pocket::PocketManager,
    trends::{Geo, TrendFinder},
};

mod error;
mod pocket;
mod trends;

static POCKET_CONSUMER_KEY_ENV_VAR: &str = "POCKET_CLEANER_CONSUMER_KEY";
static POCKET_USER_ACCESS_TOKEN: &str = "POCKET_TEMP_USER_ACCESS_TOKEN";

fn get_pocket_consumer_key() -> Result<String> {
    let key = POCKET_CONSUMER_KEY_ENV_VAR;
    let value = env::var(key).with_context(|| format!("missing app config env var: {}", key))?;
    Ok(value)
}

async fn try_main() -> Result<()> {
    env_logger::from_env(Env::default().default_filter_or("warn")).init();

    let trend_finder = TrendFinder::new();
    let trends = trend_finder.daily_trends(&Geo("US".into())).await?;

    let pocket_consumer_key = get_pocket_consumer_key()?;
    let pocket_manager = PocketManager::new(pocket_consumer_key);
    let user_pocket = pocket_manager.for_user(&env::var(POCKET_USER_ACCESS_TOKEN)?);

    let mut items = Vec::new();
    for trend in trends[..5].iter() {
        let mut relevant_items = user_pocket.get_items(&trend.name()).await?;
        items.extend(relevant_items.drain(..5).map(|i| (trend.name(), i)));
    }

    for (i, item) in items.iter().enumerate() {
        println!("{} {} (Why: {})", i, item.1.title(), item.0);
    }

    Ok(())
}

#[actix_rt::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("{}", e);
    }
}
