# Logging

## Introduction

Memory Jogger uses the common Rust [`log`](https://crates.io/crates/log)
facade and [`env_logger`](https://crates.io/crates/env_logger)
implementation. Logs are written to stderr.

## Changing the Log Levels

To change the log levels locally, set the `RUST_LOG` variable, e.g.:

```sh
export RUST_LOG=memory_jogger=debug
```

To change them in production, update the `RUST_LOG` environment variable in
Heroku:

```sh
heroku config:set RUST_LOG=memory_jogger=debug
```

## Verbose Logging

Memory Jogger also supports logging at `trace` level (includes potentially
sensitive HTTP content) by running `memory_jogger` with `--trace`.

## Guidance on Log Levels

| Level | Notes                                                                                              |
| ----- | -------------------------------------------------------------------------------------------------- |
| error | customer severely impacted, human intervention asap                                                |
| warn  | customer probably impacted, no need to wake up in the middle of the night                          |
| info  | interesting runtime events, session lifecycle, boundary events, "business" errors (e.g. bad login) |
| debug | everything that doesn't make info cut, e.g. entry/exit of non-trivial functions                    |
| trace | extremely detailed, logging state during each iteration of the loop                                |

Based on [Stack Overflow post](https://stackoverflow.com/a/8021604/4228400).
