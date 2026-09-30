# W0 — Spec intake + SQLite-on-Android feasibility spike

Date: 2026-09-30 · Branch: `ccr-8308799d-xptdcx` (PR #1) · Spec: [`../spec/SPEC-v1-operations-technical.md`](../spec/SPEC-v1-operations-technical.md)

## Gate report

```
## Gate W0 — Spec intake + SQLite-on-Android spike
Status: Built (SQLite outbox crate: host-tested; cross-compiles for Android API 36 and 24)
        Specified (runs on a device; Tauri-Android camera; everything else in the spec)
Evidence: see "Evidence" — cargo test 8/8, clippy -D warnings clean, NDK r30 builds for
          aarch64/x86_64-linux-android36 with SQLite statically linked and 16 KB-aligned LOAD segments
Changes: nocapsnap/spikes/w0-sqlite-android/ (new), nocapsnap/.hcc-nocapsnap/ (new). No existing code touched.
Tenth Man: nothing has executed on Android; the camera half of the hard gate is untouched (see below)
Decision needed from owner: approve W0 and open W1 (Tauri Android shell + camera spike)? — plus D2–D6
```

## Goal

1. Take in the owner spec and record, with labels, where the code already on PR #1 conflicts with it.
2. Run the part of Spec §7 Phase 1 ("Prove cross-compilation of the SQLite crate for Android 16") that this
   container can actually prove. The device and camera half of that hard gate is out of reach here and is
   proposed as W1.

## Spike: what was built

`nocapsnap/spikes/w0-sqlite-android/` is a standalone crate that stays out of the repo-root Cargo workspace.
It is a minimal local-first outbox (Spec §3 "Atomic Persistence", §5 "Captures"):

- SQLite via `sqlx 0.9` with `sqlite-bundled`: SQLite is compiled from source by the target's C compiler.
  On Android that compiler is the NDK clang. This is what the gate has to prove.
- WAL journal with `synchronous=FULL`, so a committed capture survives the app being killed or a power loss.
- `captures` table:
  - `client_id` is `UNIQUE`; it is the idempotency key for the future `POST /captures`.
  - `media_sha256` has a `CHECK` for 64 lowercase hex characters.
  - `sync_state` is `pending` or `synced`.
  - There is a partial index on pending rows.
  - The table is `STRICT`.
- `enqueue` is idempotent and `mark_synced` only moves a row once. Together they back the "saved offline / QR
  not ready" → "Synced / QR ready" states in Spec §3.
- `capsnap_spike_selftest` is a C-ABI export. It keeps the whole SQLite path inside the Android `.so`, the
  same kind of `lib<name>.so` Tauri ships.
- `selftest` is a CLI binary that runs the same round trip, meant to be run on a device over adb.

Reproduce:

```bash
nocapsnap/spikes/w0-sqlite-android/scripts/test-host.sh
ANDROID_NDK_HOME=/path/to/android-ndk-r30 nocapsnap/spikes/w0-sqlite-android/scripts/build-android.sh 36
```

## Evidence

Toolchain: `rustc 1.94.1`, `cargo 1.94.1`. NDK: `android-ndk-r30-linux.zip`, whose SHA-1 was checked against
Google's `repository2-3.xml`:

```
ndk.zip: OK
Pkg.Revision = 30.0.16248370   Pkg.ReleaseName = r30
aarch64-linux-android36-clang  (present, alongside 24/35/37)
```

Host tests (`scripts/test-host.sh`):

```
running 8 tests
test new_captures_start_pending_in_capture_order ... ok
test mark_synced_moves_capture_out_of_the_outbox_once ... ok
test committed_captures_survive_close_and_reopen ... ok
test ffi_selftest_returns_zero_and_rejects_null ... ok
test opens_in_wal_with_full_sync_and_foreign_keys ... ok
test re_enqueue_is_idempotent_on_client_id ... ok
test schema_rejects_malformed_digests ... ok
test selftest_round_trip_passes ... ok
test result: ok. 8 passed; 0 failed
```

```
$ cargo clippy --locked --all-targets -- -D warnings   → Finished, no warnings
$ cargo fmt --check                                     → clean
$ cargo run --bin selftest
W0 selftest ok sqlite=3.51.3 journal=wal path=/tmp/capsnap-w0-selftest.db
```

Android cross-compile (`scripts/build-android.sh 36`):

```
== aarch64-linux-android API 36
libcapsnap_store_spike.so: ELF 64-bit LSB shared object, ARM aarch64, version 1 (SYSV), dynamically linked, stripped
selftest:                  ELF 64-bit LSB pie executable, ARM aarch64, version 1 (SYSV), dynamically linked, interpreter /system/bin/linker64, stripped
size: 2838856 bytes
NEEDED: libdl.so libm.so libc.so
LOAD alignment: 0x4000
exported: capsnap_spike_selftest
embedded SQLite source id: 2026-03-13 10:38:09 737ae4a34738ffa0c3ff7f9bb18df914dd1cad163f28fd6b6e114a344fe6d618
== x86_64-linux-android API 36
libcapsnap_store_spike.so: ELF 64-bit LSB shared object, x86-64, ...
NEEDED: libdl.so libm.so libc.so
LOAD alignment: 0x4000
exported: capsnap_spike_selftest
embedded SQLite source id: 2026-03-13 10:38:09 737ae4a3…e6d618
```

`scripts/build-android.sh 24 aarch64-linux-android` gives the same result: the same `NEEDED` libs, `0x4000`
alignment, the export and the SQLite source id. It builds for Tauri's default `minSdk` too.

How to read the evidence:

- **`NEEDED` is only bionic `libc`/`libm`/`libdl`.** SQLite is statically linked, so the app doesn't depend
  on the device's own SQLite. The NDK doesn't offer that as a stable API anyway.
- **`LOAD alignment: 0x4000`.** The library is 16 KB page-aligned. Google Play requires 16 KB page-size
  support for apps targeting Android 15+, and blocks non-compliant updates from Feb 1, 2027
  (developer.android.com/guide/practices/page-sizes). NDK r28+ aligns to 16 KB by default.
- **The source-id string is present.** SQLite code is in the stripped `.so`. The repo-root
  `[profile.release] strip = true` removes symbols, so a symbol count proves nothing here.

## Labels

| Item | Label | Basis |
|---|---|---|
| SQLite outbox semantics (WAL, idempotent enqueue, one-way sync ack, digest CHECK, durability across reopen) | **Built** | 8 host tests |
| SQLite crate cross-compiles for Android API 36 (arm64 + x86_64) and API 24 (arm64), 16 KB aligned, statically linked | **Built** | NDK r30 build output above |
| The same code *runs* on Android 16 | **Specified** | Not executed: no device, no KVM for an emulator. Owner procedure in D3 |
| Tauri-Android camera access | **Specified** | Untouched; proposed W1 |
| Tauri shell, SvelteKit SPA, Axum server, guest portal, auth, retention, backups | **Specified** | Spec only |
| Expo app + Express API + S3 code on PR #1 | **Built, off-spec** | See the conflict table |

## Existing code on PR #1 vs the spec

PR #1 (commits `e6681d1`…`6dd654f`) was built from the earlier Expo scaffold brief. Measured against this
spec, most of it is superseded:

| Spec | PR #1 today | Conflict |
|---|---|---|
| §2 Tauri 2 + SvelteKit + Axum + SQLite | Expo SDK 57 / React Native, Express/TypeScript | Whole stack |
| §2 no AWS; self-hosted | `S3Storage` (`services/api/src/lib/storage/s3.ts`), CloudFront docs | Rejected dependency. The `local` driver is closer to the spec |
| §4 256-bit token in the fragment `/g/#token`, then `POST /api/v1/guest/session` + `replaceState` | 128-bit token in the path: `randomBytes(16)` → `${appUrl}/r/${token}` (`services/api/src/modules/photos/service.ts:38,63`) | Entropy, token placement and exchange |
| §5 tenant IDs come from the session, never the body | `locationId` read from the request body (`services/api/src/modules/photos/schema.ts:4`) | Tenant isolation |
| §5 `POST /media` (streamed binary) + `POST /captures` (idempotent) under `/api/v1` | base64 JSON `POST /api/photos/capture` (`services/api/src/app.ts:16`) | Contract |
| §6 no guest PII | `customerName` on `Review` / `SubmitReviewRequest` (`packages/shared/src/types.ts:63,81`; `services/api/src/modules/reviews/schema.ts:7`) | **Direct violation**, even though it is a stub |
| §3 resize and EXIF strip on-device; server validates signature, MIME, SHA-256 and decode | Server-side sharp pipeline; no digest | Responsibility split |
| §3 SQLite outbox with offline states | In-memory `pendingCapture` (`apps/mobile/src/state/pendingCapture.ts`) | Not local-first |
| §6 retention defaults | None | Missing |
| §6 only CAMERA | Expo manifest: CAMERA, INTERNET, VIBRATE (`apps/mobile/app.json:20`) | See D5 |
| — (not in the spec) | Watermark stamp, "Share review link" (share sheet/SMS) | Unspecified features; D2 decides |

Reusable as reference material: the Play-readiness findings (API 36 target, 16 KB pages, blocked RECORD_AUDIO),
the UI flow and palette, the placeholder icons, and the store listing text (which needs its Data safety
section rewritten).

## Findings in the spec itself

1. **§6 "Only CAMERA is permitted" vs sync.**
   - Syncing needs `INTERNET`. That is a normal, install-time permission, not a runtime prompt.
   - Reading §6 as "CAMERA is the only *runtime/dangerous* permission" makes it consistent.
   - It needs owner confirmation (D5).
2. **§2 "Tauri 2.5+ … low-level access to the Android camera" has no official Tauri camera plugin behind it.**
   The realistic options (W1 decides, with a device) are:
   - (a) WebView `getUserMedia`. Needs the Android WebView permission bridge.
   - (b) `<input type="file" accept="image/*" capture>`. Hands off to the system camera app via an intent;
     the app then may not need CAMERA at all.
   - (c) A small custom Kotlin Tauri plugin.
   - This choice also decides whether the manifest carries CAMERA (§6).
3. **§3 client-side resize/EXIF strip.** Doing this in Rust on the device (e.g. the `image` crate) adds
   native size and needs its own Android build check. A WebView canvas is lighter but its EXIF handling
   differs by WebView version. W2 decides.
4. **§4 one-time links + "Share review link" by SMS.** Any sharing puts a bearer token into third-party
   messaging. The spec doesn't mention sharing at all (D2).
5. **§7 Zap hosting.** Axum needs a long-running Linux process, so a static web space plan can't host it.
   The spec already calls this a hard gate (D4).

## Tenth Man

The strongest case that W0 is wrong, incomplete or overclaimed:

- **Nothing ran on Android.** Compiling proves the NDK toolchain can build bundled SQLite and link it; it
  doesn't prove bionic runtime behavior. Things like WAL `-shm` files under SELinux or in the app sandbox,
  `fsync` on real flash, or a different page size at runtime are unproven. Even the adb self-test (D3) runs as
  the `shell` user in `/data/local/tmp`, not inside the Tauri app's private storage. Only W1's APK closes that.
- **The harder half of the hard gate is untouched.** Spec §7 says "verify Tauri-Android camera access before any
  UI development". W0 cannot pass the whole gate; calling Phase 1 "done" would be an overclaim.
- **sqlx 0.9 is new.** It requires Rust ≥ 1.94 (`rust-version = "1.94.0"` in its index entry), which pins every
  developer and CI machine to that toolchain. `rusqlite` would be a smaller, sync alternative with the same
  bundled-SQLite build path. The choice here followed the house default (SQLx) and wasn't benchmarked.
- **`synchronous=FULL` + WAL is the durable choice, but its write latency on cheap kitchen tablets hasn't been
  measured.**
- **Schema gaps.** Timestamps are `TEXT`, and pending order relies on uniform RFC 3339 UTC strings. The schema
  doesn't enforce that format yet.
- **16 KB alignment is proven for this one `.so`, not for a full APK.** The final APK still needs
  `zipalign -c -P 16 -v 4` and Play's pre-launch report.
- **The repo-root `.cargo/config.toml` (HMS Forge) forces `sccache` + `mold`.** The spike scripts neutralize it
  with `RUSTC_WRAPPER=` / `CARGO_ENCODED_RUSTFLAGS=`. A plain `cargo test` in this directory fails on a machine
  without those tools.
- **The branch constraint put a Rust spike on the same PR as the off-spec Expo/Express code.** Merging PR #1
  as-is would land a stack the spec rejects (D1/D2).

## Decisions needed from owner

- **D1 — Gate:** approve W0 and open **W1: Tauri 2 Android shell + camera spike**? Yes/No.
  - I would scaffold a minimal Tauri 2 app that links this crate and build a debug APK for API 36 in this
    container, with evidence of the build and the 16 KB `zipalign` check.
  - It would implement camera options (a) and (b) above behind two buttons.
  - You would run it on a physical Android 16 device, because this container has no device or KVM.
- **D2 — PR #1's Expo / Express / S3 code:** it conflicts with the spec (table above). Nothing gets discarded
  without your approval. Options:
  - (a) Remove it from the branch in W1. It stays in git history.
  - (b) Keep it as reference until the Rust stack reaches parity.
  - (c) Close PR #1 and continue on a fresh PR.
  - This also decides whether the watermark and "Share review link" features carry over.
- **D3 — On-device SQLite check (about 5 minutes, optional before W1):** with a USB-debugging Android 16 phone:
  ```bash
  ANDROID_NDK_HOME=/path/to/android-ndk-r30 nocapsnap/spikes/w0-sqlite-android/scripts/build-android.sh 36 aarch64-linux-android
  adb push nocapsnap/spikes/w0-sqlite-android/target/api36/aarch64-linux-android/release/selftest /data/local/tmp/
  adb shell /data/local/tmp/selftest /data/local/tmp/w0.db
  # expect: W0 selftest ok sqlite=3.51.3 journal=wal path=/data/local/tmp/w0.db
  ```
  That result upgrades "runs on Android" from Specified to Built, but only at the shell level (see Tenth Man).
- **D4 — Hosting:** which Zap plan is it: a vServer/root server (a long-running Axum process is fine) or static
  web space (not viable)? This blocks Spec Phase 3.
- **D5 — Permissions:** confirm that §6 means "CAMERA is the only runtime permission; INTERNET (install-time) is
  allowed". If camera option (b) wins in W1, should the app ship without CAMERA at all?
- **D6 — minSdk:** both API 24 (Tauri's default) and API 36 builds work. Should the app target 36 with minSdk
  24, or raise minSdk (e.g. 29+) to shrink the test matrix?
