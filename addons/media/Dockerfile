# syntax=docker/dockerfile:1.7
# Ferrokit media add-on: ffmpeg for the video and audio tools, driven by ferrokit-worker.
# Build from the repo root:
#   docker compose --profile media build

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
      ffmpeg \
      fonts-dejavu-core fonts-liberation fonts-noto-core fonts-noto-cjk \
      tini wget \
 && rm -rf /var/lib/apt/lists/* \
 && useradd --uid 10001 --no-create-home --shell /usr/sbin/nologin ferrokit \
 && mkdir /jobs && chown 10001 /jobs
# Fail the build if any encoder or filter a tool relies on is missing from this ffmpeg.
RUN set -e; \
    enc="$(ffmpeg -hide_banner -encoders)"; \
    for e in libx264 libx265 libvpx-vp9 libopus libmp3lame libvorbis; do \
      echo "$enc" | grep -qw "$e" || { echo "ffmpeg is missing the $e encoder" >&2; exit 1; }; \
    done; \
    flt="$(ffmpeg -hide_banner -filters)"; \
    for f in vidstabdetect vidstabtransform rubberband subtitles ass zscale tonemap showwavespic loudnorm afftdn silenceremove palettegen paletteuse; do \
      echo "$flt" | grep -qw "$f" || { echo "ffmpeg is missing the $f filter" >&2; exit 1; }; \
    done
COPY --from=worker /ferrokit-worker /usr/local/bin/ferrokit-worker
USER ferrokit
ENV TMPDIR=/tmp HOME=/tmp FERROKIT_WORKER_PORT=8090 FERROKIT_MEDIA_DIR=/jobs
EXPOSE 8090
HEALTHCHECK --interval=30s --timeout=3s --start-period=10s \
  CMD wget -qO- http://127.0.0.1:8090/health >/dev/null || exit 1
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/ferrokit-worker"]
