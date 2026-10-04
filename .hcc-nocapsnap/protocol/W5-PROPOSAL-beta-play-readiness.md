# W5 — Beta and Play readiness (PROPOSAL)

Date: 2026-10-04 · Status: **Specified (proposal only)**. Not opened. Nothing here can close before W3b (a live server)
and W4b (the restore drill, which lifts R1).

Spec §7 Phase 5: *"Finalize the Data Safety declarations, ensuring they reflect actual behavior (no tracking/fonts)."*
The spec's closing statement adds two blocks on public release: *"Legal approval of the publisher identity and the
physical device camera spike are absolute blocking decisions."* The camera spike is done (W1, D9). The publisher
identity is the owner's (P1).

## Gate report

```
## Gate W5 — Beta and Play readiness
Status: Specified (proposal only)
Evidence: none yet; exit evidence defined below
Changes: this file only
Tenth Man: a Data Safety form that is true today can stop being true after one later change
Decision needed from owner: open W5? plus P1–P4 below
```

## What the app does today (from the repo, 2026-10-04)

| Fact | Where |
|---|---|
| Permissions: `INTERNET`, `ACCESS_NETWORK_STATE`, `CAMERA`. Nothing else | `AndroidManifest.xml` |
| `allowBackup=false`, `fullBackupContent=false`, data-extraction rules set | `AndroidManifest.xml` |
| Target SDK 37, minimum 24 (spec says API 36+ as the target; met) | `build.gradle.kts` |
| App CSP: `default-src 'self'`, no remote scripts, no remote fonts, connect only to the app's own IPC | `tauri.conf.json` |
| Staff sign in with a login and password; a revocable device session is stored on the phone | W3a record |
| Dish photos leave the phone for the owner's own server (EXIF and GPS stripped on device) | Spec §3, W2 |
| Guests give no name, email or phone; the guest page is served by the owner's server | Spec §4, §6 |
| No analytics, no ads, no third-party SDKs, no crash reporter | CSP, Cargo and package manifests |

Spec §6 says only `CAMERA` is permitted among sensitive permissions. `INTERNET` and `ACCESS_NETWORK_STATE` are
normal permissions, needed for sync and the Online/Offline status; they should be named as such in the Data Safety
notes so the "only CAMERA" rule isn't read as a contradiction.

## Proposed scope

| Area | What | Label |
|---|---|---|
| Data Safety draft | An answer for every question in Play Console's form, each tied to a line of code or config above. Drafted by us; **submitted by the owner** | Specified |
| Truth test | A CI check that fails when the manifest gains a permission, or the CSP gains a remote origin, without the Data Safety draft being updated in the same PR | Specified |
| Privacy policy | A public page on the guest domain (served by the server, no third-party assets) covering staff data, dish photos, guest feedback, the retention periods (Spec §6) and the backup window (W4), plus a contact. The URL is required by Play | Specified |
| Account deletion | Play requires a way to request deletion of an account and data for apps with accounts. Staff accounts here are created by an admin, so the route is an in-app/privacy-policy contact path plus `capsnapctl`; the exact wording needs the owner's review | Specified |
| Signing and release | The upload key (owner-made, see the Android CI record), four repository secrets, a `versionCode` bump per upload, a signed `.aab` from CI | Specified |
| Store listing | Name, short and full description, screenshots from the real build, icon, feature graphic, content rating questionnaire, target audience (not for children) | Specified |
| Beta | Internal testing first, then closed testing with real restaurants' staff on test data until R1 lifts | Specified |

## Exit evidence

1. Data Safety draft reviewed line by line against the build by two readers: one of them reads the APK's merged
   manifest and network behaviour, not our docs.
2. The truth-test CI check exists and fails on a deliberate permission addition.
3. The privacy policy is live on the guest domain, linked from the app's Settings and the store listing.
4. A signed `.aab` is built by CI and accepted by Play Console on the internal track; the owner installs it from
   Play, not a sideloaded APK, and runs the D9 checklist again, including the step still marked "not reported"
   (back button and camera indicator).
5. Closed-testing feedback from at least one real restaurant, on test data.

## Tenth Man

- **A true form goes stale.** The Data Safety answers describe the app at one moment. The CI check narrows the gap
  (permissions, origins) but cannot see a new field the server starts collecting; that stays a review item for every
  PR that touches the data model.
- **"No tracking" is a claim about the owner's own server too.** Retention is enforced by code (Spec §6), but backups
  keep 30 days (W4a). The privacy policy must say so; leaving it out would make the policy false.
- **Publisher identity is the long pole.** An organization account needs the LLC's legal details and a D-U-N-S
  number; verification takes days to weeks and is the owner's. Everything else can wait for it, so P1 should start now.
- **Play's testing rules change.** New developer accounts may be required to run a closed test with a minimum number
  of testers for a minimum period before production. Whether that applies to an organization account must be
  checked against Play Console's current help pages when W5 opens. This proposal does not assert the numbers.
- **A signed build in a store is hard to undo.** A `versionCode` is spent forever, and a lost upload key needs
  Google to reset it.

## Decisions needed from owner

- **W5 — Open it?** Recommended: open the parts that need no server now (Data Safety draft, truth test, privacy
  policy text, listing assets); hold signing and beta until W3b and W4b are done.
- **P1 — Publisher identity.** Register the Play developer account as **Hammurabi Coding Company LLC**
  (organization) and start D-U-N-S verification. The spec makes this a blocking decision.
- **P2 — Privacy policy owner.** Who is the contact named in it, and is a legal review wanted before it goes live?
- **P3 — Distribution.** Public on Play, or private distribution to invited restaurants only?
- **P4 — Beta group.** Which restaurant(s) will test, and on which phones?

## Labels

| Item | Label |
|---|---|
| Everything in this file | **Specified**: nothing built |
