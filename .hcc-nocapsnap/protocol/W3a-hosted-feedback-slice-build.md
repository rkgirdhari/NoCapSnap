# W3a — Hosted feedback slice, built and host-tested

Date: 2026-09-30 · Status: **Approved by the owner (2026-09-30).** The owner opened W3a with *"Approved, open W3a"*.
See "Closure".
Proposal: [`W3-PROPOSAL-hosted-feedback-slice.md`](W3-PROPOSAL-hosted-feedback-slice.md).

Nothing was deployed; that is W3b. Nothing has run on an Android device (D9 is still open).

## Gate report

```
## Gate W3a — Hosted feedback slice (build + host test)
Status: Built — host, browser and APK evidence; no deployment, no device run
Evidence: 57 host tests (store 25, onboard 17, sync 6, server 9); guest portal browser checks;
          real QR decoded by jsQR, ZXing and OpenCV; clippy -D warnings on all crates
          (+ app for Android); svelte-check 0/0; release APK Play checks (see "Evidence")
Changes: server/ (new), crates/capsnap-sync (new), crates/capsnap-store (migration 0004),
         crates/capsnap-onboard (TLS fix), app/ (sign-in, sync, real QR), evidence/W3a/
Tenth Man: the sync client and the server have only ever talked on 127.0.0.1; no phone, TLS or real
           domain has been in the loop yet
Decision needed from owner: approve W3a; for W3b: D4, G1, G2; plus D9, R1, P1 (below)
```

Commits:

| Commit | Content |
|---|---|
| `21867df` | Server; ONB-1 https fix |
| `3b34f23` | Sync client and store sync state |
| `1370da2` | App sign-in, sync and real QR |
| (this report's commit) | Formatting and this report |

## What was built

### Server (`nocapsnap/server`, Rust, Axum and SQLite in one binary)

- **Staff sign-in:**
  - Passwords are hashed with Argon2id.
  - Each phone gets a device session: a 256-bit bearer token, stored only as a SHA-256 hash. Sessions last 30
    days and can be revoked (`revoke-sessions`, per the spec runbook's "Device lost").
  - Sign-in is limited to 5 failures per sign-in name per 15 minutes.
  - A wrong password and an unknown name get the same answer, with equal-cost hashing.
- **Tenancy (Spec §5).** The organization, role and allowed locations come from the session only.
  - Request bodies are `deny_unknown_fields`, so an `orgId` is refused.
  - Other tenants' ids get "not found".
- **Media (Spec §5).** Uploads are streamed with a 10 MiB cap and accepted only if:
  - the SHA-256 matches the declared `X-Content-SHA256`;
  - the MIME type is `image/jpeg` and the file signature is a JPEG;
  - there are no metadata segments: no APP1–APP15, no COM, and APP0 must be plain JFIF with no thumbnail;
  - the image fully decodes;
  - its long edge is at most 2048 px.

  Anything else is refused. The server checks that the phone did its job. It does not re-process photos, which
  is stricter than the proposal's "re-strips".
- **Captures.**
  - Idempotent on the phone's `clientId`.
  - The acknowledgement carries `<public base>/g/#<token>`. The token is 256-bit and only its hash is stored.
  - Links are single-use and last 30 days.
  - A retried capture (the phone never saw the answer) gets a fresh token, and the old one stops working; the
    server can't resend a token it doesn't keep. Once a guest has answered, no new link is issued.
- **Guest portal** (`/g/`, Spec §4):
  - A static page compiled into the binary.
  - It reads the fragment token, calls `history.replaceState` before anything else, and exchanges the token
    once for a 30-minute `HttpOnly; SameSite=Strict; Secure` cookie.
  - The guest sees the plate photo and one neutral form: 1–5 plus an optional comment of up to 2,000
    characters (counted as characters, not bytes).
  - Each link accepts one submission.
  - The CSP allows only `'self'`. There are no third-party assets and no guest PII fields.
- **ONB-1 endpoint** (`POST /onboarding/import`):
  - Admins and managers only.
  - Consent is recorded from the session before any fetch.
  - It returns the draft and saves nothing else.
- **Retention (Spec §6).** The job runs hourly and on demand (`capsnap-server retention`):
  - photos are deleted 30 days after sync;
  - photos never attached to a capture are deleted after 1 day;
  - feedback is deleted after 12 months;
  - expired guest sessions and old device sessions are deleted.
- **Logs.** Each request logs the method, route *template*, status and time. There are no paths, queries or
  bodies, and no colour codes when not on a terminal.
- **Security headers** on every response:
  - CSP;
  - `nosniff`;
  - `no-referrer`;
  - `DENY` framing;
  - `no-store` caching;
  - a Permissions-Policy.
- **Admin commands.** `create-org`, `create-location` (checked against the IANA tz database), `create-staff`
  (password from stdin), `import-menu`, `revoke-sessions` and `retention`. There is no public sign-up.

### Phone sync (`crates/capsnap-sync` + store migration 0004)

- **Sign-in:**
  - Picks a location: the current one if still allowed, else the first.
  - Replaces the demo menu with the server's (`source = server`).
  - Stores the session in app-private storage. Backup is disabled since W2.
- **Outbox:**
  - Each photo is uploaded once; the server's media id is remembered.
  - The capture is then posted and the acknowledgement stored.
  - A network, session or server failure stops the run. A capture the server refuses is recorded and skipped.
  - A photo the server no longer has is uploaded again.
  - A revoked session signs the phone out locally and keeps its plates.
- **Transport:**
  - Staff credentials only go over `https://`. Plain http is allowed only to the phone itself or the Android
    emulator's host.
  - All HTTP runs in Rust (`ureq` with rustls/ring and bundled Mozilla roots). The WebView's CSP stays closed.
- **What stays on the phone:**
  - The **table label is never sent**: the request type has no such field (owner default M6).
  - **Demo plates never sync.**

### App

- **Settings:**
  - Signed out: a sign-in form (server address, sign-in name, password). The password is never stored.
  - Signed in: the restaurant and role, a location picker, Sync now and Sign out.
- **Sync timing.** Sync runs right after each capture and every minute while the app is open, one run at a time.
  Screens refresh on a `sync-updated` event.
- **History:**
  - Retry is live when signed in.
  - "Last synced" is real.
  - Demo plates read "Demo · stays on this phone" and don't count as waiting.
- **Invitation.** The **real guest QR**:
  - It encodes the server-issued link at error correction H.
  - It uses round data dots with **square finder patterns** and a 4-module quiet zone.
  - Its expiry date is shown.
  - It has honest states for pending, demo, expired and no-link.

## Findings fixed during the gate

1. **ONB-1 could not read any https website.** W2 approved it as Built.
   - The crate was compiled with reqwest's default features off and no TLS feature added, so there was no TLS
     backend. Every https request failed before DNS with "scheme is not http".
   - W2's tests only used plain http, so W2's **Built** label for ONB-1 was an overclaim.
   - Fixed: rustls is enabled, and a new https test pins the behaviour.
   - Still unproven: a real TLS handshake with a public site. This sandbox only allows traffic through a proxy,
     and the importer deliberately never uses one.
2. **The mockup's rounded QR finder squares defeat OpenCV.** Rendering the same payload in five styles showed
   that rounded finders fail OpenCV's classic and Aruco detectors at every size, while ZXing and jsQR read them.
   Square finders with dots are read by all four decoders. The component was changed to square finders.
3. **The sync result disappeared.** After a full Retry the waiting block (and its "1 synced." message) vanished.
   The result now shows outside that block.
4. **Server logs carried ANSI colour codes** when not on a terminal (journald). Fixed.

## Evidence

**Host tests** (each crate's `scripts/test-host.sh`, `--locked`): **57 passed**.

| Crate | Tests | Covers |
|---|---|---|
| `crates/capsnap-store` | 25 (W2 suite) | Still green after migration 0004 |
| `crates/capsnap-onboard` | 17 | Includes the new https-blocked test |
| `crates/capsnap-sync` | 6 end to end | Against the real server over HTTP on 127.0.0.1 (listed below) |
| `server` | 9 | Listed below |

Sync end-to-end tests:

- Sign in, sync, then open a guest session with the phone-held link.
- Offline, then recover.
- A lost acknowledgement recovered with no duplicate on the server.
- Re-upload of a photo the server no longer has.
- A revoked session.
- Bad credentials and non-https server addresses.

Server tests:

- Headers.
- Sign-in, the rate limit, expiry and revocation.
- Tenant and location isolation.
- Media rejection paths, with nothing left behind.
- Capture idempotency and link rotation.
- The guest flow: cookie flags, bounds, one-time use, identical errors.
- Retention.
- Onboarding role and consent.
- **Logs contain no bearer token, guest token, session cookie, password, comment, location id or image bytes.**
  This test runs in its own process.

**Lint:**

- `cargo clippy --all-targets --locked -D warnings` has zero findings on all four crates.
- The app compiles cleanly under clippy for `aarch64-linux-android`.
- `cargo fmt --check` is clean.
- `pnpm check` reports **299 files, 0 errors, 0 warnings**.

**Guest portal in Chromium**, against the real server binary (harness in `../evidence/W3a/harness/`):

| Check | Result |
|---|---|
| Token gone from the address bar after load; history entry replaced | pass |
| Same form for a 1-star and a 5-star guest (ignoring dish name and photo) | pass |
| Same thank-you for rating 1 and rating 5 | pass |
| Reopening a used link shows "no longer available" | pass |
| Third-party requests | none |
| Console or page errors | none |
| Server log contains either token | no |
| Server log contains the comment | no |

**Real guest QR** (from the app's invitation screen, preview build, 717 px screenshot; a 43-character token
confirmed; [`04-real-qr-decoded.webp`](../evidence/W3a/04-real-qr-decoded.webp)):

| Size | jsQR 1.4.0 | ZXing (zxing-cpp) | OpenCV 5.0 | OpenCV Aruco |
|---|---|---|---|---|
| 717 px | ✓ | ✓ | ✓ | ✓ |
| 300 px | ✓ | ✓ | ✓ | ✓ |
| 200 px | ✓ | ✓ | ✓ | ✓ |
| 130 px | ✓ | ✓ | ✓ | ✓ |
| 110 px | ✓ | ✓ | ✓ | — |

With the mockup's rounded finders, OpenCV and Aruco failed at every size; the variant test is in the harness.

**Screens:**

| Image | Shows |
|---|---|
| [`01-app-capture-to-qr.webp`](../evidence/W3a/01-app-capture-to-qr.webp) | Demo plate → pending → History after sync → real QR |
| [`02-app-settings-sign-in.webp`](../evidence/W3a/02-app-settings-sign-in.webp) | Settings, signed out and signed in |
| [`03-guest-portal.webp`](../evidence/W3a/03-guest-portal.webp) | Guest form → thank-you → used link |

The app screens come from the browser preview, whose sign-in and sync are simulated (labelled; compiled out of
release). The real app-to-server path is the Rust end-to-end suite above.

**Release APK:** (`pnpm tauri android build --apk --target aarch64`, then zipalign `-P 16` and apksigner with
the debug key):

| Check | Result |
|---|---|
| File | `capsnap-w3a-arm64-release-debugsigned.apk`, 12,343,950 bytes, sha256 `1a050894416f547b2cb0bd5aab61221589292b1ccf32f02de73dde6efd060548` |
| Package | `com.hammurabicoding.nocapsnap`, versionName 0.1.0, versionCode 1000, label "CapSnap" |
| SDK | minSdk 24, targetSdk 37, compileSdk 37 |
| Permissions | Unchanged from W2: INTERNET, CAMERA, plus AndroidX's signature-level `DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION` |
| Native code | arm64-v8a only; `libcapsnap_app_lib.so` 10,591,744 bytes (W2: 8,731,432; the sync client and its TLS stack), LOAD align `0x4000` |
| 16 KB alignment | `zipalign -c -P 16 -v 4`: Verification successful |
| Signature | apksigner verified; signer "CN=Android Debug" (debug key, **not** the Play upload key) |
| Manifest | `usesCleartextTraffic=false`, `allowBackup=false`, `fullBackupContent=false`, `dataExtractionRules` set |
| Preview bridge | Not in the release frontend bundle (no preview strings); `simulate_ack` absent from the `.so`, `run_selftest` and `session_sign_in` present |

`usesCleartextTraffic=false` binds Java and WebView networking. The sync client is Rust and opens its own sockets,
so the http-over-`adb reverse` route in the device checklist below depends on `check_server_url` (http only to
127.0.0.1, localhost or 10.0.2.2), not on the manifest. On a phone that is **Specified** until D9 runs.

## Labels

| Item | Label | Why |
|---|---|---|
| Server: auth, tenancy, media validation, captures, guest links and portal, retention, logs, admin commands | **Built** | 9 host tests plus the browser portal check |
| Phone sync client (sign-in, outbox, recovery paths) | **Built** | 6 end-to-end tests against the real server over HTTP |
| App sign-in, sync UI and real QR | **Built** | Browser flow (preview), svelte-check, Android clippy, APK |
| ONB-1 over https | **Built** up to the DNS stage | The https-blocked test. A real TLS handshake to a public site is **Specified** (sandbox has no direct egress) |
| Anything on a phone: sign-in, background sync, QR on screen, scanning with a real phone camera | **Specified** | No device run |
| Deployment on the vServer, TLS certificate, guest domain, journald 30-day log limit | **Specified** | W3b (D4, G1, G2) |
| Backups and restore drill | **Specified** | W4 (Spec Phase 4) |
| Staff and menu management in an app (today: server commands) | **Aspirational** | Not in W3 |

## Device checklist (adds to W2's; about 15 minutes)

```
# On a computer: run the server, then forward the phone's port 8080 to it (no https needed this way).
CAPSNAP_PUBLIC_BASE_URL=http://127.0.0.1:8080 CAPSNAP_INSECURE_COOKIES=1 capsnap-server serve
adb reverse tcp:8080 tcp:8080
```

1. **Settings → Sign in** with server `http://127.0.0.1:8080` and a staff account made with `create-staff`.
   - Pass: the restaurant's name and menu appear, and the Demo tags are gone.
2. **Capture a dish.** Within a few seconds the invitation turns from "Saved offline · QR not ready" into
   "Ready to share." with a QR.
3. **Scan that QR** with another phone's camera app.
   - The guest page opens, and its address bar shows no token.
   - Send a rating.
   - Scanning again says the link is no longer available.
4. **Airplane mode, then capture.** The plate waits. Turn airplane mode off: it syncs within a minute, or at once
   with Retry.
5. **Revoke the phone's session on the server:** `capsnap-server revoke-sessions <login>`. The next sync signs the
   phone out, and its plates stay.
6. Report pass or fail per step.

Note for step 3: the guest link points at `http://127.0.0.1:8080` in this setup, so the scanning phone needs its
own `adb reverse`. The real test of step 3 is W3b, with a domain and https.

## Tenth Man

- **Nothing has left 127.0.0.1.**
  - The sync client and the server have never crossed a real network, TLS or a real domain.
  - The app has never run this code on a phone.
  - W2 found an IPC bug by reading source, and W3a found a missing TLS backend by testing. Assume more of both.
- **The spec's hard gate is still open.**
  - D4 (hosting) has been open since W0.
  - If the ZAP product can't run a long-lived Linux process, W3b is blocked, and W3a's server has no home.
- **A retried capture silently retires its QR.** If the phone did receive a link, showed it, and then retried
  anyway (a bug, or a crash after saving but before recording), the QR already on screen stops working. The sync
  client records the acknowledgement before anything else, which makes this unlikely, but it isn't impossible.
- **The guest portal is a plain static page, not SvelteKit.** Spec §2 names SvelteKit for the frontend. A single
  form didn't justify a second build pipeline in the server, but it is a deviation (P1).
- **Staff management is command-line only.** Creating staff, resetting passwords and editing menus need shell
  access to the server. That's fine for a pilot with the owner at the keyboard, and not fine for restaurants.
- **The sign-in rate limit is per name, in memory.** A restart clears it. A spray across many names isn't limited
  per address, because behind a proxy every client shares one address. Caddy (G2) could add per-address limits.
- **No real guest data yet (R1).** Until W4's backups and restore drill, a server disk failure would lose every
  capture and every piece of feedback.
- **The QR moved away from the mockup** (square finders). This was chosen for scannability; the owner may want to
  see it on paper.

## Decisions needed from owner

- **W3a:** approve this gate?
- **D4, G1, G2** (unchanged) are needed for W3b:
  - D4: hosting details (the ZAP product and the `uname …` output; no passwords).
  - G1: the guest domain.
  - G2: Caddy (recommended) or certificates inside the server.
- **D9:** run the device checklist (W2's plus the one above) before W3b. Recommended.
- **R1:** test data only until the W4 restore drill passes. Recommended; no answer yet.
- **P1 (new):** keep the guest page as a small static page (recommended: 7.5 KB in all, no build step), or rebuild it
  in SvelteKit to match Spec §2's wording?

## Closure (2026-09-30)

- **W3a approved.** The owner replied *"approved"* to this report. The evidence and labels stand as written: every
  device item stays **Specified**, and so does everything in W3b.
- **P1 (guest page): default kept.** The guest portal stays a small static page served by the server. This is a
  recorded deviation from Spec §2's SvelteKit wording.
- **R1 (real guest data): default kept.** Test data only until the W4 restore drill passes.
- **D9 (device check): not answered, so still open.** The approval is not read as a waiver. It is still
  recommended before W3b.
- **D4, G1, G2: still open.** W3b cannot start without them. D4 is Spec §7's hard gate.
- Next: W3b (deploy), once D4, G1 and G2 are answered. Nothing starts without the owner's approval.
