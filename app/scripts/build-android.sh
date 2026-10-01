#!/usr/bin/env bash
# Builds the CapSnap debug APK and prints the evidence the W1 gate report quotes.
#
#   ANDROID_HOME=/path/to/sdk NDK_HOME=/path/to/ndk-r30 JAVA_HOME=/path/to/jdk21 \
#     scripts/build-android.sh [aarch64|x86_64 ...]
#
# Needs: Android SDK (platform + build-tools 36.1+), NDK r28+ (16 KB pages by
# default), JDK 17+, Rust Android targets, pnpm (via corepack).
set -euo pipefail
cd "$(dirname "$0")/.."

: "${ANDROID_HOME:?set ANDROID_HOME to the Android SDK}"
: "${NDK_HOME:?set NDK_HOME to an NDK r28+ install}"
TARGETS=("$@")
[ ${#TARGETS[@]} -eq 0 ] && TARGETS=(aarch64)

# Ignore any rustc wrapper (e.g. sccache) a parent or user cargo config may set.
export RUSTC_WRAPPER=""
export CI=true

corepack pnpm install --frozen-lockfile
args=()
for t in "${TARGETS[@]}"; do args+=(--target "$t"); done
corepack pnpm tauri android build --debug --apk "${args[@]}"

BT="$(ls -d "$ANDROID_HOME"/build-tools/* | sort -V | tail -1)"
READELF="$NDK_HOME/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-readelf"
for apk in src-tauri/gen/android/app/build/outputs/apk/*/debug/*.apk; do
  echo "== $apk ($(stat -c %s "$apk") bytes)"
  "$BT/aapt2" dump badging "$apk" | grep -E "^package:|minSdkVersion|targetSdkVersion|uses-permission:|uses-feature:|^native-code|^application-label:"
  "$BT/zipalign" -c -P 16 -v 4 "$apk" | tail -1
  "$BT/apksigner" verify --print-certs "$apk" | head -2
  tmp="$(mktemp -d)"
  unzip -q -o "$apk" 'lib/*' -d "$tmp"
  for so in "$tmp"/lib/*/*.so; do
    echo "$(basename "$(dirname "$so")")/$(basename "$so"): LOAD align $("$READELF" -lW "$so" | awk '$1=="LOAD"{print $NF}' | sort -u | tr '\n' ' ')"
  done
  rm -rf "$tmp"
done
