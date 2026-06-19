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