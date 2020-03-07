# You can override this `--build-arg BASE_IMAGE=...` to use different
# version of Rust or OpenSSL.
ARG BASE_IMAGE=ekidd/rust-musl-builder:latest

# Our first FROM statement declares the build environment.
FROM ${BASE_IMAGE} AS builder

RUN USER=rust:rust cargo new --bin /home/rust/src/pocket_cleaner
WORKDIR /home/rust/src/pocket_cleaner

# Create empty run binary specified by 'default-run' in Cargo.toml, needed by
# cargo build
RUN mkdir src/bin
RUN echo "fn main() {}" >./src/bin/pc_server.rs

COPY --chown=rust:rust ./Cargo.toml ./Cargo.lock ./
RUN cargo build --release --features vendored \
        && rm -f target/x86_64-unknown-linux-musl/release/deps/pc_console* \
        && rm -f target/x86_64-unknown-linux-musl/release/deps/pc_server* \
        && rm -r src

COPY --chown=rust:rust ./src ./src
RUN cargo build --release --features vendored

FROM alpine:3.11.3
RUN apk --no-cache add ca-certificates
COPY --from=builder \
        /home/rust/src/pocket_cleaner/target/x86_64-unknown-linux-musl/release/pc_server \
        /home/rust/src/pocket_cleaner/target/x86_64-unknown-linux-musl/release/pc_send_digest_email \
        /usr/local/bin/

# Run the image as a non-root user
RUN adduser -D webuser
USER webuser

CMD ["/usr/local/bin/server"]
