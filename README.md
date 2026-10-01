# CapSnap (nocapsnap)

Private, dish-level guest feedback for restaurants.
**Hammurabi Coding Company LLC** · Founder: **R. K. Girdhari** ·
Android package `com.hammurabicoding.nocapsnap`

The owner specification is the source of truth:
[`.hcc-nocapsnap/spec/SPEC-v1-operations-technical.md`](.hcc-nocapsnap/spec/SPEC-v1-operations-technical.md).
Target stack: Tauri 2 + SvelteKit (static SPA) on Android, Rust/Axum + SQLite
server, self-hosted, no cloud services.

## Work runs in gates

Each gate ends with a report in [`.hcc-nocapsnap/protocol/`](.hcc-nocapsnap/protocol/)
and waits for owner approval. Claims are labelled **Built** (compiles + tested),
**Specified** (designed, not built) or **Aspirational**.

| Gate | Scope | Status |
|---|---|---|
| W0 | Spec intake + SQLite-on-Android spike | Approved |
| W1 | Tauri 2 Android shell + camera spike, staff UI design | Closed; device checklist D7 waived by the owner |
| W2 | Staff UI from the owner's NO CAP SNAP mockups, on-device photo pipeline, ONB-1 import crate | Approved (device check D9 still open) |
| W3a | Hosted feedback slice, built and host-tested: server, guest portal, phone sync, real QR | Approved (device check D9 still open) |
| W3b | Deploy to the ZAP VPS (nginx, TLS, guest domain) | Open; deploy kit in progress. G1 (domain) still needed |

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
│   └── scripts/build-android.sh
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
the file here. Reports are records, so they were not rewritten.
