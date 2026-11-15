# syntax=docker/dockerfile:1.7
# Ferrokit OCR add-on: OCRmyPDF and Tesseract with every language, driven by ferrokit-worker.
# Build from the repo root:
#   docker compose --profile ocr build

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
      ocrmypdf tesseract-ocr-all unpaper pngquant \
      tini wget \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --uid 10001 --no-create-home --shell /usr/sbin/nologin ferrokit
COPY --from=worker /ferrokit-worker /usr/local/bin/ferrokit-worker
USER ferrokit
ENV TMPDIR=/tmp HOME=/tmp FERROKIT_WORKER_PORT=8090
EXPOSE 8090
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s \
  CMD wget -qO- http://127.0.0.1:8090/health >/dev/null || exit 1
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/ferrokit-worker"]
