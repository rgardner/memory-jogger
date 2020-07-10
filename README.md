# Memory Jogger

![Memory Jogger CI](https://github.com/rgardner/memory-jogger/workflows/Memory%20Jogger%20CI/badge.svg)

Finds items from your [Pocket][pocket] library that are
relevant to trending news. I have thousands of unread Pocket items and Memory
Jogger enables me to find new meaning in articles and videos I saved years
ago. I deployed Memory Jogger to [Heroku](https://www.heroku.com/) and set up
a daily job to email me unread Pocket items based on [Google
Trends][google-trends] results from the past two days. Memory Jogger is written
in [Rust][rust].

## Documentation Quick Links

- [Getting Started](#getting-started)
  - [Server Setup](#server-setup)
  - [Local Setup](#local-setup)
    - [Local Installation](#local-installation)
    - [Local Next Steps](#local-next-steps)
  - [Email Setup](#email-setup)
- [Contributing](#contributing)
- [Third Party API Documentation](#third-party-api-documentation)
- [License](#license)
- [Contribution](#contribution)

## Features

- Uses [Google Trends][google-trends] to find trending news
  and [tf-idf](https://en.wikipedia.org/wiki/Tf%E2%80%93idf) to rank unread
  Pocket items
- Can be run locally (typically using the SQLite backend)
- Can be run in the cloud (typically using the PostgreSQL backend)
- Can be configured to send emails (via [SendGrid][sendgrid])

```sh
# View relevant Pocket items based on Google Trends
$ memory_jogger relevant --user-id 1
[2020-07-09T17:23:39Z INFO  memory_jogger] finding trends
[2020-07-09T17:23:39Z INFO  memory_jogger] syncing database with Pocket
[2020-07-09T17:23:40Z INFO  memory_jogger] searching for relevant items
Trend Mary Kay Letourneau: https://trends.google.com/trends/explore?q=Mary+Kay+Letourneau&date=now+7-d&geo=US
        Hacker News Highlights, The Alan Kay Edition: https://app.getpocket.com/read/1310095698
        Excerpt - Japan\'s Decision for War in 1941: Some Enduring Lessons: https://app.getpocket.com/read/89684589
Trend Ninja: https://trends.google.com/trends/explore?q=Ninja&date=now+7-d&geo=US
        Full Spectrum Engineer Or Why The World Needs Polymaths: https://app.getpocket.com/read/350991133
Trend Roger Stone: https://trends.google.com/trends/explore?q=Roger+Stone&date=now+7-d&geo=US
        Roger Federer as Religious Experience: https://app.getpocket.com/read/1250394
        The world’s biggest telescope is ready. The problem: staffing it: https://app.getpocket.com/read/2374120153
# View Google Trends
$ memory_jogger trends
Hamilton
Canada Day
Pokemon Unite
# View full help output
$ memory_jogger --help
memory_jogger 2.0.0
Finds items from your Pocket library that are relevant to trending news.

USAGE:
    memory_jogger [FLAGS] --database-url <database-url> <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
        --trace      Shows trace messages, including potentially sensitive HTTP data
    -V, --version    Prints version information

OPTIONS:
        --database-url <database-url>     [env: DATABASE_URL=]

SUBCOMMANDS:
    completions    Generates shell completions
    db             Retrieves items from the database
    help           Prints this message or the help of the given subcommand(s)
    pocket         Interacts with Pocket
    relevant       Shows relevant Pocket items for latest trends
    saved-items    Syncs and searches saved items
    trends         Shows latest trends
```

## Getting Started

Memory Jogger can be installed and run locally or deployed to a server.

### Server Setup

See [docs/heroku.md](docs/heroku.md) for instructions on deploying Memory
Jogger to Heroku.

### Local Setup

#### Local Installation

By default, Memory Jogger requires
[`libpq`](https://www.postgresql.org/download/) and
[`sqlite`](https://www.sqlitetutorial.net/download-install-sqlite/). Once
these dependencies are installed, you can install Memory Jogger locally via
Rust's package manager, `cargo`:

```sh
cargo install --git https://github.com/rgardner/memory-jogger.git
```

To install with SQLite support only:

```sh
cargo install --git https://github.com/rgardner/memory-jogger.git --no-default-features --features sqlite
```

To install with PostgreSQL support only:

```
cargo install --git https://github.com/rgardner/memory-jogger.git --no-default-features --features postgres
```

#### Local Next Steps

Once Memory Jogger is installed, you need to setup the database, get a Pocket
user access token, and create the user in the database.

```sh
# For SQLite
export DATABASE_URL=<path/to/sqlite_db.db>
# For PostgreSQL
export DATABASE_URL=<postgres_connection_string>
```

With the `DATABASE_URL` environment variable set, Memory Jogger will create
and/or configure the database on start-up.

Next, obtain a [Pocket][pocket] app consumer key by creating an application
in their [Developer Portal](https://getpocket.com/developer/apps/):

- Permissions: Retrieve
- Platforms: Desktop (other)

Set the `MEMORY_JOGGER_POCKET_CONSUMER_KEY` environment variable to the
obtained consumer key.

Finally, create a user and set their Pocket access token.

```sh
$ memory_jogger pocket auth
Follow URL to authorize application: https://getpocket.com/auth/authorize?request_token=<redacted_request_token>&redirect_uri=memory_jogger%3Afinishauth
Press enter to continue

<redacted_user_access_token>
$ memory_jogger db user add --email <your_email> --pocket-access-token <redacted_user_access_token>
id: 1
```

With the required setup complete, try out Memory Jogger:

```sh
memory_jogger relevant --user-id <user_id, 1 above>
```

### Email Setup

Email setup is optional and typically used when running Memory Jogger on a
server. Memory Jogger uses [SendGrid][sendgrid] internally for sending
emails. Create an account on the [SendGrid][sendgrid] website and then set
the `MEMORY_JOGGER_SENDGRID_API_KEY` environment variable to your SendGrid API
key.

## Contributing

Memory Jogger is a typical [Rust][rust] application and can be built and tested
via `cargo` (e.g. `cargo build`, `cargo test`). Optionally, install
[Invoke][pyinvoke] for Python 3.8+ to run other custom builds tasks:

```sh
$ invoke --list
Available tasks:

  build   Builds Memory Jogger.
  clean   Removes built artifacts.
  fmt     Runs rustfmt on all source files.
  lint    Performs clippy on all source files.
  test    Runs all tests.
```

[Large](https://testing.googleblog.com/2010/12/test-sizes.html) tests are
disabled by default as they are slow and require [Pocket][pocket] test
credentials. Create a separate Pocket application in the [Pocket Developer
Portal](https://getpocket.com/developer/apps/):

- Permissions: Add, Modify, Retrieve
- Platforms: Desktop (other)

Then, create a test Pocket account and authorize the test application:

```sh
MEMORY_JOGGER_POCKET_CONSUMER_KEY=<test_pocket_consumer_key> memory_jogger pocket auth
```

Finally, set the environment variables and enable the `large_tests` Cargo
feature:

```sh
export MEMORY_JOGGER_TEST_POCKET_CONSUMER_KEY=<test_pocket_consumer_key>
export MEMORY_JOGGER_TEST_POCKET_USER_ACCESS_TOKEN=<test_pocket_user_access_token>
export PG_DATABASE_URL=<pg_database_connection_if_postgres_feature_enabled>
cargo test --features large_tests
```

[pyinvoke]: https://www.pyinvoke.org/

### Third Party API Documentation

- [Google Trends][google-trends]
  - [Unofficial JS Reference Client Library](https://github.com/pat310/google-trends-api)
- [Pocket](https://getpocket.com/)
  - [Pocket Developer Homepage](https://getpocket.com/developer/)
  - [Pocket Authentication API](https://getpocket.com/developer/docs/authentication)
  - [Pocket Retrieve API](https://getpocket.com/developer/docs/v3/retrieve)
- [SendGrid][sendgrid]
  - [SendGrid v3 Web API](https://sendgrid.com/docs/API_Reference/api_v3.html)
  - [SendGrid Send Mail API](https://sendgrid.com/docs/API_Reference/Web_API_v3/Mail/index.html)

## License

Licensed under either of

- Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT license
  ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.

[google-trends]: https://trends.google.com/trends/
[pocket]: https://getpocket.com/
[rust]: https://www.rust-lang.org/
[sendgrid]: https://sendgrid.com/
