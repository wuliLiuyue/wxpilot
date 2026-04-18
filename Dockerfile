FROM rust:1.87-bookworm AS builder

WORKDIR /app
COPY rust /app/rust

WORKDIR /app/rust
RUN cargo build --release -p wxp-license-server --bin wxp-license-server

FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl sqlite3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/rust/target/release/wxp-license-server /usr/local/bin/wxp-license-server

WORKDIR /srv/wxp-license-server

CMD ["wxp-license-server"]
