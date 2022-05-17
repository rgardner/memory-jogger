# Memory Jogger Architecture

Memory Jogger syncs Pocket items to local storage and runs queries locally to
surface relevant content.

## Code Map

### `xtask`

The project's "build system." Follows the [xtask][xtask] conventions.

### `crates/pocket`

API wrapper for Pocket.

### `crates/pocket_sync`

### `crates/mj_repl`

[xtask]: https://github.com/matklad/cargo-xtask
