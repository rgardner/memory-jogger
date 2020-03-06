# You can override this `--build-arg BASE_IMAGE=...` to use different
# version of Rust or OpenSSL.
ARG BASE_IMAGE=ekidd/rust-musl-builder:latest

# Our first FROM statement declares the build environment.
FROM ${BASE_IMAGE} AS builder

RUN USER=root cargo new --bin /home/rust/src/pocket_cleaner
WORKDIR /home/rust/src/pocket_cleaner

# Touch default run binary (needed by cargo build)
RUN mkdir src/bin
RUN echo "fn main() {}" >./src/bin/server.rs

COPY ./Cargo.toml ./Cargo.lock ./
RUN RUSTFLAGS=-Clinker=musl-gcc cargo build \
        --release \
        --features vendored \
        && rm -f target/x86_64-unknown-linux-musl/release/deps/pocket_cleaner* \
        && rm -f target/x86_64-unknown-linux-musl/release/deps/server* \
        && rm -r src

COPY ./src ./src
RUN RUSTFLAGS=-Clinker=musl-gcc cargo build \
        --release \
        --features vendored

FROM alpine:3.11.3
RUN apk --no-cache add ca-certificates
COPY --from=builder \
        /home/rust/src/pocket_cleaner/target/x86_64-unknown-linux-musl/release/server \
        /home/rust/src/pocket_cleaner/target/x86_64-unknown-linux-musl/release/send_digest_email \
        ./

# Run the image as a non-root user
RUN adduser -D webuser
USER webuser

CMD ["./server"]
