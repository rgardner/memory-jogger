//! Sends email digest.

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
    data_store::{GetSavedItemsQuery, SavedItem, SavedItemSort, SavedItemStore, StoreFactory},
    email::{Mail, SendGridAPIClient},
    error::{PocketCleanerError, Result},
    pocket::PocketManager,
    trends::{Geo, Trend, TrendFinder},
    SavedItemMediator,
};
use structopt::StructOpt;

// Email constants
static EMAIL_SUBJECT: &str = "Pocket Cleaner Daily Digest";
const MAX_ITEMS_PER_EMAIL: usize = 4;
const NUM_ITEMS_PER_TREND: usize = 2;
const MAIN_USER_ID: i32 = 1;

#[derive(Debug, StructOpt)]
#[structopt(about = "Sends Pocket Cleaner digest emails.")]
struct CLIArgs {
    #[structopt(short, long)]
    dry_run: bool,
}

fn get_pocket_url(item: &SavedItem) -> String {
    format!("https://app.getpocket.com/read/{}", item.pocket_id())
}

fn get_email_body(
    relevant_items: &[RelevantItem],
    user_id: i32,
    item_store: &SavedItemStore,
) -> Result<String> {
    let mut body = String::new();
    body.push_str("<b>Timely items from your Pocket:</b>");

    if relevant_items.is_empty() {
        body.push_str("Nothing relevant found in your Pocket, returning some items you may not have seen in a while");
        let items = item_store.get_items(&GetSavedItemsQuery {
            user_id,
            sort_by: Some(SavedItemSort::TimeAdded),
            count: Some(3),
        })?;

        body.push_str("<ol>");
        for item in items {
            body.push_str(&format!(
                r#"<li><a href="{}">{}</a></li>"#,
                get_pocket_url(&item),
                item.title(),
            ));
        }
        body.push_str("</ol>");
    } else {
        body.push_str("<ol>");
        for item in relevant_items {
            body.push_str(&format!(
                r#"<li><a href="{}">{}</a> (Why: {})</li>"#,
                get_pocket_url(&item.pocket_item),
                item.pocket_item.title(),
                item.trend
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

async fn try_main() -> Result<()> {
    let args = CLIArgs::from_args();

    env_logger::from_env(Env::default().default_filter_or("warn")).init();

    // Initialize SSL certificates. Do this early-on before any network requests.
    openssl_probe::init_ssl_cert_env_vars();

    // Check required environment variables
    let pocket_consumer_key = get_required_env_var(config::POCKET_CONSUMER_KEY_ENV_VAR)?;
    let sendgrid_api_key = get_required_env_var(config::SENDGRID_API_KEY_ENV_VAR)?;
    let from_email = get_required_env_var(config::FROM_EMAIL_ENV_VAR)?;

    let trend_finder = TrendFinder::new();
    // Request at least 2 days in case it's too early in the morning and there
    // aren't enough trends yet.
    let num_days = 2;
    let trends = trend_finder.daily_trends(&Geo::default(), num_days).await?;

    let store_factory = StoreFactory::new()?;
    let mut user_store = store_factory.create_user_store();
    let user = user_store.get_user(MAIN_USER_ID)?;
    let mut saved_item_store = store_factory.create_saved_item_store();

    {
        let user_pocket_access_token = user.pocket_access_token().ok_or_else(|| {
            PocketCleanerError::Unknown("Main user does not have Pocket access token".into())
        })?;

        let user_pocket =
            PocketManager::new(pocket_consumer_key).for_user(&user_pocket_access_token);
        let mut saved_item_mediator =
            SavedItemMediator::new(&user_pocket, &mut saved_item_store, &mut user_store);
        saved_item_mediator.sync(MAIN_USER_ID).await?;
    }

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

    let mail = Mail {
        from_email,
        to_email: user.email(),
        subject: EMAIL_SUBJECT.into(),
        html_content: get_email_body(&items, user.id(), &saved_item_store)?,
    };
    if args.dry_run {
        println!("{}", mail);
    } else {
        let sendgrid_api_client = SendGridAPIClient::new(sendgrid_api_key);
        sendgrid_api_client.send(&mail).await?;
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
