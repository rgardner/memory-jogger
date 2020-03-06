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

use actix_web::{middleware::Logger, web, App, HttpServer};
use anyhow::{Context, Result};
use env_logger::Env;
use listenfd::ListenFd;
use pocket_cleaner::{config::AppConfig, view};

static POCKET_CONSUMER_KEY_ENV_VAR: &str = "POCKET_CLEANER_CONSUMER_KEY";
static POCKET_USER_ACCESS_TOKEN: &str = "POCKET_TEMP_USER_ACCESS_TOKEN";

fn get_pocket_consumer_key() -> Result<String> {
    let key = POCKET_CONSUMER_KEY_ENV_VAR;
    let value = env::var(key).with_context(|| format!("missing app config env var: {}", key))?;
    Ok(value)
}

async fn try_main() -> Result<()> {
    env_logger::from_env(Env::default().default_filter_or("warn")).init();

    let port = env::var("PORT").context("PORT environment variable must be set")?;
    let port: i32 = port.parse().context("PORT must be a number")?;

    let pocket_consumer_key = get_pocket_consumer_key()?;
    let pocket_user_access_token = env::var(POCKET_USER_ACCESS_TOKEN)?;

    openssl_probe::init_ssl_cert_env_vars();
    let mut server = HttpServer::new(move || {
        App::new()
            .data(AppConfig {
                pocket_consumer_key: pocket_consumer_key.clone(),
                pocket_user_access_token: pocket_user_access_token.clone(),
            })
            .wrap(Logger::default())
            .service(
                web::scope("/api/v1")
                    .service(web::resource("/trends").route(web::get().to(view::trends_view))),
            )
    });

    let mut listenfd = ListenFd::from_env();
    server = if let Some(l) = listenfd.take_tcp_listener(0)? {
        server.listen(l)?
    } else {
        let addr = format!("0.0.0.0:{}", port);
        println!("Listening on http://{}", addr);
        server.bind(addr)?
    };

    server.run().await?;

    Ok(())
}

#[actix_rt::main]
async fn main() {
    if let Err(e) = try_main().await {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
