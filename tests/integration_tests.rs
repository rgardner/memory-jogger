use std::{env, ffi::OsString};

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

static DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";
static POCKET_CONSUMER_KEY_ENV_VAR: &str = "MEMORY_JOGGER_POCKET_CONSUMER_KEY";

lazy_static::lazy_static! {
    static ref BIN_UNDER_TEST: escargot::CargoRun = escargot::CargoBuild::new()
        .bin("memory_jogger")
        .current_release()
        .features("large_tests")
        .run()
        .expect("failed to create `cargo run` command");
}

struct TestContext {
    pocket_consumer_key: String,
    pocket_user_access_token: String,
    database_url: OsString,
}

impl TestContext {
    /// Creates a test context from environment variables.
    fn new(database_url: OsString) -> Self {
        Self {
            pocket_consumer_key: env::var("MEMORY_JOGGER_TEST_POCKET_CONSUMER_KEY").unwrap(),
            pocket_user_access_token: env::var("MEMORY_JOGGER_TEST_POCKET_USER_ACCESS_TOKEN")
                .unwrap(),
            database_url,
        }
    }
}

/// Creates user in database with valid Pocket credentials.
fn create_user(context: &TestContext) {
    BIN_UNDER_TEST
        .command()
        .args(&[
            "db",
            "user",
            "add",
            "--email",
            "test.user@example.com",
            "--pocket-access-token",
            &context.pocket_user_access_token,
        ])
        .env(DATABASE_URL_ENV_VAR, &context.database_url)
        .unwrap()
        .assert()
        .success();
}

#[test]
#[cfg_attr(not(feature = "large_tests"), ignore)]
fn test_sqlite_relevant_items_succeeds_and_displays_output() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let sqlite_db = temp_dir.child("memory_jogger.db");
    let context = TestContext::new(sqlite_db.path().into());
    create_user(&context);

    BIN_UNDER_TEST
        .command()
        .arg("relevant")
        .env(DATABASE_URL_ENV_VAR, &context.database_url)
        .env(POCKET_CONSUMER_KEY_ENV_VAR, &context.pocket_consumer_key)
        .unwrap()
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Your Pocket journey starts now. Make the most of it.",
        ));
}

#[test]
#[cfg_attr(not(feature = "large_tests"), ignore)]
fn test_sqlite_saved_items_sync_and_search_returns_results() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let sqlite_db = temp_dir.child("memory_jogger.db");
    let context = TestContext::new(sqlite_db.path().into());
    create_user(&context);

    let user_id = "1";
    BIN_UNDER_TEST
        .command()
        .args(&["saved-items", "sync", "--user-id", user_id])
        .env(DATABASE_URL_ENV_VAR, &context.database_url)
        .env(POCKET_CONSUMER_KEY_ENV_VAR, &context.pocket_consumer_key)
        .unwrap()
        .assert()
        .success();

    BIN_UNDER_TEST
        .command()
        .args(&["saved-items", "search", "Pocket", "--user-id", user_id])
        .env(DATABASE_URL_ENV_VAR, sqlite_db.path())
        .unwrap()
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Your Pocket journey starts now. Make the most of it.",
        ));
}

#[test]
#[cfg_attr(not(feature = "large_tests"), ignore)]
fn test_trends_subcommand_succeeds_and_displays_output() {
    BIN_UNDER_TEST
        .command()
        .args(&["trends"])
        .env(DATABASE_URL_ENV_VAR, "FAKE_DATABASE_URL")
        .unwrap()
        .assert()
        .success()
        .stdout(predicates::str::is_empty().not());
}
