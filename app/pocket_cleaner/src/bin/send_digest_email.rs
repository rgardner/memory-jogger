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

use std::env;

use anyhow::{Context, Result};
use env_logger::Env;
use pocket_cleaner::{
    email::{Mail, SendGridAPIClient},
    pocket::{PocketItem, PocketManager},
    trends::{Geo, Trend, TrendFinder},
};
use structopt::StructOpt;

static POCKET_CONSUMER_KEY_ENV_VAR: &str = "POCKET_CLEANER_CONSUMER_KEY";
static POCKET_USER_ACCESS_TOKEN_ENV_VAR: &str = "POCKET_TEMP_USER_ACCESS_TOKEN";

static SENDGRID_API_KEY_ENV_VAR: &str = "POCKET_CLEANER_SENDGRID_API_KEY";
static FROM_EMAIL_ENV_VAR: &str = "POCKET_CLEANER_FROM_EMAIL";
static TO_EMAIL_ENV_VAR: &str = "POCKET_CLEANER_TO_EMAIL";

static EMAIL_SUBJECT: &str = "Pocket Cleaner Daily Digest";

#[derive(Debug, StructOpt)]
#[structopt(about = "Sends Pocket Cleaner digest emails.")]
struct CLIArgs {
    #[structopt(short, long)]
    dry_run: bool,
}

fn get_required_env_var(key: &str) -> Result<String> {
    let value = env::var(key).with_context(|| format!("missing app config env var: {}", key))?;
    Ok(value)
}

fn get_pocket_url(item: &PocketItem) -> String {
    format!("https://app.getpocket.com/read/{}", item.id())
}

fn get_email_body(items: &[RelevantItem]) -> String {
    let mut body = String::new();
    body.push_str("<b>Timely items from your Pocket:</b>");

    body.push_str("<ol>");
    for item in items {
        body.push_str(&format!(
            r#"<li><a href="{}">{}</a> (Why: {})</li>"#,
            get_pocket_url(&item.pocket_item),
            item.pocket_item.title(),
            item.trend
        ));
    }
    body.push_str("</ol>");

    body
}

struct RelevantItem {
    pub pocket_item: PocketItem,
    pub trend: Trend,
}

async fn try_main() -> Result<()> {
    let args = CLIArgs::from_args();

    env_logger::from_env(Env::default().default_filter_or("warn")).init();

    // Initialize SSL certificates. Do this early-on before any network requests.
    openssl_probe::init_ssl_cert_env_vars();

    // Check required environment variables
    let pocket_consumer_key = get_required_env_var(POCKET_CONSUMER_KEY_ENV_VAR)?;
    let pocket_user_access_token = get_required_env_var(POCKET_USER_ACCESS_TOKEN_ENV_VAR)?;
    let sendgrid_api_key = get_required_env_var(SENDGRID_API_KEY_ENV_VAR)?;
    let from_email = get_required_env_var(FROM_EMAIL_ENV_VAR)?;
    let to_email = get_required_env_var(TO_EMAIL_ENV_VAR)?;

    let trend_finder = TrendFinder::new();
    let trends = trend_finder.daily_trends(&Geo::default()).await?;

    let pocket_manager = PocketManager::new(pocket_consumer_key);
    let user_pocket = pocket_manager.for_user(&pocket_user_access_token);

    let mut items = Vec::new();
    for trend in trends[..5].iter() {
        let mut relevant_items = user_pocket.get_items(&trend.name()).await?;
        items.extend(relevant_items.drain(..5).map(|item| RelevantItem {
            pocket_item: item,
            trend: trend.clone(),
        }));
    }

    let mail = Mail {
        from_email,
        to_email,
        subject: EMAIL_SUBJECT.into(),
        html_content: get_email_body(&items),
    };
    if args.dry_run {
        println!(
            r"Send email:
        from: {}
        to: {}
        subject: {}
        body:\n{}",
            mail.from_email, mail.to_email, mail.subject, mail.html_content
        );
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
