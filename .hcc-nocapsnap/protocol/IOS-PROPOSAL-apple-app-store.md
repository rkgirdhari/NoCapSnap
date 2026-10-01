# iOS — CapSnap on the Apple App Store (PROPOSAL)

Date: 2026-10-01 · Status: **Proposal only.** On 2026-10-01 the owner approved adding iOS to the plan as a later gate
(*"Approved and proceed"*, answering whether to add it). Opening it needs a separate approval.

**When:** after W4 (backups and restore drill). Android goes first, in the spec's order: W5 Play readiness, then W6
release. iOS can then run alongside W5.

**Spec deviation:** Spec §7 is Android-first, and its Phase 7 is Windows; iOS is not in it. This proposal adds a
platform. It changes neither the server nor the guest page, and the Spec §6 privacy rules apply to iOS unchanged.

## Gate report

```
## Gate iOS — Apple App Store (proposal)
Status: Specified (proposal only; nothing built)
Evidence: none yet; exit evidence defined below
Changes: this file only
Tenth Man: adds a second phone platform while the first has never run on a real device (D9)
Decision needed from owner: I1–I4 below, and approval to open this gate after W4
```

## What carries over unchanged

| Part | Why it should work on iOS | Label |
|---|---|---|
| UI: SvelteKit screens, design tokens | Runs in the iOS WebView (WKWebView) as it does in Android's | Specified |
| Notches and the home indicator | `app.css` already pads with `env(safe-area-inset-*)` | Specified |
| Photo hand-off to Rust | `bridge/device.ts` sends raw bytes everywhere except Android (base64 there) | Specified |
| Rust core: `capsnap-store`, `capsnap-sync` | No Android-only code. SQLite is bundled, TLS is rustls, images are pure Rust | Specified until it compiles for `aarch64-apple-ios` |
| Tauri app shell | The same `mobile_entry_point`, and no Tauri plugins to port | Specified |
| Server, guest portal, deploy kit | Untouched; the QR codes are the same | Built (W3a/W3b) |

## What is new for iOS

1. **Project:** `pnpm tauri ios init` generates the Xcode project. The bundle ID stays `com.hammurabicoding.nocapsnap`.
   Recommended minimum: iOS 16 (see I4).
2. **Camera:**
   - an `NSCameraUsageDescription` text, which Apple shows in the permission prompt;
   - both W1 routes tested on an iPhone: the in-app viewfinder (`getUserMedia`, available in WKWebView since iOS 14.3)
     and the phone's own camera (`<input capture>`).
3. **No backups of app data** (the iOS version of Spec §6's "Android backup disabled"): exclude the database and
   photos from iCloud and computer backups (`isExcludedFromBackup`). That is a small native call; it is the only
   expected native code.
4. **Sync timing:** iOS suspends apps in the background sooner than Android. Sync must run when the app opens and
   comes back to the foreground, not only on the 60-second timer.
5. **App icons and launch screen** from the existing icon set.
6. **Store paperwork:**
   - the App Privacy label: staff sign-in details and dish photos, used only to run the app, no tracking;
   - a privacy policy URL;
   - the export-compliance answer: HTTPS only, standard encryption;
   - review notes and a demo account on the live server.

## Build machine

iOS apps can only be built on macOS with Xcode. This cloud workspace is Linux, so it cannot build or sign them.
Options (I2):

- **The owner's Mac:** simplest. Xcode is free.
- **GitHub Actions macOS runners:** the same CI pattern as the server (build, test, upload to TestFlight). Signing
  keys and the App Store Connect API key would live in GitHub secrets. On a private repository, macOS minutes count
  at a higher rate than Linux against the plan's allowance, so check the cost first.

## Exit evidence

- Builds for an iPhone and for the simulator.
- Clippy is clean for `aarch64-apple-ios`.
- **A device checklist on a real iPhone:** both camera routes, a capture saved offline, sync against
  `nocapsnap.hammurabi.click`, and the guest QR scanned by a second phone.
- A TestFlight build installed by the owner.
- App Privacy answers match what the app actually does, and the backup exclusion has been verified.

## Tenth Man

- **The first phone platform has never run on a phone.** D9 is still open on Android. Adding iOS before Android has
  passed a device check doubles the untested surface. That is why this proposal waits for W4, by which time D9
  should be closed.
- **Camera behaviour in iOS's WebView can surprise.** Permission prompts and live-camera quirks differ from Android.
  The phone's-own-camera route is the fallback, as it was on Android in W1.
- **Apple review of a staff-only app.** Reviewers need a working account and a reason the app is useful. Without a
  live server and demo data it will be rejected. Private distribution (I3) avoids public review for invited
  restaurants.
- **Cost and upkeep:** $99 a year, a Mac (or macOS CI minutes), and two store listings to keep current, run by one
  person.

## Decisions needed from owner

- **I1, the Apple Developer account.**
  - As **Hammurabi Coding Company LLC** (organization; needs a D-U-N-S number, the same one Google Play's organization
    account uses), or personal.
  - Recommended: the LLC, matching the publisher identity the spec requires.
- **I2, the build machine.** Your Mac, or GitHub macOS runners.
- **I3, distribution.**
  - The public App Store, which any restaurant can download, or **private distribution through Apple Business
    Manager** (Custom Apps, only for businesses you invite).
  - Recommended: public, so new restaurants can find and install it, with TestFlight first.
- **I4, minimum iOS version.** Recommended: iOS 16. That covers iPhones from 2017 on (iPhone 8 and later) and keeps
  camera behaviour predictable.
- **Open the gate after W4?** Recommended: yes, alongside W5.
