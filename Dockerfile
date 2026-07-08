FROM node:22-alpine AS frontend

WORKDIR /app/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ .
ENV NEXT_PUBLIC_BASE_PATH=/__HAPPYVIEW_BP__
RUN npm run build

FROM rust:1.96.1-bookworm AS builder

WORKDIR /app

# Build dependencies first (cached until Cargo.toml/Cargo.lock change)
COPY Cargo.toml Cargo.lock ./
RUN mkdir -p src/bin && echo "fn main() {}" > src/main.rs && touch src/lib.rs && echo "fn main() {}" > src/bin/migrate_lua_sql.rs
ENV SQLX_OFFLINE=true
RUN cargo build --release && rm -rf src target/release/.fingerprint/happyview-*

# Build application code
COPY src/ src/
COPY migrations/ migrations/
ARG HAPPYVIEW_VERSION
ENV HAPPYVIEW_VERSION=$HAPPYVIEW_VERSION
RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    gosu \
    && rm -rf /var/lib/apt/lists/*

# The service runs as a non-root system user (uid/gid 10001); the binary needs no
# root privileges at runtime and binds an unprivileged port (3000). The container
# starts as root only so the entrypoint can fix ownership of a mounted data volume
# (e.g. a SQLite volume carried over from a root-era install) before dropping to
# the app user via gosu.
RUN groupadd --system --gid 10001 app \
    && useradd --system --uid 10001 --gid app --home-dir /app --no-create-home \
       --shell /usr/sbin/nologin app

WORKDIR /app

COPY --from=builder /app/target/release/happyview /usr/local/bin/happyview
RUN chmod +x /usr/local/bin/happyview
COPY migrations/ /app/migrations
# The static dir is writable at runtime: the entrypoint rewrites the base-path
# sentinel in place and removes the marker.
COPY --from=frontend --chown=app:app /app/web/out /srv/static
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh && touch /srv/static/.base-path-pending

# Data dir for the default SQLite backend (DATABASE_URL=sqlite://data/...).
# The entrypoint re-chowns this to the app user at startup, so a volume mounted
# here — including one created by an older root-era install — becomes writable.
RUN mkdir -p /app/data && chown app:app /app/data

ENV STATIC_DIR=/srv/static

EXPOSE 3000

# Starts as root; entrypoint chowns /app/data and drops to the app user via gosu.
ENTRYPOINT ["/entrypoint.sh"]
