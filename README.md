# Memory Jogger

![Memory Jogger CI](https://github.com/rgardner/memory-jogger/workflows/Pocket%20Cleaner%20CI/badge.svg)

Finds items from your [Pocket](https://getpocket.com/) library that are
relevant to trending news. I have thousands of unread Pocket items and Memory
Jogger enables me to find new meaning in articles and videos I saved years
ago. I deployed Memory Jogger to [Heroku](https://www.heroku.com/) and set up
a daily job to email me unread Pocket items based on [Google
Trends][google-trends] results from the past two days.

## Features

- Uses [Google Trends][google-trends] to find trending news
  and [tf-idf](https://en.wikipedia.org/wiki/Tf%E2%80%93idf) to rank unread
  Pocket items
- Can be run locally (typically using SQLite backend)
- Can be run in the cloud (typically using the PostgreSQL backend)
- Can be configured to send emails (via [SendGrid](https://sendgrid.com/))

```sh
# View relevant Pocket items based on Google Trends
$ memory_jogger relevant --dry-run
How Lin-Manuel Miranda taught liberals to love Alexander Hamilton (https://app.getpocket.com/read/1116619900), Why: Hamilton (https://trends.google.com/trends/explore?q=Hamilton&date=now%207-d&geo=US)
Canada Cuts Down On Red Tape. Could It Work In The U.S.? (https://app.getpocket.com/read/934754123), Why: Canada Day (https://trends.google.com/trends/explore?q=Canada%20Day&date=now%207-d&geo=US)
Pokemon Sun / Moon QR Codes (https://app.getpocket.com/read/1476660543, Why: Pokemon Unite (https://trends.google.com/trends/explore?q=Pokemon%20Unite&date=now%207-d&geo=US)
# View Google Trends
$ memory_jogger trends
Hamilton
Canada Day
Pokemon Unite
# View full help output
$ memory_jogger --help
memory_jogger 2.0.0
Interacts with Pocket Cleaner DB and APIs.

USAGE:
    memory_jogger --database-url <database-url> <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --database-url <database-url>     [env: DATABASE_URL=mj.db]

SUBCOMMANDS:
    db             Retrieve items from the database
    help           Prints this message or the help of the given subcommand(s)
    pocket         Interact with Pocket
    relevant       View relevant Pocket items for latest trends
    saved-items    Sync and search saved items
    trends         View latest trends
```

## Getting Started

By default, Memory Jogger is compiled with both SQLite and PostgreSQL support
and chooses the backend based on the `--database-url` argument (or
`DATABASE_URL` environment variable). To compile only one backend:

- SQLite-only: `cargo build --no-default-features --features sqlite`
- PostgreSQL-only: `cargo build --no-default-features --features postgres`

### Local Setup

```sh
# Build
cargo build --release
target/release/memory-jogger db user add \
  --email <your_email> \
  --pocket-access-token <your_pocket_access_token>
```

### Cloud Setup

See [docs/heroku.md](docs/heroku.md) for instructions on deploying Memory
Jogger to Heroku.

## Contributing

Memory Jogger uses [Invoke][pyinvoke] to manage build task execution.

Install Python 3.8+ and [Invoke][pyinvoke].

To run in a Docker container, run:

```sh
invoke run --docker
```

[pyinvoke]: https://www.pyinvoke.org/

### References

- [Google Trends][google-trends]
  - [Unofficial JS Reference Client Library](https://github.com/pat310/google-trends-api)
- [Pocket](https://getpocket.com/)
  - [Pocket Developer Homepage](https://getpocket.com/developer/)
  - [Pocket Authentication API](https://getpocket.com/developer/docs/authentication)
  - [Pocket Retrieve API](https://getpocket.com/developer/docs/v3/retrieve)
- [SendGrid](https://sendgrid.com/)
  - [SendGrid v3 Web API](https://sendgrid.com/docs/API_Reference/api_v3.html)
  - [SendGrid Send Mail API](https://sendgrid.com/docs/API_Reference/Web_API_v3/Mail/index.html)

[google-trends]: https://trends.google.com/trends/
