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

use actix_web::{middleware::Logger, web, App, HttpServer};
use env_logger::Env;
use listenfd::ListenFd;
use pocket_cleaner::{
    config::{self, get_required_env_var, AppConfig},
    error::{PocketCleanerError, Result},
    view,
};

async fn try_main() -> Result<()> {
    env_logger::from_env(Env::default().default_filter_or("warn")).init();

    let port = get_required_env_var("PORT")?;
    let port: i32 = port
        .parse()
        .map_err(|e| PocketCleanerError::Unknown(format!("PORT must be a number: {}", e)))?;

    let pocket_consumer_key = get_required_env_var(config::POCKET_CONSUMER_KEY_ENV_VAR)?;
    let pocket_user_access_token = get_required_env_var(config::POCKET_USER_ACCESS_TOKEN_ENV_VAR)?;

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
