#!/usr/bin/env bash
# Cross-compiles the W0 spike for Android with the NDK's clang and prints the
# evidence the gate report quotes (ELF arch, NEEDED libs, page alignment,
# SQLite symbols).
#
#   ANDROID_NDK_HOME=/path/to/android-ndk-r30 scripts/build-android.sh [API] [TARGET...]
#   API defaults to 36 (Android 16); targets default to aarch64 and x86_64.
set -euo pipefail
cd "$(dirname "$0")/.."

: "${ANDROID_NDK_HOME:?set ANDROID_NDK_HOME to an NDK r28+ install}"
API="${1:-36}"
shift || true
TARGETS=("$@")
[ ${#TARGETS[@]} -eq 0 ] && TARGETS=(aarch64-linux-android x86_64-linux-android)

BIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin"
export RUSTC_WRAPPER="" CARGO_ENCODED_RUSTFLAGS=""

for target in "${TARGETS[@]}"; do
  cc="$BIN/${target}${API}-clang"
  [ -x "$cc" ] || { echo "missing $cc (NDK too old for API $API?)" >&2; exit 1; }
  env_target="${target//-/_}"
  upper="$(echo "$env_target" | tr '[:lower:]' '[:upper:]')"
  export "CC_${env_target}=$cc" "AR_${env_target}=$BIN/llvm-ar"
  export "CARGO_TARGET_${upper}_LINKER=$cc"
  export CARGO_TARGET_DIR="target/api${API}"

  cargo build --locked --release --target "$target"

  out="target/api${API}/${target}/release"
  so="$out/libcapsnap_store.so"
  echo "== ${target} API ${API}"
  file "$so" "$out/selftest" | sed "s|$PWD/||"
  echo "size: $(stat -c %s "$so") bytes"
  echo "NEEDED: $("$BIN/llvm-readelf" -d "$so" | awk '/NEEDED/{print $NF}' | tr -d '[]' | tr '\n' ' ')"
  echo "LOAD alignment: $("$BIN/llvm-readelf" -lW "$so" | awk '$1=="LOAD"{print $NF}' | sort -u | tr '\n' ' ')"
  echo "exported: $("$BIN/llvm-nm" -D --defined-only "$so" | awk '/capsnap_spike_selftest/{print $NF}')"
  # Release builds are stripped (repo-root profile), so look for SQLite's
  # embedded source-id string rather than symbols.
  echo "embedded SQLite source id: $(grep -a -m1 -oE '[0-9]{4}-[0-9]{2}-[0-9]{2} [0-9:]{8} [0-9a-f]{64}' "$so" || echo MISSING)"
done
