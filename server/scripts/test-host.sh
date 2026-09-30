#!/usr/bin/env bash
# Host tests for capsnap-server. The repo-root .cargo/config.toml (HMS Forge)
# wants sccache + mold; this crate needs neither, so neutralise both.
set -euo pipefail
cd "$(dirname "$0")/.."
export RUSTC_WRAPPER="" CARGO_ENCODED_RUSTFLAGS=""
cargo test --locked "$@"
