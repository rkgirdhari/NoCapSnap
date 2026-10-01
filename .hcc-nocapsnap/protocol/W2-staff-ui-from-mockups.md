# W2 — Staff UI from the owner's mockups (local vertical slice)

Date: 2026-09-30 · Status: **Approved by the owner (2026-09-30).** Nothing in W2 has run on an Android device.
D7 was waived in W1, and D9 (below) is still open. See "Closure".

**Input.** Five high-fidelity mockups from the owner, titled "NO CAP SNAP":
1. Home
2. Capture 01/03 "Which dish?"
3. Capture 02/03 "The plate, as served."
4. History while offline
5. Guest invitation / QR

They are now committed in [`../design/mockups/`](../design/mockups/) with SHA-256 checksums. Their palette was
**measured** from the files, not estimated; see `../design/mockups/README.md` and `../design/sample-mockups.py`.

## Gate report

```
## Gate W2 — Local vertical slice
Status: Approved 2026-09-30 (built; browser, host and APK evidence only; no device run)
Evidence: 25 store tests + 17 import tests; svelte-check 0/0; clippy -D warnings (Android arm64, debug and release);
          release APK Play checks; mockup-vs-build images; QR non-scannable (2 decoders); see "Evidence"
Changes: crates/capsnap-store (promoted + photo pipeline + menu), crates/capsnap-onboard (new, ONB-1),
         app/ (new UI, bridge, Android backup rules), .hcc-nocapsnap/evidence/W2/
Tenth Man: nothing has run on a phone; W2 found a latent Android IPC bug from W1 by reading Tauri's source
Decision needed from owner: D9 device check before W3; M4 display serif; M7 photo colour profile
```

## Screen-by-screen mapping

| Mockup | What it needs | W2 (offline, on device) | Needs the server (W3) | Spec |
|---|---|---|---|---|
| **Home** — "Good evening, Maya.", location pill "Atelier No. 8 ▾", hero photo "Tonight's service · 12 captures", "Capture a dish", 2 waiting / 8 QR ready cards, Recent plates, tabs Home / History / Insights / Settings | Staff first name, allowed locations, today's captures with thumbnails and dish names, counts | Layout, counts, thumbnails from local media, recent list. Name and location from a locally provisioned profile, clearly marked as W2 stub data | Real staff identity and the list of authorised locations from the session | §5: tenant and role come from the session, never from client input |
| **01/03 Prepare — "Which dish?"** Menu search, All / Mains / Starters chips, dish cards with photos and radio select, "Table label (optional) · For staff reference only", "Continue to camera" | Menu items per location with category and photo | A local `menu_items` table seeded from a fixture with the same shape as the server's; search and filter run locally | `GET /api/v1/locations/{id}/menu-items`, cached for offline use | §5 route map |
| *(camera step, not in the mockups)* | — | The W1 camera routes (viewfinder or phone camera), restyled | — | §3 |
| **02/03 Review — "The plate, as served."** Photo; dish · location · table; "Check the dish. Keep diners and receipts out of frame."; Retake / "Save this capture"; "Saved on this device first." | On-device resize and EXIF strip before save | Resize to 2048 px and strip EXIF/GPS in Rust on the device, then save to the outbox | — | §3 client-side hardening; §6 no guest PII (the privacy hint helps) |
| **History (offline)** — "Your plates are safe here."; "2 waiting to sync · Saved on this device. Guest QR not ready."; "Last synced Today · 6:42 PM"; rows with thumbnail, dish, time and a Waiting to sync / QR ready chip; "Retry when connected" | Outbox state, last-sync time, a sync client | Everything except the sync itself. "Last synced" reads "Never" and Retry is disabled until W3 | The sync protocol and server acknowledgement | §3 outbox and wording |
| **Guest invitation — "Ready to share."** Synced · QR ready, dish card, styled QR (rounded modules, centre emblem), "Ask your guest to scan for private feedback.", "This invitation can expire or be revoked." | A live 256-bit capability token and a fragment URL (`/g/#token`) with expiry and revocation | The screen only, showing a **non-scannable placeholder** labelled "Concept QR · not live" (as the mockup does). No fake links to guests | Token issue, expiry and revocation; the guest portal with fragment exchange and `replaceState` | §4, §6 retention (links last 30 days or until first submission) |
| **Insights** tab | Aggregated private feedback | Placeholder only | Needs feedback data (after W3) | §1 internal QC only; never public |
| **Settings** tab | Account, location, sign-out, the W1 device check | Device check and about screen | Sign-out and session revocation | §7 runbook: device lost |

## Visual deltas from the current W1 build (measured from the mockup files)

| Role | W1 build (brand sheet, D8) | Mockups (measured) | Mockup contrast |
|---|---|---|---|
| Background | Royal Purple `#2D1B4E` | Deep aubergine `#190926` | — |
| Headline | Ivory `#FBF7EE`, system serif | Cream `#FCE9C5`, high-contrast display serif | 15.88:1 |
| Primary CTA | Gold `#F6E05E` | Warm yellow `#FDCF10` with ink `#110803` | 13.31:1 |
| Cards | Bamboo Brown `#744210` with a muted-gold edge | Layered plum-browns: `#2A1B27` (dish, review), `#1E1223` (list), `#533A39` (stat card, pill), `#402C2E` (input) | stat card against background 1.83:1, so it needs a border (as W1 found) |
| Muted text | Text-safe muted gold `#C18A39` | Warm mauve-greys `#98827C`, `#A1949B`, `#8A7D86` | 4.66–6.71:1 |
| Synced / online | Bamboo Green `#68D391` | `#9AD999` on chip `#272F28`; dot `#8FEB8C` | 8.37:1 |
| Waiting / offline | Gold chip | Amber `#E8B872` on `#3A2827`; `#F2BF72` | 7.62:1, 11.25:1 |
| QR | White card, dark modules | Cream `#FDF0CE`, aubergine modules `#1D0E25` | 16.23:1 |

**Every text pair measured in the mockups passes WCAG AA**; the lowest is the Review kicker at 4.66:1.

Other deltas:

- **Wordmark:** "NO CAP SNAP" in tracked capitals.
- **Chips:** carry icons (hourglass, QR).
- **Photography:** dish photos everywhere.
- **Tabs:** Home, History, Insights, Settings. Capture becomes the primary CTA; Manager folds into Settings.

## Proposed W2 scope and exit evidence

Build (offline, on device):
- New navigation and the Home, Prepare, camera, Review and History screens.
- The guest-invitation screen with a non-live placeholder.
- Settings with the device check.
- A local menu fixture.
- On-device resize and EXIF strip.
- Thumbnails.
- Promote the store to `crates/capsnap-store`.
- Remove the browser-preview bridge and `simulate_ack` from release bundles.

Exit evidence:
- Rust tests: resize (2048 long edge); EXIF/GPS removal verified by re-parsing; menu and outbox queries.
- svelte-check clean.
- Browser screenshots per screen next to each mockup.
- Release APK Play checks, as in W1.
- A device checklist covering the new flow.

## Tenth Man (proposal)

- **The spec's hard gate forbids this order.** Spec §7 Phase 1 says "verify Tauri-Android camera access before
  any UI development". D7 (the device checklist) hasn't run. Building five polished screens on an unproven camera
  path risks rework. The owner can waive this explicitly, but it should be a recorded decision, not drift.
- **The mockups imply data W2 can't truthfully have.** A staff name, locations, menus with photos and live QR
  codes all need the server. Stub data risks looking like a working product in demos. Stubs must be visibly
  labelled.
- **Display-font fidelity has a cost.** Matching the mockup serif means bundling a font. Spec §6 bans *external*
  fonts; a bundled, OFL-licensed font file is local, but it adds size and needs a licence check (house rule: verify
  the licence page, not the marketing copy).
- **The styled QR can fail to scan.** Rounded modules and a centre emblem need error correction level H and a clear
  quiet zone. W3 must prove scannability with a decoder test.
- **There are three colour sources.** The brand sheet, the W1 build and the mockups each differ. Without one
  canonical source, every screen becomes a judgement call.

## Decisions needed from owner (proposal; answered below)

- **M1 — Order.** Waive the D7 device check and start W2 now, or run D7 first (recommended by the spec's own hard
  gate)?
- **M2 — Name on screen.** The in-app wordmark "NO CAP SNAP" versus Spec §1's official display name "CapSnap".
  Proposal: the Android label and Play listing stay "CapSnap"; the in-app wordmark is "NO CAP SNAP".
- **M3 — Canonical colours.** The mockups (measured above, all AA) or the brand sheet (W1)? Recommendation:
  adopt the measured mockup palette as the design tokens, since it's the owner's most detailed artefact and it
  passes AA. Keep the brand-sheet names as the marketing palette.
- **M4 — Display serif.** Bundle an OFL display serif to match the mockups (licence and size checked first), or
  stay with the phone's system serif?
- **M5 — Status wording.** The mockup chips say "Waiting to sync". Spec §3 requires "saved offline / QR not ready"
  to be explicit. Proposal: keep the mockup chip and banner, but the chip reads "Saved offline · QR not ready" to
  satisfy the spec.
- **M6 — Table label.** Device-only (staff reference, never synced) or synced to the server as staff metadata? It
  is never shown to guests either way.

## Owner decisions (2026-09-30)

| # | Question | Owner's answer | Applied as |
|---|---|---|---|
| M1 | Build before the D7 device check? | *"Send me the QR Code and then waive and move on"* | D7 waived; install QR sent; W2 started |
| M2 | Name on screen | NO CAP SNAP in-app | In-app wordmark "NO CAP SNAP"; the Android label and Play listing stay "CapSnap" (Spec §1) |
| M3 | Canonical colours | Mockup palette | The measured palette (`../design/mockups/README.md`) becomes the design tokens |
| O1 | Onboarding | *"Build in the URL Search & Scrape for the Restaurant being requested … where they either link it or give us permission to scrape"* | **Consent-based import from the restaurant's own website** replaces the Google Places idea (see ONB-1). No third-party service. |
| M4 | Display serif | not answered | **Default:** the device serif stays (no bundled font) |
| M5 | Status wording | not answered | **Default:** chips carry the mockup's look with the spec's words ("Saved offline · QR not ready") |
| M6 | Table label | not answered | **Default:** device-only; never synced or shown to guests |


---

# W2 build report (2026-09-30)

Commits on `ccr-8308799d-xptdcx`:

| Commit | Content |
|---|---|
| `1c15b52` | Store promoted to `crates/capsnap-store`; photo pipeline; menu cache |
| `ca0c3cd` | Staff UI |
| `30feb63` | ONB-1 import crate |
| (this report's commit) | Android backup rules and this report |

## What was built

### Store (`crates/capsnap-store`)

- **Photo pipeline** (`src/photo.rs`, Spec §3 "Client-Side Hardening"):
  - Every photo is decoded and the EXIF orientation is applied to the pixels.
  - It is downscaled to at most **2048 px on the long edge**. Photos are never upscaled.
  - It is re-encoded as a baseline JPEG (q 85) with **no metadata segments at all**: no EXIF, GPS, XMP, ICC or
    comments.
  - A 480 px thumbnail is written beside it.
  - Large reductions use area averaging. Small ones use Catmull-Rom.
  - Sources above 16,384 px per side are refused.
  - The original bytes are never written to disk.
  - The SHA-256 is of the processed file.
  - The work runs on a blocking thread.
- **Migration 0003:**
  - `menu_items`, with `source` = `demo` | `server` | `import`, so demo rows can't pass as real ones.
  - `settings`: a closed key set of display name and location.
  - Capture details: `menu_item_id`, the **copied** `dish_name`, a **device-only** `table_label` (≤ 16
    characters from letters, digits, spaces and `-#/.`; owner default M6), and the processed width and height.
- **Demo seed.** On first run the app seeds "Atelier No. 8" and 7 dishes. The first three are in mockup 02's order.
- **`photo_from_base64`.** Used by the Android IPC path (see finding 1). The length is capped before decoding.

### App (`app/`)

- **Tokens.** The mockup palette is the design tokens (M3).
  - Every text colour was checked against every solid surface it sits on. All pairs pass WCAG AA; the table is
    at the top of `src/app.css`. The one exception is text over the Home hero photo, which relies on a dark
    gradient, so its contrast depends on the photo (as in the mockup).
  - Two colours were restricted to surfaces where they pass: helper grey `#8A7D86` fails on the `#2A1B27` card
    (4.17:1), and placeholders use `#B3A6AD` (5.55:1 on inputs).
- **Chrome.** The in-app wordmark is "NO CAP SNAP" (M2; the Android label stays "CapSnap"). The tabs are Home,
  History, Insights and Settings; Manager folded into Settings.
- **Screens:**
  - **Home:** greeting and service line, location pill tagged **Demo**, today's hero photo and count, "Capture a
    dish", "N saved offline" and "N QR ready", recent plates.
  - **01/03 "Which dish?":** search (ignores case and accents), category chips, dish cards showing each dish's
    latest capture thumbnail, table label with inline validation, a sticky "Continue to camera". The subtitle
    reads "Demo menu" while the menu is the seed.
  - **Camera:** the in-app viewfinder or the phone's camera app, switchable; the choice is remembered in Settings.
  - **02/03 "The plate, as served.":** the photo is shown **whole, not cropped**, so staff can check the edges
    for diners and receipts; this is a deliberate deviation from the mockup's crop. Then the dish · location ·
    table card, the privacy hint, Retake, "Save this capture" and "Saved on this device first."
  - **03/03 guest invitation:**
    - While pending: "Saved on this device." with "Saved offline · QR not ready" and a placeholder where the QR
      will be.
    - Once acknowledged: "Ready to share.", "Synced · QR ready" and the **concept QR**, captioned "Concept QR ·
      not live". It is deliberately not decodable.
  - **History:** "Your plates are safe here.", the offline banner, "Last synced" (reads **Never** until an
    acknowledgement exists), day groups, rows with thumbnail, time, table and chip. Retry is **disabled** with
    the reason shown.
  - **Insights:** local numbers only (today, last 7 days, most captured). Guest feedback is shown as "not live
    yet", not faked.
  - **Settings:** name for the greeting, restaurant (Demo), camera route, the privacy statement, the device check
    (SQLite round trip in the app sandbox), and a **debug-build-only** "Simulate server acknowledgement".
- **Status wording.** Every status chip carries the spec's words, "Saved offline · QR not ready" and "Synced ·
  QR ready" (M5 default), in the mockup's chip style.
- **Back button.** The capture steps are shallow-routed, so Android's back button walks Review → Camera →
  Which dish?.
- **Browser preview.** It is compiled only under `vite dev` or `VITE_CAPSNAP_PREVIEW=1`. The bundle Tauri ships
  does not contain it: no demo menu, canvas pipeline or in-memory store.
- **Android:**
  - Window colours use `#190926`.
  - **Backup and device transfer are switched off** (`allowBackup=false`, `fullBackupContent=false`, and
    `dataExtractionRules` excluding every domain; Spec §6).
  - There is a minimum inset for the status and gesture bars under edge-to-edge.

### ONB-1 (`crates/capsnap-onboard`)

This implements the owner's O1 design as a library: consent → guarded fetch → extract → draft. See the crate README
and `../backlog/ONB-1-onboarding-autofill.md`. The `POST /api/v1/onboarding/import` endpoint and its review
screen come with the W3 server.

## Findings fixed during the gate

1. **Android would have rejected every photo** (latent since W1; W1's raw-bytes claim was wrong for Android).
   - On Android, Tauri 2.12 never uses its custom IPC protocol. Its own `ipc-protocol.js` says so: *"on Android
     we never use it because Android does not have support to reading the request body"*.
   - Instead the payload goes over `postMessage` as JSON. A `Uint8Array` becomes a JSON array of numbers, and
     `capture_ingest` accepted only `InvokeBody::Raw`.
   - Fix: on Android the UI sends `{photoBase64}` (FileReader, about 1.33× the photo's size). Rust accepts raw
     bytes or base64, with the length capped before decoding.
   - The encoder matches Node's base64 byte for byte from 0 to 6 MB.
   - Responses are unaffected: raw media responses come back through Tauri's channel fetch, which Android
     supports (read in `ipc/protocol.rs`).
   - **Still Specified until run on a device.**
2. **The camera could switch itself on after the user had left.** `getUserMedia` resolving after the user
   switched to the phone camera app, or pressed Back, used to start the stream anyway. The browser flow exposed
   it. A start token now stops any late stream immediately.
3. **Retake raced the back navigation.** Retake now waits until the camera step is showing before it starts the
   camera.
4. **The robots.txt wildcard matcher was exponential** on patterns like `/*a*a*a…b`. It now uses linear
   backtracking. A test covers 40 wildcards against a 4,000-character path.
5. **Robots errors were mislabelled.** A 5xx on robots.txt was reported as "robots.txt asks not to read /", and a
   blocked DNS answer surfaced as a robots error. Both now have their own errors: `RobotsUnavailable` and
   `BlockedAddress`.
6. **The preview module shipped in release.** A top-level `.map()` pinned it in the bundle. It is now a factory,
   and the release bundle is grep-clean.
7. **Android backup would have copied tenant data off the device** (the default `allowBackup`). It is now off, as
   described above.

## Evidence

**Store**, `crates/capsnap-store/scripts/test-host.sh`: 25 passed (outbox 8, ingest 7, menu 3, photo 7). One timing
probe is ignored by default. `cargo clippy --all-targets -D warnings` is clean.

- The EXIF fixture is hand-built:
  - IFD0 has Make "Pix" and **Orientation 6**; the GPS IFD holds latitude **41° 52′ 30″ N**.
  - An independent parser (kamadak-exif 0.6.1) reads all three from the input first.
- After processing:
  - The same parser reports **no EXIF at all**, in both the photo and the thumbnail.
  - A walk of the JPEG markers finds only APP0, DQT, SOF, DHT, DRI and SOS: **no APP1–APP15 and no COM**.
  - The byte string `Exif\0\0` is absent.
- A 64×32 photo with Orientation 6 comes out 32×64, and the red corner moves top-left → top-right.
- Size checks:
  - 3000×1000 (area averaging) and 1200×2400 (Catmull-Rom) both come out with a long edge of exactly 2048, and
    the short edge within 1 px of the true ratio.
  - 800×600 is kept.
  - Thumbnails come out at 480 px.
- A truncated or header-only JPEG is refused.
- Output is deterministic, so digests deduplicate.
- **Timing** (host x86-64, release): a 4000×3000 JPEG becomes 2048×1536 plus its thumbnail in **223 ms**. The
  source is synthetic and compresses well, so a real 12 MP photo will take longer; phone timing is unmeasured.
- **Android cross-compile** (`scripts/build-android.sh 36`, at `1c15b52`), both 16 KB aligned (`LOAD 0x4000`),
  NEEDED only libdl, libm and libc:

| Target | `libcapsnap_store.so` size |
|---|---|
| arm64 | 2,883,032 bytes |
| x86_64 | 2,948,016 bytes |

**App:**

- `pnpm check` (svelte-check, warnings fail): **297 files, 0 errors, 0 warnings**.
- `cargo clippy --target aarch64-linux-android -- -D warnings` is clean in both debug and release.
- `simulate_ack` exists only under `#[cfg(debug_assertions)]`. The release clippy build compiles the handler list
  without it, and the name is absent from the release library, while `run_selftest` (also 12 bytes) is present.

**Browser flow** (Playwright, harness in `../evidence/W2/harness/`):

- Setup:
  - The preview build runs at 375×667 @3x, in America/Chicago, on a fixed Tuesday-evening clock.
  - It uses Roboto and Noto Serif, the fonts Android's WebView uses for system-ui and serif. They were loaded
    from `@fontsource-variable` (OFL-1.1), for the test only.
- The run:
  - Name set.
  - Three plates captured through the real UI with the phone-camera route (dish photos cropped from the owner's
    mockups as camera input).
  - One acknowledgement simulated.
  - History checked with the network off.
  - The viewfinder route checked with Chromium's fake camera.
- Results:
  - Back from Review leaves no live video element.
  - Retake restarts the live camera.
  - No page errors. The only failed requests were `/favicon.png` while offline.

**Mockup vs build** (mockup on the left, build on the right):

| Screen | Image |
|---|---|
| Home | [`01-home.webp`](../evidence/W2/01-home.webp) |
| Which dish? | [`02-prepare.webp`](../evidence/W2/02-prepare.webp) |
| Review | [`03-review.webp`](../evidence/W2/03-review.webp) |
| History, offline | [`04-history-offline.webp`](../evidence/W2/04-history-offline.webp) |
| Guest invitation | [`05-guest-invitation.webp`](../evidence/W2/05-guest-invitation.webp) |

Also: [`06-other-screens.webp`](../evidence/W2/06-other-screens.webp) (empty Home, pending invitation, viewfinder,
Insights) and [`07-settings-full.webp`](../evidence/W2/07-settings-full.webp).

**Remaining visual deltas.** Most of the gap is the display serif. The mockups use a condensed high-contrast
serif, while the build uses the device's Noto Serif (M4 default), which is wider. Headings run one size smaller
to compensate, and long dish names wrap to two lines. The location pill has no ▾ because there is only one
location. The "Demo" tags are additions.

**Concept QR is not scannable.**

| Image | jsQR 1.4.0 (scales 1, ½, ¼) | OpenCV 5.0.0 `QRCodeDetector` |
|---|---|---|
| Screenshot of the concept QR | **nothing decoded** | corners found, **nothing decoded** |
| Control: an existing real QR | decoded at every scale | decoded |

**ONB-1**, `crates/capsnap-onboard/scripts/test-host.sh`: 17 passed (robots 6, import 11). Clippy `-D warnings`
is clean.

- The import tests run against an in-process HTTP server with test hosts pinned to 127.0.0.1. They cover:
  - JSON-LD `Restaurant` plus a declared `/menu`, including an invalid JSON-LD block that is skipped.
  - Fallbacks: OpenGraph, `tel:` links and microdata.
  - robots.txt: Disallow-all, our own group overriding `*`, and 5xx meaning "read nothing".
  - Redirects: `www` → page is followed; off-site is refused **without contacting** the other host; more than 3
    redirects is refused.
  - Caps: size (both declared and streamed), non-HTML content, timeout, and the page limit.
  - Consent: mismatched consent causes **zero requests**.
  - The production IP policy refuses 127.0.0.1, 10.0.0.8 and 169.254.169.254 at DNS time, again with **zero
    requests**.
  - IP classification tables (22 blocked, 5 public), URL rules and the deny-list.
- **Mutation check.** With `is_public_ip` forced to `true`, 2 tests fail. It was restored afterwards.
- **Not proven here.** `no_proxy()`: this sandbox sets only `HTTPS_PROXY`, and the tests use plain http.

**Release APK** (`pnpm tauri android build --apk --target aarch64`, then zipalign `-P 16` and apksigner with the
debug key):

| Check | Result |
|---|---|
| File | `capsnap-w2-arm64-release-debugsigned.apk`, 10,480,270 bytes, sha256 `345d6b69d7064f77e0098bb073f8d22b95577918102e4941399465dd6c668845` |
| Package | `com.hammurabicoding.nocapsnap`, versionName 0.1.0, versionCode 1000, label "CapSnap" |
| SDK | minSdk 24, targetSdk 37, compileSdk 37 |
| Permissions | INTERNET, CAMERA, plus AndroidX's signature-level `DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION` (not user-facing) |
| Features | `camera.any` (required); rear `camera` not required |
| Native code | arm64-v8a only; `libcapsnap_app_lib.so` 8,731,432 bytes, LOAD align `0x4000` |
| 16 KB alignment | `zipalign -c -P 16 -v 4`: Verification successful |
| Signature | apksigner verified; signer "CN=Android Debug" (debug key, **not** the Play upload key) |
| Manifest | `usesCleartextTraffic=false`, `allowBackup=false`, `fullBackupContent=false`, `dataExtractionRules` set |

## Labels

| Item | Label | Why |
|---|---|---|
| Photo pipeline, menu cache, settings, outbox | **Built** | Host tests; compiles for Android arm64 |
| Every W2 screen, both camera routes in a browser, status wording, demo labels | **Built** | Browser flow and screenshots |
| Release APK packaging and Play checks | **Built** | aapt2, zipalign, apksigner |
| ONB-1 import library | **Built** | Host tests |
| Anything on an Android device: WebView camera permission, both routes, base64 IPC, media responses, SQLite in the sandbox, insets, phone-side photo timing and memory | **Specified** | No device run (D7 waived; D9 below) |
| Sync, real guest QR tokens, sign-in, the ONB-1 endpoint and review screen | **Specified** | W3 and later |
| Bundled display serif (M4), Google Places onboarding | **Aspirational** | Not decided / needs a spec amendment |

## Device checklist (about 10 minutes, Android 12 or later, USB debugging)

```
adb uninstall com.hammurabicoding.nocapsnap   # W1 build, if installed
adb install capsnap-w2-arm64-release-debugsigned.apk
```

1. Launch.
   - Pass: no white flash, the header clears the status bar, and the tabs clear the gesture bar.
2. **Settings:**
   - Enter a name and tap Save. Home should greet you by it.
   - Run the device check. Expected: `ok sqlite=3.51.3 journal=wal · app sandbox`.
3. **Capture a dish** with the in-app viewfinder:
   1. Pick a dish and enter table "12B".
   2. Tap **Continue to camera**. The camera permission prompt should appear at this moment.
   3. Take a photo. Review shows the whole photo.
   4. Tap **Save this capture**. The screen shows "03 / 03 · Share", "Saved on this device." and "Saved offline ·
      QR not ready".
4. Repeat with **Use the phone camera app instead**. No extra permission prompt should appear.
5. **Back button:** from Review → Camera → Which dish? → Home. The camera indicator must go off when you leave
   the camera step.
6. **History:**
   - Rows show thumbnails, times and "Table 12B".
   - Turn on airplane mode: the header shows **Offline**, and Retry is disabled.
7. **Kill and reopen** the app: both plates are still there.
8. **Pull one stored photo and check it** (this proves Spec §3 on the device):
   ```
   adb exec-out run-as com.hammurabicoding.nocapsnap sh -c 'find . -name "*.jpg" ! -name "*.thumb.jpg" | head -1'
   adb exec-out run-as com.hammurabicoding.nocapsnap cat <that path> > plate.jpg
   exiftool plate.jpg   # expect no GPS, Make, Model or Orientation; long edge ≤ 2048
   ```
9. Report pass or fail per step, with a screenshot of anything that looks wrong.

## Tenth Man (build)

- **Nothing has run on a phone, and W2 shows what that costs.**
  - A core W1 claim ("raw bytes over IPC") was false on Android. Only a read of Tauri's source caught it.
  - The fix (base64) is itself unverified.
  - Other traps of the same kind are likely: the WebView's camera permission bridge, FileReader memory on large
    photos, `env(safe-area-inset-*)` support in older WebViews.
  - Spec §7 made the device check a hard gate for exactly this reason.
- **Memory on low-end phones.** A full-resolution photo from the camera-app route (12–50 MP, 4–12 MB) is held
  several times at peak:
  - the JS bytes and their base64 string;
  - the Java UTF-16 copy on the `postMessage` bridge;
  - the Rust decode (50 MP RGB is about 150 MB).

  This is unmeasured on a device. A mitigation, if needed, is to downscale in the WebView before sending. It would
  keep orientation (`createImageBitmap` applies it), but the Rust pipeline would no longer see the original.
- **Colour.** Dropping the ICC profile tags Display-P3 photos as untagged, so they look slightly duller on
  wide-gamut screens. Converting to sRGB is possible (`moxcms` is already in the dependency tree) at some CPU cost
  (M7).
- **The table label can still hold a name.** "SMITH" passes the character rule. It is device-only, never synced,
  and labelled "For staff reference only", but the rule can't enforce intent.
- **Demo data can still be screenshotted as if it were real.** It is tagged "Demo" in three places. The greeting
  name is whatever staff type.
- **ONB-1 trusts the requester's word.** "I own or manage this website" can't be verified by the crate. Consent
  is recorded with who, when and which URL; robots.txt and the low page cap limit the harm of a false claim. A
  stronger check (a DNS TXT or meta-tag token) could come in W3 if the owner wants it.
- **ONB-1 is not reachable yet.** No endpoint and no review UI exist until the W3 server.

## Decisions needed from owner

- **D9 — Device check before W3.** Run the checklist above (about 10 minutes) before server work starts
  (recommended: the capture path is the product), or waive it again.
- **M4 — Display serif (revisit).** Fidelity to the mockups is now limited mainly by the serif. Bundle an
  OFL-licensed condensed display serif (licence page checked first; about 50–150 KB), or keep the device serif?
- **M7 — Photo colour.** Keep untagged output (current; smallest, fastest), or convert wide-gamut photos to sRGB
  before re-encoding?
- **D4 — Hosting** (still open from W0, and needed for W3). The ZAP product name, plus the output of
  `uname -a; systemctl --version | head -1; nproc; free -h; df -h /`. Please don't send passwords or keys.

## Closure (2026-09-30)

- **W2 approved.** The owner replied *"Approved"* to this report. The evidence and labels stand as written: every
  device item stays **Specified**.
- **D9 (device check before W3): not answered, so still open.** The approval is not read as a waiver. The
  checklist above still applies. The W3 proposal asks again, because W3's sync client depends on the Android IPC
  path fixed here (finding 1).
- **M4 (display serif): default kept.** The device serif stays; no font is bundled.
- **M7 (photo colour): default kept.** Output stays untagged (no ICC).
- **D4 (hosting): still open.** Spec §7 makes it a hard gate for deploying W3.
- Next: [`W3-PROPOSAL-hosted-feedback-slice.md`](W3-PROPOSAL-hosted-feedback-slice.md), which is a proposal only.
  Nothing starts without the owner's approval.

## Erratum (found in W3a, 2026-09-30)

- **ONB-1 was labelled Built but could not read https sites.** The crate had no TLS backend: reqwest was built
  without default features, and none was added. W2's tests used only plain http, so they missed it. It was fixed
  in W3a (`21867df`), with an https regression test. See the W3a report, finding 1.
