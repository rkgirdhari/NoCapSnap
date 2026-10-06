# CapSnap

**Private, dish-level guest feedback for restaurants.**

[![CI](https://github.com/rkgirdhari/nocapsnap/actions/workflows/ci.yml/badge.svg)](https://github.com/rkgirdhari/nocapsnap/actions/workflows/ci.yml)
[![Server and deploy kit](https://github.com/rkgirdhari/nocapsnap/actions/workflows/capsnap-server.yml/badge.svg)](https://github.com/rkgirdhari/nocapsnap/actions/workflows/capsnap-server.yml)
[![Android](https://github.com/rkgirdhari/nocapsnap/actions/workflows/android.yml/badge.svg)](https://github.com/rkgirdhari/nocapsnap/actions/workflows/android.yml)
· Proprietary, source visible ([LICENSE](LICENSE)) · [Security policy](SECURITY.md)

Restaurant staff photograph the dish they prepared for a guest. The guest scans a QR code and tells the restaurant,
privately, what they thought of that exact plate: a 1–5 rating and an optional comment. The restaurant uses it for
internal quality control.

- No app and no account for the guest.
- Nothing is posted publicly, and negative answers are never filtered out or redirected to review sites.

> **Status: pre-release.** CapSnap is built in reviewed stages ("gates", below). The server and guest page are built
> and tested; the Android app has not yet been released to the Play Store, and iOS is planned.

## How it works

1. **Staff app** (Android; iOS planned):
   - choose the dish and take the photo;
   - the photo is saved on the phone first, so a bad signal never loses it;
   - it syncs in the background.
2. **Server** (self-hosted, Rust):
   - checks that the photo is a clean image with no hidden metadata;
   - stores it and issues a one-use guest link, shown as a QR code.
3. **Guest page:**
   - the guest scans the code and rates the dish;
   - every guest sees the same neutral form, whatever their answer;
   - the feedback is stored privately for the restaurant. *(A staff screen for reading it is not built yet.)*

## Principles

- **Privacy first.**
  - No guest names, emails or phone numbers.
  - No third-party trackers, fonts or analytics.
  - Fixed retention: photos 30 days after sync, feedback 12 months, logs 30 days.
- **Self-hosted.** One Rust binary with SQLite, behind nginx. No cloud services.
- **Offline first.** The staff app works without a connection and catches up later.
- **Evidence over claims.** Every claim is labelled **Built**, **Specified** or **Aspirational**, and every stage
  ends with a written report that argues against itself.

**Hammurabi Coding Company LLC** · Founder **R. K. Girdhari** · Android package `com.hammurabicoding.nocapsnap`.

The owner's specification is the source of truth:
[`.hcc-nocapsnap/spec/SPEC-v1-operations-technical.md`](.hcc-nocapsnap/spec/SPEC-v1-operations-technical.md).
Stack: Tauri 2 + SvelteKit (static SPA) on Android; Rust/Axum + SQLite on the server.

## Work runs in gates

Each gate ends with a report in [`.hcc-nocapsnap/protocol/`](.hcc-nocapsnap/protocol/)
and waits for owner approval. Claims are labelled **Built** (compiles + tested),
**Specified** (designed, not built) or **Aspirational**.

| Gate | Scope | Status |
|---|---|---|
| W0 | Spec intake + SQLite-on-Android spike | Approved |
| W1 | Tauri 2 Android shell + camera spike, staff UI design | Closed; device checklist D7 waived by the owner |
| W2 | Staff UI from the owner's NO CAP SNAP mockups, on-device photo pipeline, ONB-1 import crate | Approved; device check D9 run on Android (2026-10-03): passed, two steps open (see the W2 record) |
| W3a | Hosted feedback slice, built and host-tested: server, guest portal, phone sync, real QR | Approved; D9 on the live server (2026-10-06): sign-in, sync, QR and the guest page pass |
| W3b | Deploy to the ZAP VPS (nginx, TLS, guest domain) | **Live** at <https://nocapsnap.hammurabi.click> (2026-10-06): Let's Encrypt, smoke 11/11, phone sign-in works; VPS on Ubuntu 26.04.1 |
| W4a | Backup tooling: encrypted snapshot, verify, restore, failure pause, daily timer, photos encrypted once into a pool | Built; tested on the host and in the CI deploy-kit rehearsal. W4b (copies 2 and 3, the owner's restore drill, alerting) waits on B1–B4 and a live box ([proposal](.hcc-nocapsnap/protocol/W4-PROPOSAL-operations-hardening.md)) |
| W5a | Privacy policy page (`/privacy`), Play Data Safety draft, CI check that keeps it true, `capsnapctl remove-staff`, in-app policy link | Built and tested; the draft is not yet entered in Play Console ([draft](.hcc-nocapsnap/protocol/W5a-data-safety-draft.md)) |
| W5 | Beta and Play readiness: signing, store listing, closed testing | Proposed; the rest waits on W3b, W4b and the publisher identity ([proposal](.hcc-nocapsnap/protocol/W5-PROPOSAL-beta-play-readiness.md)) |
| iOS | Apple App Store (not in the spec; added by the owner 2026-10-01) | Proposed; opens after W4 ([proposal](.hcc-nocapsnap/protocol/IOS-PROPOSAL-apple-app-store.md)) |
| Desktop | Windows build of the staff app (Spec Phase 7, brought forward by the owner 2026-10-01) | Installers built; awaiting the owner's first run ([record](.hcc-nocapsnap/protocol/DESKTOP-windows-staff-app.md)) |

## Layout

```
.
├── .github/workflows/capsnap-server.yml   # server CI + deploy kit rehearsal
├── .hcc-nocapsnap/
│   ├── spec/        # owner spec (verbatim)
│   ├── protocol/    # gate reports W0, W1, …
│   ├── design/      # owner mockups + measured palette
│   ├── evidence/    # gate evidence (W2: mockup-vs-build images, harness)
│   └── backlog/     # specified work (ONB-1)
├── app/             # staff app: SvelteKit static SPA + Tauri 2 (Android)
│   ├── src/         # UI — design language in src/app.css
│   ├── src-tauri/   # Rust core + gen/android (Gradle project)
│   └── scripts/     # build-android.sh (debug APK), package-android.sh (release signing + Play checks)
├── server/          # Axum + SQLite server: staff API, media checks, guest portal (/g/)
├── deploy/          # W3b: preflight, install, smoke test, release switching for the ZAP VPS
└── crates/
    ├── capsnap-store/   # SQLite outbox, menu cache, on-device photo pipeline
    ├── capsnap-sync/    # phone side of sync: sign-in, outbox upload, guest links
    └── capsnap-onboard/ # ONB-1: consent-based import from a restaurant's own site (server-side)
```

## Staff app (`app/`)

```bash
cd app
corepack enable && pnpm install
pnpm dev            # browser preview at http://127.0.0.1:1420 (in-memory, nothing saved)
VITE_CAPSNAP_PREVIEW=1 pnpm build && pnpm preview   # the same preview from a production build
pnpm check          # svelte-check, warnings fail
ANDROID_HOME=… NDK_HOME=… JAVA_HOME=… scripts/build-android.sh   # debug APK + Play checks
```

**Getting the APK and the Windows installers:**

- **From GitHub Actions.** The Android workflow (`.github/workflows/android.yml`) builds a release APK for every
  change to the app. Download it from the run's `capsnap-android` artifact, which is kept for 30 days. The Desktop
  workflow (`desktop.yml`) does the same for the Windows installers (`capsnap-windows`).
- **From a GitHub Release,** where they are kept permanently. Both workflows publish to the same release,
  `v<app version>`. Two ways to publish one:
  - push a tag `vX.Y.Z` that matches the app version (both workflows run);
  - or run each workflow by hand on `main` (Actions → Android / Desktop → Run workflow) with "publish a release"
    ticked. GitHub creates the tag `v<app version>` if it doesn't exist yet. If it does, that tag's commit is built
    and the files are added to its release.
- **Signing.** Until the Play upload key is added as repository secrets, builds are signed with a throwaway test
  key, so uninstall the previous build before installing a new one.

Details are in [the Android CI record](.hcc-nocapsnap/protocol/ANDROID-ci-build.md).

Rust tests: `scripts/test-host.sh` in each of `crates/capsnap-store`, `crates/capsnap-sync`,
`crates/capsnap-onboard` and `server`. The server README explains how to run it locally.
CI (`.github/workflows/capsnap-server.yml`) tests the server, builds a static x86_64 binary
and rehearses the deploy kit on a real systemd + nginx runner; `deploy/README.md` is the runbook.
A plain `pnpm build` (what Tauri bundles) leaves the browser preview out.

The test scripts clear `RUSTC_WRAPPER` and `CARGO_ENCODED_RUSTFLAGS`, so a user or parent
cargo config that asks for `sccache` or `mold` doesn't break the build.

The earlier Expo / Express / S3 prototype was removed per owner decision D2
(W0). It remains in the history of `rkgirdhari/software` (last in commit `7604304`).

## Where this came from

CapSnap was built under `nocapsnap/` in `rkgirdhari/software` (PR #1, gates W0 to W3b) and
moved here on 2026-10-01 at the owner's request, with its history (`git subtree split`).
Gate reports written before the move cite paths with a `nocapsnap/` prefix; drop it to find
the file here. Reports are records, so they were not rewritten, except where text is marked "redacted for
publication": details of the host's other services were removed before the repository went public.

## License and security

Proprietary: © 2026 Hammurabi Coding Company LLC, all rights reserved. The source is public to read. See
[LICENSE](LICENSE) for what that does and doesn't allow, and [CONTRIBUTING.md](CONTRIBUTING.md).

Found a vulnerability? Please report it privately, as described in [SECURITY.md](SECURITY.md).
