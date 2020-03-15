# You can override this `--build-arg BASE_IMAGE=...` to use different
# version of Rust or OpenSSL.
ARG BASE_IMAGE=rust:1.42.0-buster

# Our first FROM statement declares the build environment.
FROM ${BASE_IMAGE} AS builder

RUN USER=rust cargo new --bin /usr/src/pocket_cleaner
WORKDIR /usr/src/pocket_cleaner

# Create empty run binary specified by 'default-run' in Cargo.toml, needed by
# cargo build
RUN mkdir src/bin
RUN echo "fn main() {}" >./src/bin/pc_server.rs

COPY ./Cargo.toml ./Cargo.lock ./
RUN cargo build --release \
        && rm -f target/release/deps/pc_server* \
        && rm -r src

COPY ./migrations ./migrations
COPY ./src ./src
RUN cargo build --release

FROM debian:buster-slim
RUN apt-get update && apt-get install --yes --no-install-recommends \
        ca-certificates=20190110 \
        libpq5=11.7-0+deb10u1 \
        && rm -rf /var/lib/apt/lists/*

COPY --from=builder \
        /usr/src/pocket_cleaner/target/release/pc_console \
        /usr/src/pocket_cleaner/target/release/pc_send_digest_email \
        /usr/src/pocket_cleaner/target/release/pc_server \
        /usr/local/bin/

CMD ["/usr/local/bin/pc_server"]
