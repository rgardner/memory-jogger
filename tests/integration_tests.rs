use std::{env, ffi::OsString, sync::Mutex};

use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;

static DATABASE_URL_ENV_VAR: &str = "DATABASE_URL";
static POCKET_CONSUMER_KEY_ENV_VAR: &str = "MEMORY_JOGGER_POCKET_CONSUMER_KEY";

lazy_static::lazy_static! {
    /// Any test that accesses a Postgres database must grab this mutex.
    static ref PG_MUTEX: Mutex<()> = Mutex::new(());
    static ref BIN_UNDER_TEST: escargot::CargoRun = { let mut cmd = escargot::CargoBuild::new()
        .bin("memory_jogger")
        .current_release();
        cfg_if::cfg_if! {
            if #[cfg(not(all(feature = "postgres", feature = "sqlite")))] {
                cmd = cmd.no_default_features();
            }
        }
        cfg_if::cfg_if! {
            if #[cfg(all(feature = "postgres", feature = "sqlite"))] {
                cmd = cmd.features("large_tests");
            } else if #[cfg(feature = "postgres")] {
                cmd = cmd.features("large_tests postgres");
            } else if #[cfg(feature = "sqlite")] {
                cmd = cmd.features("large_tests sqlite");
            } else {
                compile_error!("postgres and/or sqlite feature must be enabled");
            }
        }

        cmd.run().expect("failed to create `cargo run` command") };
}

struct TestContext {
    pocket_consumer_key: String,
    pocket_user_access_token: String,
    database_url: OsString,
}

impl TestContext {
    /// Creates a Postgres test context.
    fn postgres() -> Self {
        Self {
            pocket_consumer_key: env::var("MEMORY_JOGGER_TEST_POCKET_CONSUMER_KEY").unwrap(),
            pocket_user_access_token: env::var("MEMORY_JOGGER_TEST_POCKET_USER_ACCESS_TOKEN")
                .unwrap(),
            database_url: env::var_os("PG_DATABASE_URL").unwrap(),
        }
    }

    /// Creates a test context from environment variables.
    fn sqlite(database_url: OsString) -> Self {
        Self {
            pocket_consumer_key: env::var("MEMORY_JOGGER_TEST_POCKET_CONSUMER_KEY").unwrap(),
            pocket_user_access_token: env::var("MEMORY_JOGGER_TEST_POCKET_USER_ACCESS_TOKEN")
                .unwrap(),
            database_url,
        }
    }
}

struct UserId(usize);

/// Creates user in database with valid Pocket credentials.
fn create_user(context: &TestContext) -> UserId {
    let output = BIN_UNDER_TEST
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
        .output()
        .unwrap();
    let output = String::from_utf8(output.stdout).unwrap().trim().to_string();

    let id_start = output.find("id: ").unwrap() + 4;
    let user_id = output[id_start..].parse::<usize>().unwrap();
    UserId(user_id)
}

#[test]
#[cfg_attr(not(all(feature = "large_tests", feature = "postgres")), ignore)]
fn test_postgres_relevant_items_succeeds_and_displays_output() {
    let context = TestContext::postgres();
    let _m = PG_MUTEX.lock().expect("Mutex got poisoned by another test");
    let user_id = create_user(&context).0.to_string();

    BIN_UNDER_TEST
        .command()
        .args(&["relevant", "--user-id", &user_id])
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
#[cfg_attr(not(all(feature = "large_tests", feature = "postgres")), ignore)]
fn test_postgres_saved_items_sync_and_search_returns_results() {
    let context = TestContext::postgres();
    let _m = PG_MUTEX.lock().expect("Mutex got poisoned by another test");
    let user_id = create_user(&context).0.to_string();

    BIN_UNDER_TEST
        .command()
        .args(&["saved-items", "sync", "--user-id", &user_id])
        .env(DATABASE_URL_ENV_VAR, &context.database_url)
        .env(POCKET_CONSUMER_KEY_ENV_VAR, &context.pocket_consumer_key)
        .unwrap()
        .assert()
        .success();

    BIN_UNDER_TEST
        .command()
        .args(&["saved-items", "search", "Pocket", "--user-id", &user_id])
        .env(DATABASE_URL_ENV_VAR, &context.database_url)
        .unwrap()
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "Your Pocket journey starts now. Make the most of it.",
        ));
}

#[test]
#[cfg_attr(not(all(feature = "large_tests", feature = "sqlite")), ignore)]
fn test_sqlite_relevant_items_succeeds_and_displays_output() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let sqlite_db = temp_dir.child("memory_jogger.db");
    let context = TestContext::sqlite(sqlite_db.path().into());
    let user_id = create_user(&context).0.to_string();

    BIN_UNDER_TEST
        .command()
        .args(&["relevant", "--user-id", &user_id])
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
#[cfg_attr(not(all(feature = "large_tests", feature = "sqlite")), ignore)]
fn test_sqlite_saved_items_sync_and_search_returns_results() {
    let temp_dir = assert_fs::TempDir::new().unwrap();
    let sqlite_db = temp_dir.child("memory_jogger.db");
    let context = TestContext::sqlite(sqlite_db.path().into());
    let user_id = create_user(&context).0.to_string();

    BIN_UNDER_TEST
        .command()
        .args(&["saved-items", "sync", "--user-id", &user_id])
        .env(DATABASE_URL_ENV_VAR, &context.database_url)
        .env(POCKET_CONSUMER_KEY_ENV_VAR, &context.pocket_consumer_key)
        .unwrap()
        .assert()
        .success();

    BIN_UNDER_TEST
        .command()
        .args(&["saved-items", "search", "Pocket", "--user-id", &user_id])
        .env(DATABASE_URL_ENV_VAR, &context.database_url)
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
