# Planner: create recipe of dependencies
FROM lukemathwalker/cargo-chef:0.1.77-rust-1.93.1-slim-trixie AS planner
WORKDIR /app
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Toolchain base: everything needed to build the eBPF crate, shared by both cachers
FROM lukemathwalker/cargo-chef:0.1.77-rust-1.93.1-slim-trixie AS toolchain-base
RUN apt-get update && apt-get install -y --no-install-recommends \
    curl \
    clang \
    llvm \
    libelf-dev \
    pkg-config \
    git \
    && rm -rf /var/lib/apt/lists/*
RUN rustup toolchain install nightly --component rust-src
RUN cargo +nightly install bpf-linker

# Dev Cacher: pre-build external dependencies in DEBUG mode
FROM toolchain-base AS dev-cacher
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --recipe-path recipe.json

# Dev Target: final target for local coding. Uses cargo-watch for hot-reloads.
FROM toolchain-base AS development
WORKDIR /app
RUN cargo install --locked cargo-watch
RUN rustup component add rustfmt clippy
# Pull pre-compiled debug dependencies
COPY --from=dev-cacher /app/target target
COPY --from=dev-cacher /usr/local/cargo /usr/local/cargo
COPY . .
CMD ["cargo", "watch", "-x", "run"]

# Release Cacher: pre-build external dependencies in RELEASE mode
FROM toolchain-base AS release-cacher
WORKDIR /app
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Release Builder: compiles application code on top of cached release dependencies
FROM toolchain-base AS builder
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