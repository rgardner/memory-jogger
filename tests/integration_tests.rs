use std::env;

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

static DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";
static TEST_USER_EMAIL: &str = "fake.email@example.com";

lazy_static::lazy_static! {
    static ref BIN_UNDER_TEST: escargot::CargoRun = escargot::CargoBuild::new()
        .bin("memory_jogger")
        .current_release()
        .features("large_tests")
        .run()
        .expect("failed to create `cargo run` command");
}

#[test]
#[cfg_attr(not(feature = "large_tests"), ignore)]
fn test_sqlite_relevant_items_succeeds_and_displays_output() {
    let pocket_consumer_key = env::var("MEMORY_JOGGER_TEST_POCKET_CONSUMER_KEY").unwrap();
    let pocket_access_token = env::var("MEMORY_JOGGER_TEST_POCKET_USER_ACCESS_TOKEN").unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let sqlite_db = temp_dir.child("memory_jogger.db");
    BIN_UNDER_TEST
        .command()
        .args(&[
            "db",
            "user",
            "add",
            "--email",
            TEST_USER_EMAIL,
            "--pocket-access-token",
            &pocket_access_token,
        ])
        .env(DATABASE_URL_ENV_VAR, sqlite_db.path())
        .env("MEMORY_JOGGER_POCKET_CONSUMER_KEY", &pocket_consumer_key)
        .unwrap()
        .assert()
        .success();

    BIN_UNDER_TEST
        .command()
        .arg("relevant")
        .env(DATABASE_URL_ENV_VAR, sqlite_db.path())
        .env("MEMORY_JOGGER_POCKET_CONSUMER_KEY", &pocket_consumer_key)
        .unwrap()
        .assert()
        .success()
        .stdout(predicates::str::is_empty().not());
}

#[test]
#[cfg_attr(not(feature = "large_tests"), ignore)]
fn test_sqlite_saved_items_sync_and_search_returns_results() {
    let pocket_consumer_key = env::var("MEMORY_JOGGER_TEST_POCKET_CONSUMER_KEY").unwrap();
    let pocket_access_token = env::var("MEMORY_JOGGER_TEST_POCKET_USER_ACCESS_TOKEN").unwrap();
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let sqlite_db = temp_dir.child("memory_jogger.db");
    BIN_UNDER_TEST
        .command()
        .args(&[
            "db",
            "user",
            "add",
            "--email",
            TEST_USER_EMAIL,
            "--pocket-access-token",
            &pocket_access_token,
        ])
        .env(DATABASE_URL_ENV_VAR, sqlite_db.path())
        .env("MEMORY_JOGGER_POCKET_CONSUMER_KEY", &pocket_consumer_key)
        .unwrap()
        .assert()
        .success();

    let user_id = "1";
    BIN_UNDER_TEST
        .command()
        .args(&["saved-items", "sync", "--user-id", user_id])
        .env(DATABASE_URL_ENV_VAR, sqlite_db.path())
        .env("MEMORY_JOGGER_POCKET_CONSUMER_KEY", &pocket_consumer_key)
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
        .stdout(predicates::str::is_empty().not());
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
