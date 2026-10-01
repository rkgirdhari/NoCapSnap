#!/usr/bin/env bash
# Host tests for capsnap-sync. Clears any rustc wrapper or flags a parent or user cargo
# config may set (CapSnap began inside a repo whose root config wanted sccache + mold).
set -euo pipefail
cd "$(dirname "$0")/.."
export RUSTC_WRAPPER="" CARGO_ENCODED_RUSTFLAGS=""
cargo test --locked "$@"
