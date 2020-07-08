# Memory Jogger Database Usage

Memory Jogger supports SQLite and PostgreSQL as database backends via the
[Diesel](https://diesel.rs/) ORM. Diesel is typically used with one backend, so
additional flags need to be passed to the `diesel` CLI to specify which backend
should be configured or migrated.

To use SQLite:

- Set `DIESEL_CONFIG_FILE` environment variable to `diesel_sqlite.toml`
- Set `DATABASE_URL` environment variable to `<path/to/sqlite_database.db>`
- Pass `--migration-dir migrations/sqlite` to all `diesel` CLI commands
  P

To use PostgreSQL:

- Set `DIESEL_CONFIG_FILE` environment variable to `diesel_postgres.toml`
- Set `DATABASE_URL` environment variable to `<postgres_db_connection>`
- Pass `--migration-dir migrations/postgres` to all `diesel` CLI commands
