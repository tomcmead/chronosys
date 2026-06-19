# Planner: create recipe of dependencies
FROM lukemathwalker/cargo-chef:latest-rust-1.85-slim AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Dev Cacher: pre-build external dependencies in DEBUG mode
FROM lukemathwalker/cargo-chef:latest-rust-1.85-slim AS dev-cacher
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json

# Dev Target: final target for local coding. Uses cargo-watch for hot-reloads.
FROM lukemathwalker/cargo-chef:latest-rust-1.85-slim AS development
WORKDIR /app
RUN cargo install --locked cargo-watch
# Pull pre-compiled debug dependencies
COPY --from=dev-cacher /app/target target
COPY --from=dev-cacher /usr/local/cargo /usr/local/cargo
COPY . .
CMD ["cargo", "watch", "-x", "run"]

# Release Cacher: pre-build external dependencies in RELEASE mode
FROM lukemathwalker/cargo-chef:latest-rust-1.85-slim AS release-cacher
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Release Builder: compiles application code on top of cached release dependencies
FROM lukemathwalker/cargo-chef:latest-rust-1.85-slim AS builder
WORKDIR /app
COPY . .
COPY --from=release-cacher /app/target target
COPY --from=release-cacher /usr/local/cargo /usr/local/cargo
RUN cargo build --release

# Release Target: copies just the binary into a stripped-down Debian environment
FROM debian:bookworm-slim AS release
WORKDIR /usr/local/bin
COPY --from=builder /app/target/release/chronosys .
CMD ["./chronosys"]