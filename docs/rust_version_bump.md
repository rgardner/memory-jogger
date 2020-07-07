# Updating Rust Version

The Rust version used for production and CI are maintained in two different
places:

- Production: [Dockerfile](../Dockerfile)
  - Update `BASE_IMAGE` value to target version. See [rust - Docker
    Hub](https://hub.docker.com/_/rust) for supported versions.
- CI: [pocket_cleaner.ci.yml](../.github/workflows/pocket_cleaner.ci.yml)
  - Update all `actions-rs/toolchain` step `toolchain` values to the target
    version
