FROM lukemathwalker/cargo-chef:latest-rust-1-alpine3.20 AS chef
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --bin eh_bot --recipe-path recipe.json

FROM chef AS builder

# nodejs, npm, bash for archive viewer. perl, make for vendored openssl
RUN apk add --no-cache nodejs npm bash perl make

COPY --from=planner /app/recipe.json recipe.json

# Build dependencies
RUN cargo chef cook  --bin eh_bot --release --recipe-path recipe.json

# Build application
COPY . .
RUN cargo build --release --bin eh_bot

FROM alpine:3.20 AS runtime
WORKDIR /app
COPY --from=builder /app/target/release/eh_bot /usr/local/bin
ENTRYPOINT ["/usr/local/bin/eh_bot"]