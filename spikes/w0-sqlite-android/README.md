# W0 spike — SQLite outbox on Android

Feasibility spike for Spec §7 Phase 1: a minimal local-first capture outbox
(SQLite via sqlx 0.9, bundled) that cross-compiles for Android API 36.

- Host tests: `scripts/test-host.sh`
- Android build + evidence: `ANDROID_NDK_HOME=/path/to/ndk-r30 scripts/build-android.sh 36`
- On a device: see D3 in the gate report.

Gate report, evidence and open decisions:
[`../../.hcc-nocapsnap/protocol/W0-spec-intake-sqlite-android-spike.md`](../../.hcc-nocapsnap/protocol/W0-spec-intake-sqlite-android-spike.md)

This is throwaway spike code, not the app's store crate. Labels follow the
HCC gated protocol: only what the tests and build output show is **Built**.
