# Desktop — CapSnap staff app for Windows

Date: 2026-10-01 · Status: **Opened by the owner** (*"keep what we got, just build the tauri desktop"*).
The owner works on Windows and wants the app there now. This brings Spec §7 Phase 7 ("Windows 11 Target: defer desktop
packaging until mobile stability is verified") forward, before mobile has been verified on a device (D9).

## Gate report

```
## Gate Desktop — Windows build of the staff app
Status: Built (installers); Specified (running on the owner's PC)
Evidence: desktop.yml run 36887773722 on windows-2022, green in 8 min → .msi + setup .exe with SHA256SUMS
Changes: desktop.yml (new); tauri.conf.json window opens centred, 412×860, resizable down to 360×560
Tenth Man: the installers are unsigned, so Windows SmartScreen will warn; and the app has no server to sync with yet
Decision needed from owner: none to build. Run the installer and report what you see
```

## What it is

- **The same app, not a new one.** Same Rust core (store, sync), same screens, same locked-down permissions (core IPC
  only).
- **Installers:**
  - `CapSnap_0.1.0_x64_en-US.msi`, the standard Windows installer;
  - `CapSnap_0.1.0_x64-setup.exe`, a smaller installer that can fetch WebView2 if it's missing (Windows 10 and 11
    normally have it).
- **Where data lives:** `%APPDATA%\com.hammurabicoding.nocapsnap` (database and photos), separate from Android.
- **Camera:** the in-app viewfinder uses the PC's webcam through WebView2, and Windows asks permission first. The
  "phone's own camera" route becomes a file picker, so photos can come from disk.
- **Demo mode** works offline at once. Sign-in and sync need a CapSnap server, and none is live yet (W3b).

## Facts that changed today (recorded for W3b)

- **The VPS (vServer #609469) runs Windows Server 2016,** not Linux as the deploy documents assumed. It serves the
  company site through IIS on port 80.
- **The W3b Linux deploy kit can't run on it as-is.** The owner has not chosen between reinstalling Linux, a second
  VPS, and a Windows deploy route. W3b deployment is paused, and all W3b code stays as built.
- **Windows Server 2016 reaches end of extended support on 12 January 2027.**
- **ZAP maintenance on that VPS:** 7 October, 05:00–11:00.

## Labels

| Item | Label | Why |
|---|---|---|
| Windows installers built from this repo | **Built** | `desktop.yml` run 36887773722 (commit `07b1873`), green; checksums re-verified after download |
| The app running on the owner's Windows PC | **Specified** | Nobody has run it yet |
| Webcam capture in WebView2 | **Specified** | Untested on Windows |
| Code-signed installers (no SmartScreen warning) | **Aspirational** | Needs a code-signing certificate, which is a paid purchase |

## Tenth Man

- **Desktop before a single phone test.** The spec puts desktop last for a reason: the product is a phone app for
  restaurant floors. A desktop build is great for demos and for the owner to try the screens, but it proves nothing
  about Android (D9).
- **Unsigned installers.** Windows will say "Windows protected your PC". Click **More info → Run anyway**. Fine for the
  owner; not acceptable for customers.
- **The UI is phone-shaped.** It runs in a narrow window. A layout designed for desktop is not part of this.

## First build (2026-10-01)

| File | Size | SHA-256 |
|---|---|---|
| `CapSnap_0.1.0_x64-setup.exe` | 4.3 MB | `148c42a3e8efae6136f4ba974283785b1f720a7f053b6ebfa287a26bbe174a33` |
| `CapSnap_0.1.0_x64_en-US.msi` | 6.1 MB | `24333c1466809cebbcecdad0ba3a6db6f79ad2f988cbed1618c9978283a4d368` |

- The run's other checks were all green: the Rust crates, the app UI and Rust shell, and the dependency audit.
- No browser-preview code was found in the release build.
- The setup .exe was sent to the owner on 2026-10-01. The run keeps both files for 30 days.
