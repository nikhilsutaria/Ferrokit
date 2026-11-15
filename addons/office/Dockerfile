# syntax=docker/dockerfile:1.7
# Ferrokit Office add-on: LibreOffice (kept warm by unoserver), Chromium and Ghostscript,
# driven by the ferrokit-worker binary. Build from the repo root:
#   docker compose --profile office build

# 1. The worker (static musl binary)
FROM rust:1-alpine AS worker
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY Cargo.toml Cargo.lock ./
COPY crates crates
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p ferrokit-worker \
 && cp target/release/ferrokit-worker /ferrokit-worker

# 2. Runtime
FROM debian:trixie-slim
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
      libreoffice-writer-nogui libreoffice-calc-nogui libreoffice-impress-nogui libreoffice-draw-nogui \
      python3-uno python3-pip \
      chromium ghostscript \
      fonts-liberation fonts-dejavu-core fonts-crosextra-carlito fonts-crosextra-caladea \
      fonts-noto-core fonts-noto-cjk \
      tini wget ca-certificates \
 && pip install --no-cache-dir --break-system-packages --no-deps unoserver==3.7 \
 && apt-get purge -y python3-pip && apt-get autoremove -y \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --uid 10001 --no-create-home --shell /usr/sbin/nologin ferrokit
COPY crates/ferrokit-worker/assets /usr/local/share/ferrokit
COPY --from=worker /ferrokit-worker /usr/local/bin/ferrokit-worker
USER ferrokit
ENV TMPDIR=/tmp HOME=/tmp FERROKIT_WORKER_PORT=8090 FERROKIT_ASSETS=/usr/local/share/ferrokit
EXPOSE 8090
HEALTHCHECK --interval=30s --timeout=3s --start-period=30s \
  CMD wget -qO- http://127.0.0.1:8090/health >/dev/null || exit 1
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/ferrokit-worker"]
