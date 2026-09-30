#!/usr/bin/env bash
# Host tests for the W0 spike. The repo-root .cargo/config.toml (HMS Forge)
# wants sccache + mold; this spike needs neither, so neutralise both.
set -euo pipefail
cd "$(dirname "$0")/.."
export RUSTC_WRAPPER="" CARGO_ENCODED_RUSTFLAGS=""
cargo test --locked "$@"
