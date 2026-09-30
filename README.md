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
| W1 | Tauri 2 Android shell + camera spike, staff UI design | Built; awaiting owner device checklist (D7) |

## Layout

```
nocapsnap/
├── .hcc-nocapsnap/
│   ├── spec/        # owner spec (verbatim)
│   └── protocol/    # gate reports W0, W1, …
├── app/             # staff app: SvelteKit static SPA + Tauri 2 (Android)
│   ├── src/         # UI — design language in src/app.css
│   ├── src-tauri/   # Rust core + gen/android (Gradle project)
│   └── scripts/build-android.sh
└── spikes/
    └── w0-sqlite-android/   # local-first SQLite outbox + capture ingest
```

## Staff app (`app/`)

```bash
cd nocapsnap/app
corepack enable && pnpm install
pnpm dev            # browser preview at http://127.0.0.1:1420 (in-memory, nothing saved)
pnpm check          # svelte-check, warnings fail
ANDROID_HOME=… NDK_HOME=… JAVA_HOME=… scripts/build-android.sh   # debug APK + Play checks
```

Rust tests for the store: `spikes/w0-sqlite-android/scripts/test-host.sh`.

The repo-root `.cargo/config.toml` belongs to HMS Forge and expects `sccache`
and `mold`; the CapSnap scripts switch those off (`RUSTC_WRAPPER=`).

The earlier Expo / Express / S3 prototype was removed per owner decision D2
(W0). It remains in git history; the last commit containing it is `7604304`.
