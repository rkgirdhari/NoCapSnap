# Android CI — CapSnap APK built and kept on GitHub

Date: 2026-10-01 · Status: **Opened by the owner** (*"go build the android workflow"*), after asking where the APK
is saved.

Until now each APK (W1, W2, W3a) was built in a temporary cloud workspace and sent to the owner in chat. The gate
reports kept their checksums but not the files, and the workspace is deleted when its session ends.

## Gate report

```
## Android CI — release APK built and kept on GitHub
Status: Built (workflow and packaging script); Specified (the APK running on a phone, D9)
Evidence: android.yml run 36902688713 (commit edcd239) green in 7 min: a 14,830,222-byte test-signed APK,
          stripped, 16 KB-aligned, Play checks passing, checksum re-verified after download.
          package-android.sh also run locally on the W3a build, including the failure cases
Changes: android.yml and app/scripts/package-android.sh (new); release profile strip = "symbols"; README
Tenth Man: test builds get a new signing key every run; the repository is public, so its builds are downloadable
Decision needed from owner: none to merge. Later (W5): create the Play upload key and add four repository secrets
```

## What it does

| When | What you get | Kept |
|---|---|---|
| A PR or a push to `main` that touches the app or its crates | Artifact `capsnap-android`: `CapSnap_0.1.0_arm64_test-signed.apk` and `SHA256SUMS.txt` | 30 days |
| A tag `vX.Y.Z` that equals the app version in `tauri.conf.json` | The same build as a **GitHub Release** (a pre-release while builds are test-signed) | Permanently |
| On demand: Actions → Android → Run workflow | The same as a push | 30 days |
| On demand on `main`, with **"publish a release"** ticked | The same build as GitHub Release `v<app version>`, for when a tag can't be pushed (this workspace's git access can push branches but not tags). If the tag doesn't exist, `main` is built and GitHub creates the tag on it; if it does, the tag's commit is built and the APK is added to its release | Permanently |

**Every build fails the run if any check fails.** The checks are the ones the W1–W3a gate reports ran by hand:

- no browser preview bridge in the bundled frontend;
- `zipalign -P 16` and LOAD alignment `0x4000` (16 KB pages);
- the native library is stripped (no symbol table);
- an APK Signature Scheme v2 signature;
- package `com.hammurabicoding.nocapsnap` at the app's version;
- minSdk 24, targetSdk 36 or later, `arm64-v8a` only;
- `usesCleartextTraffic=false` and `allowBackup=false`;
- not debuggable.

**The toolchain is pinned to the one the W1–W3a APKs were built with:**

- Android platform 37.0;
- build-tools 36.1.0;
- NDK r30 (30.0.16248370);
- JDK 21.

## Signing

**Now (no secrets): a throwaway key, new on every run.**

- Android won't install an APK over one signed with a different key, so uninstall CapSnap before installing
  another build.
- Uninstalling deletes what is on the phone. Today that is demo data only.
- The W3a APK sent on 2026-10-01 was signed with yet another key, so it also has to be uninstalled first.
- No `.aab` is made, because Google Play only accepts bundles signed with the upload key.

**With the Play upload key (W5).**

1. The owner creates the key on their own PC. Any JDK has `keytool`:
   ```
   keytool -genkeypair -v -keystore capsnap-upload.jks -storetype PKCS12 -alias upload -keyalg RSA -keysize 4096 -validity 9125
   ```
   Keep the file and its passwords somewhere safe and backed up. If the key is lost, Google has to reset it.
2. The owner adds four repository secrets in GitHub → Settings → Secrets and variables → Actions:

   | Secret | Value |
   |---|---|
   | `ANDROID_UPLOAD_KEYSTORE_BASE64` | The keystore file as base64. In PowerShell: `[Convert]::ToBase64String([IO.File]::ReadAllBytes("capsnap-upload.jks"))` |
   | `ANDROID_UPLOAD_KEY_ALIAS` | `upload` |
   | `ANDROID_UPLOAD_KEYSTORE_PASSWORD` | The keystore password |
   | `ANDROID_UPLOAD_KEY_PASSWORD` | The key password. For PKCS12 it is the same as the keystore password |

3. From then on, every build signs the APK and the `.aab` with that key, and tagged releases are full releases.
   - The key is written to the runner only for the build and deleted afterwards.
   - It is never in the repository and never sent to Claude.
   - Pull requests from forks and Dependabot don't receive the secrets, so their builds stay test-signed.

## Evidence

Checked locally in the cloud workspace, against the W3a release build (unsigned output of
`pnpm tauri android build --apk --target aarch64`):

```
== CapSnap_0.1.0_arm64_test-signed.apk (12343950 bytes, signed with the throwaway test key)
Signer #1 certificate DN: CN=CapSnap test build, O=Hammurabi Coding Company LLC
zipalign -c -P 16 4: aligned
package: name='com.hammurabicoding.nocapsnap' versionCode='1000' versionName='0.1.0' … compileSdkVersion='37'
minSdkVersion:'24'
targetSdkVersion:'37'
uses-permission: INTERNET, CAMERA, (AndroidX) DYNAMIC_RECEIVER_NOT_EXPORTED_PERMISSION
native-code: 'arm64-v8a'
arm64-v8a/libcapsnap_app_lib.so: 10591744 bytes, LOAD align 0x4000
```

**The upload-key path, with a stand-in key and a stand-in bundle:**

- the APK was signed with that key;
- the `.aab` was signed with `jarsigner`, and `jar verified` came back;
- both were listed in `SHA256SUMS.txt`.

**Failure cases, each of which stops the run:**

- the preview bridge left in `build/`;
- an upload key set but no `.aab` built;
- a wrong key password.

**First run on GitHub (run 36900350649, commit `878e257`):**

- **Result:** green in 8 minutes, but the APK was 21,355,150 bytes against W3a's 12,343,950.
- **Cause:** its `libcapsnap_app_lib.so` (19,605,568 bytes) still carried its symbol table: `.symtab` plus
  `.strtab` were about 6.4 MB. The W3a library had none.
- **What is not known:** why the local builds came out stripped is not established. A small test library built
  locally with Rust 1.94 keeps `.symtab` by default, and CI ran Rust 1.99.
- **Fix:**
  - `app/src-tauri/Cargo.toml` now sets `[profile.release] strip = "symbols"`, so the result no longer depends on
    the toolchain. On a test library built with NDK r30, this removes `.symtab` and keeps the exported symbols
    Android loads.
  - `package-android.sh` now fails if a library still has `.symtab`. Run against the first CI APK, it stops with
    "still has its symbol table"; run against the W3a build, it passes.
- **Still unexplained:** CI's `.text` section is also about 1.8 MB larger (8.8 MB against 7.1 MB), which is
  consistent with the newer compiler. CI uses current stable Rust, like the other workflows.

**Second run (run 36902688713, commit `edcd239`):** green in 7 minutes.

| Check | Result |
|---|---|
| File | `CapSnap_0.1.0_arm64_test-signed.apk`, 14,830,222 bytes, sha256 `f88827649bee61cbe422979a99f0c00136b6d3acdd8d65b0c0368f07e98cd80c` |
| Native library | `libcapsnap_app_lib.so`, 13,079,352 bytes: stripped (no `.symtab`), and its 24 exported `Java_*`/`JNI_OnLoad` entry points kept |
| Alignment | LOAD `0x4000`; `zipalign -c -P 16` passes |
| Signature | apksigner verifies it; signer "CN=CapSnap test build" (a throwaway key) |
| Package | `com.hammurabicoding.nocapsnap` 0.1.0, versionCode 1000, `arm64-v8a` |
| After download | `SHA256SUMS.txt` matches; the checks above were re-run here and agree |

It is 2.5 MB larger than W3a: the library's code grew from 7.1 MB to 8.8 MB under the newer compiler (see above).

**The release step's logic, dry-run for both signing modes:**

- test-signed builds become a pre-release;
- upload-key builds become a full release with the `.aab`.

## Labels

| Item | Label | Why |
|---|---|---|
| Packaging script: align, sign, check, checksums | **Built** | Run locally on the W3a build, in both signing modes and the failure cases |
| The workflow building the APK on GitHub | **Built** | Run 36902688713, green; the APK was re-verified after download |
| The CI APK running on a phone | **Specified** | Sent to the owner on 2026-10-01; device check D9 is still open |
| Manual run → GitHub Release | **Built** | [v0.1.0](https://github.com/rkgirdhari/NoCapSnap/releases/tag/v0.1.0), run 36926943872: tag on `bfa5cf0`, pre-release, APK and `SHA256SUMS.txt` attached. The APK was re-downloaded from the release: its checksum matches, it is signed, `arm64-v8a`, and stripped |
| Pushed tag, a release made by hand, or adding to an existing release → GitHub Release | **Specified** | Not run yet for Android; the shared `.github/scripts/publish-release.sh` and the source step were dry-run locally (stub `gh`; a shallow clone of a local remote with and without the tag) |
| Upload-key signing of a real `.aab` | **Specified** | Tested with a stand-in key and bundle; needs the real key (W5) |
| Google Play accepting the bundle | **Specified** | W5 |

## Tenth Man

- **A new key on every test build.**
  - Each new build means uninstalling the old one first.
  - That's fine while there is only demo data on the phone, and it ends once the upload key exists.
  - A stable test key would avoid it, but it would have to be a secret too, so it's no simpler than the real one.
- **The repository is public.**
  - Any signed-in GitHub user can download the workflow artifacts, and anyone can download a Release.
  - The APK holds no secrets: the server address and sign-in are entered on the phone.
  - The app will be public on Google Play anyway, but a test-signed copy circulating is a copy the owner doesn't
    control.
  - Suggestion: tag only builds meant to be kept.
- **versionCode comes from the version** (0.1.0 → 1000).
  - Play rejects a second upload with the same versionCode.
  - So each Play upload needs a version bump: a W5 release step.
- **arm64 only**, as in W1–W3a.
  - This leaves out phones that only run 32-bit apps, and x86_64 Chromebooks and emulators.
  - Adding ABIs is one flag per target, at the cost of a larger APK and a longer build.
- **Build time.** About 10–20 minutes per app change. Actions minutes are free for public repositories.

## First release: v0.1.0 (2026-10-01)

- **Made by:** a manual run of the Android workflow on `main` with "publish a release", requested by the owner ("tag v0.1.0 so the APK gets kept"). The tag could not be pushed from the cloud workspace: its git access returns HTTP 403 for tags.
- **Release:** <https://github.com/rkgirdhari/NoCapSnap/releases/tag/v0.1.0>. It is a pre-release, because the build is test-signed. Tag `v0.1.0` points to `bfa5cf0`.
- **APK:** `CapSnap_0.1.0_arm64_test-signed.apk`, 14,842,510 bytes, sha256 `d83ed22ef5cf810be7a92f7bb42201d6d6b6924fca78e56a345c7aab49f866ea`.
- **Windows installers:** not in the first publish. Added later by `desktop.yml`'s release step, built from the same tagged commit (see the Desktop record).
