#!/bin/sh
# Run the Rust test-suite inside Docker (qpdf + Rust toolchain), caching deps in named volumes.
set -e
cd "$(dirname "$0")/.."
MSYS_NO_PATHCONV=1 docker run --rm \
  -v "$(pwd -W 2>/dev/null || pwd):/src" -w /src \
  -v ferrokit-cargo-registry:/usr/local/cargo/registry \
  -v ferrokit-target:/target -e CARGO_TARGET_DIR=/target \
  rust:1-alpine sh -c "apk add -q --no-cache qpdf musl-dev >/dev/null && cargo ${*:-test --workspace}"
