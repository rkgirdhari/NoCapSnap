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
| W1 | Tauri 2 Android shell + camera spike | In progress |

## Layout

```
nocapsnap/
├── .hcc-nocapsnap/
│   ├── spec/        # owner spec (verbatim)
│   └── protocol/    # gate reports W0, W1, …
└── spikes/
    └── w0-sqlite-android/   # SQLite outbox cross-compiled for Android API 36
```

The earlier Expo / Express / S3 prototype was removed per owner decision D2
(W0). It remains in git history; the last commit containing it is `7604304`.
