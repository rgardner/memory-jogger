# You can override this `--build-arg BASE_IMAGE=...` to use different
# version of Rust or OpenSSL.
ARG BASE_IMAGE=rust:1.51.0-buster

# Our first FROM statement declares the build environment.
# hadolint ignore=DL3006
FROM ${BASE_IMAGE} AS builder

RUN USER=rust cargo new --bin /usr/src/memory_jogger
WORKDIR /usr/src/memory_jogger

COPY ./Cargo.toml ./Cargo.lock ./
RUN cargo build --release --no-default-features --features "postgres" \
        && rm -f target/release/deps/memory_jogger* \
        && rm -r src

COPY ./migrations ./migrations
COPY ./src ./src
RUN cargo build --release --no-default-features --features "postgres"

FROM debian:buster-slim
RUN apt-get update && apt-get install --yes --no-install-recommends \
        ca-certificates=20200601~deb10u2 \
        libpq5=11.7-0+deb10u1 \
        && rm -rf /var/lib/apt/lists/*

COPY --from=builder \
        /usr/src/memory_jogger/target/release/memory_jogger \
        /usr/local/bin/
