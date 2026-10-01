#!/usr/bin/env bash
# Turns Tauri's unsigned release build into CapSnap's Android deliverables, and checks them the way the W1-W3a
# gate reports did by hand. Run after `pnpm tauri android build --apk [--aab] --target aarch64`.
#
#   ANDROID_HOME=/path/to/sdk scripts/package-android.sh OUT_DIR
#
# Signing:
#   - Play upload key: set CAPSNAP_KEYSTORE (a keystore file), CAPSNAP_KEY_ALIAS, CAPSNAP_KEYSTORE_PASSWORD and
#     CAPSNAP_KEY_PASSWORD. The APK and the AAB (needs --aab in the build) are signed with it.
#   - Otherwise: the APK is signed with a throwaway key made for this run. That is a test build: Android will not
#     install it over a build signed with a different key, so uninstall the old one first. No AAB is produced,
#     because Google Play only accepts bundles signed with the upload key.
#
# Needs: build-tools 36.1+ (BUILD_TOOLS picks a version; default: the newest installed), a JDK (keytool and
# jarsigner), node, readelf and unzip.
set -euo pipefail
cd "$(dirname "$0")/.."

OUT="${1:?usage: package-android.sh OUT_DIR}"
: "${ANDROID_HOME:?set ANDROID_HOME to the Android SDK}"
BT="$ANDROID_HOME/build-tools/${BUILD_TOOLS:-$(ls "$ANDROID_HOME/build-tools" | sort -V | tail -1)}"
OUTPUTS=src-tauri/gen/android/app/build/outputs
APK_IN="$OUTPUTS/apk/universal/release/app-universal-release-unsigned.apk"
AAB_IN="$OUTPUTS/bundle/universalRelease/app-universal-release.aab"
VERSION="$(node -p 'require("./src-tauri/tauri.conf.json").version')"
PACKAGE=com.hammurabicoding.nocapsnap

fail() { echo "::error::$*" >&2; exit 1; }

# The browser preview bridge (simulated sign-in and sync) must never ship. Tauri embeds the frontend in the
# native library compressed, so check the build it embedded.
if grep -rl "guests.preview.invalid" build; then fail "the browser preview bridge is in the release build"; fi

[ -f "$APK_IN" ] || fail "no unsigned release APK at $APK_IN"
mkdir -p "$OUT"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT

if [ -n "${CAPSNAP_KEYSTORE:-}" ]; then
  : "${CAPSNAP_KEY_ALIAS:?}" "${CAPSNAP_KEYSTORE_PASSWORD:?}" "${CAPSNAP_KEY_PASSWORD:?}"
  [ -f "$AAB_IN" ] || fail "an upload key is set but there is no AAB at $AAB_IN (build with --aab)"
  signing="upload key"
  ks="$CAPSNAP_KEYSTORE"
  apk_out="$OUT/CapSnap_${VERSION}_arm64.apk"
else
  signing="throwaway test key"
  ks="$work/test.keystore"
  CAPSNAP_KEY_ALIAS=capsnap-test
  CAPSNAP_KEYSTORE_PASSWORD="$(od -An -N24 -tx1 /dev/urandom | tr -d ' \n')"
  CAPSNAP_KEY_PASSWORD="$CAPSNAP_KEYSTORE_PASSWORD"
  export CAPSNAP_KEYSTORE_PASSWORD CAPSNAP_KEY_PASSWORD
  keytool -genkeypair -keystore "$ks" -storetype PKCS12 -alias "$CAPSNAP_KEY_ALIAS" -keyalg RSA -keysize 3072 \
    -validity 3650 -dname "CN=CapSnap test build, O=Hammurabi Coding Company LLC" \
    -storepass:env CAPSNAP_KEYSTORE_PASSWORD -keypass:env CAPSNAP_KEY_PASSWORD > /dev/null 2>&1 \
    || fail "keytool could not make the test key"
  apk_out="$OUT/CapSnap_${VERSION}_arm64_test-signed.apk"
fi
export CAPSNAP_KEYSTORE_PASSWORD CAPSNAP_KEY_PASSWORD

# APK: page-align the native library for 16 KB devices, then sign (v2/v3 signatures cover the aligned file).
"$BT/zipalign" -P 16 -f 4 "$APK_IN" "$work/aligned.apk"
"$BT/apksigner" sign --ks "$ks" --ks-key-alias "$CAPSNAP_KEY_ALIAS" \
  --ks-pass env:CAPSNAP_KEYSTORE_PASSWORD --key-pass env:CAPSNAP_KEY_PASSWORD --out "$apk_out" "$work/aligned.apk"
rm -f "$apk_out.idsig"

echo "== $(basename "$apk_out") ($(stat -c %s "$apk_out") bytes, signed with the $signing)"
"$BT/apksigner" verify --verbose --print-certs "$apk_out" > "$work/verify.txt" || fail "apksigner verify failed"
grep -E "^Verified using v2 scheme \(APK Signature Scheme v2\): true$" "$work/verify.txt" > /dev/null \
  || fail "the APK has no v2 signature"
grep -E "^Signer #1 certificate (DN|SHA-256 digest):" "$work/verify.txt"
"$BT/zipalign" -c -P 16 4 "$apk_out" || fail "the APK is not 16 KB page-aligned"
echo "zipalign -c -P 16 4: aligned"

# Play checks, as in the gate reports.
"$BT/aapt2" dump badging "$apk_out" > "$work/badging.txt"
"$BT/aapt2" dump xmltree --file AndroidManifest.xml "$apk_out" > "$work/manifest.txt"
grep -E "^package:|minSdkVersion|targetSdkVersion|^native-code|^application-label:|uses-permission:" "$work/badging.txt"
expect() { grep -E "$2" "$work/$1" > /dev/null || fail "$3"; }
expect badging.txt "^package: name='$PACKAGE' versionCode='[0-9]+' versionName='$VERSION'" "package name or version"
expect badging.txt "^minSdkVersion:'24'$" "minSdk is not 24"
expect badging.txt "^targetSdkVersion:'(3[6-9]|[4-9][0-9])'$" "targetSdk is below 36"
expect badging.txt "^native-code: 'arm64-v8a'$" "native code is not arm64-v8a only"
expect manifest.txt ":usesCleartextTraffic\([^)]*\)=false$" "usesCleartextTraffic is not false"
expect manifest.txt ":allowBackup\([^)]*\)=false$" "allowBackup is not false"
if grep -q "application-debuggable" "$work/badging.txt"; then fail "the APK is debuggable"; fi
unzip -q -o "$apk_out" 'lib/*' -d "$work/apk"
for so in "$work"/apk/lib/*/*.so; do
  align="$(readelf -lW "$so" | awk '$1=="LOAD"{print $NF}' | sort -u | paste -sd' ')"
  echo "$(basename "$(dirname "$so")")/$(basename "$so"): $(stat -c %s "$so") bytes, LOAD align $align"
  [ "$align" = "0x4000" ] || fail "$(basename "$so") is not 16 KB-aligned"
done

# AAB, for Google Play: only with the upload key.
if [ "$signing" = "upload key" ]; then
  aab_out="$OUT/CapSnap_${VERSION}.aab"
  cp "$AAB_IN" "$aab_out"
  jarsigner -keystore "$ks" -storepass:env CAPSNAP_KEYSTORE_PASSWORD -keypass:env CAPSNAP_KEY_PASSWORD \
    "$aab_out" "$CAPSNAP_KEY_ALIAS" > /dev/null
  jarsigner -verify "$aab_out" | grep -q "^jar verified\.$" || fail "the AAB signature does not verify"
  unzip -l "$aab_out" | grep -q " base/lib/arm64-v8a/" || fail "the AAB has no arm64-v8a native code"
  echo "== $(basename "$aab_out") ($(stat -c %s "$aab_out") bytes, signed with the upload key; jar verified)"
fi

shopt -s nullglob
(cd "$OUT" && sha256sum -- *.apk *.aab > SHA256SUMS.txt && cat SHA256SUMS.txt)
