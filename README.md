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
| W2 | Staff UI from the owner's NO CAP SNAP mockups, on-device photo pipeline, ONB-1 import crate | Built; awaiting owner review (device check D9 open) |

## Layout

```
nocapsnap/
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
└── crates/
    ├── capsnap-store/   # SQLite outbox, menu cache, on-device photo pipeline
    └── capsnap-onboard/ # ONB-1: consent-based import from a restaurant's own site (server-side)
```

## Staff app (`app/`)

```bash
cd nocapsnap/app
corepack enable && pnpm install
pnpm dev            # browser preview at http://127.0.0.1:1420 (in-memory, nothing saved)
VITE_CAPSNAP_PREVIEW=1 pnpm build && pnpm preview   # the same preview from a production build
pnpm check          # svelte-check, warnings fail
ANDROID_HOME=… NDK_HOME=… JAVA_HOME=… scripts/build-android.sh   # debug APK + Play checks
```

Rust tests: `crates/capsnap-store/scripts/test-host.sh` and `crates/capsnap-onboard/scripts/test-host.sh`.
A plain `pnpm build` (what Tauri bundles) leaves the browser preview out.

The repo-root `.cargo/config.toml` belongs to HMS Forge and expects `sccache`
and `mold`; the CapSnap scripts switch those off (`RUSTC_WRAPPER=`).

The earlier Expo / Express / S3 prototype was removed per owner decision D2
(W0). It remains in git history; the last commit containing it is `7604304`.
