# capsnap-store

CapSnap's on-device store, used by the Tauri app (`../../app/src-tauri`).
Promoted from the W0 spike (`spikes/w0-sqlite-android`) in W2.

- **Outbox** (Spec §3): captures are committed to SQLite (WAL, `synchronous=FULL`)
  as `pending` and flip to `synced` only on a server acknowledgement.
- **Photo pipeline** (Spec §3 "Client-Side Hardening", `src/photo.rs`): decode,
  apply the EXIF orientation, downscale to 2048 px on the long edge, re-encode as
  JPEG with no metadata segments (EXIF, GPS, XMP, ICC, comments), plus a 480 px
  thumbnail. The original bytes are never written to disk.
- **Menu cache** (`src/menu.rs`): the shape of `GET /locations/{id}/menu-items`
  (Spec §5); until W3 it holds a demo seed whose rows say `source = 'demo'`.
- **Device settings**: a closed set of keys (display name, current location).

Commands:

- Host tests: `scripts/test-host.sh`
- Photo timing: `cargo test --release --test photo -- --ignored --nocapture`
- Android build + evidence: `ANDROID_NDK_HOME=/path/to/ndk-r30 scripts/build-android.sh 36`

Gate reports: [`../../.hcc-nocapsnap/protocol/`](../../.hcc-nocapsnap/protocol/)
