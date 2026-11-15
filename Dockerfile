# syntax=docker/dockerfile:1.7
# Ferrokit: Svelte UI embedded in one Rust binary, on Alpine with qpdf.

# 1. Web UI
FROM node:22-alpine AS web
WORKDIR /web
COPY web/package.json web/package-lock.json ./
RUN --mount=type=cache,target=/root/.npm npm ci --no-audit --no-fund
COPY web/ ./
RUN npm run build

# 2. Rust server (static musl binary with the UI embedded)
FROM rust:1-alpine AS server
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
COPY --from=web /web/build web/build
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p ferrokit-server \
 && cp target/release/ferrokit /ferrokit

# 3. Runtime
FROM alpine:3.22
RUN apk add --no-cache qpdf tini \
 && adduser -D -H -u 10001 ferrokit
COPY --from=server /ferrokit /usr/local/bin/ferrokit
USER ferrokit
ENV FERROKIT_BIND=0.0.0.0 FERROKIT_PORT=8080 TMPDIR=/tmp
EXPOSE 8080
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s \
  CMD wget -qO- http://127.0.0.1:8080/api/health >/dev/null || exit 1
ENTRYPOINT ["/sbin/tini", "--", "/usr/local/bin/ferrokit"]
