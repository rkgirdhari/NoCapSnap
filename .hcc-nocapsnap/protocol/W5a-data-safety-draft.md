# W5a — Data Safety draft and privacy policy (DRAFT for the owner)

Date: 2026-10-04 · Opened by the owner (*"Open W5, no-server parts"*, 2026-10-04); see
[`W5-PROPOSAL-beta-play-readiness.md`](W5-PROPOSAL-beta-play-readiness.md). **The owner submits the Play form.** This file
is what to enter, and why each answer is true. It describes the app as of v0.1.2 and the server in this repository.

Labels: **Built** = in the repo and tested; **Specified** = a draft answer, not yet entered in Play Console.

## Gate report

```
## Gate W5a — Data Safety draft and privacy policy
Status: Built (privacy page, guard check), Specified (the draft answers)
Evidence: server/tests/privacy.rs; scripts/check-data-safety.sh, run in CI as "Data Safety declarations"
Changes: /privacy page; CAPSNAP_PRIVACY_CONTACT setting; the check and its declared list; this file
Tenth Man: the check sees permissions, features and CSP origins, not new data fields the server starts collecting
Decision needed from owner: P1–P4 in the W5 proposal, and the contact for /privacy (P2)
```

## Draft answers for Play's Data Safety form

Play asks about data the **app collects or shares**. Data processed only on the device, or sent to the developer's own
server to run the app, is still "collected". Sent to a service provider acting for the developer is "not shared", but the
form's wording changes, so re-read it when entering.

| Question | Draft answer | Basis |
|---|---|---|
| Does the app collect or share user data? | **Yes** (staff data and photos go to the restaurant's own CapSnap server) | W3a sync |
| Is all data encrypted in transit? | **Yes** | `crates/capsnap-sync` accepts only an `https://` server address (plain http to localhost only); release builds set `usesCleartextTraffic=false` |
| Can users request data deletion? | **Yes**, through the contact on `/privacy` (see Gap 1) | `/privacy` |
| Personal info: **name** (staff display name) | Collected; not shared; required; purpose: app functionality, account management | `staff` table |
| Personal info: **user IDs** (staff login) | Collected; not shared; required; app functionality, account management | `staff` table |
| Password | Not a Data Safety category; stored only as an argon2id hash | `password.rs` |
| Photos and videos: **photos** (dish photos) | Collected; not shared; required; app functionality | Spec §3; no people are the subject, but a photo could include one. Declare as photos |
| Other: **restaurant website address** (typed by an admin to draft a menu) | Collected with the admin's confirmation that they own the site; not shared; optional; app functionality. The server, not the phone, reads the site | `onboarding_consents`, M1 |
| Other user content: **guest comment** | Collected by the guest page, not by the app; list under the web page's policy | Spec §4 |
| Location, contacts, calendar, messages, audio, health, financial, web history, files, device IDs, app activity | **Not collected** | Manifest has no such permission; no analytics |
| Advertising ID, analytics, crash logs sent off-device | **None** | No third-party SDK; CSP blocks remote hosts |
| Data shared with third parties | **None** | Only the restaurant's own server |
| Security practices: data deletion mechanism | Yes | Retention job (`retention.rs`) plus requests |
| Independent security review | No | Not done; do not claim it |

**Permissions to declare in the listing notes:** `CAMERA` (taking a dish photo, requested only at the moment of use),
`INTERNET` and `ACCESS_NETWORK_STATE` (sending photos; showing Online/Offline). These last two are normal permissions and
sensitive data is not involved; Spec §6's "only CAMERA" is about sensitive permissions.

## What is built in this slice

| Item | Label | Evidence |
|---|---|---|
| `GET /privacy` on the server: the policy, no scripts, no outside references, retention periods matching the code, the contact escaped | **Built** | `server/tests/privacy.rs` |
| `CAPSNAP_PRIVACY_CONTACT` setting (in `deploy/capsnap.env.in`, commented out) | **Built** | `server/src/config.rs` |
| `scripts/check-data-safety.sh` + `declared.txt` + the CI job | **Built** | Fails on an added permission (shown when written), passes on the current app |
| The draft answers above | **Specified** | Not entered in Play Console |

## Gaps (found while drafting)

1. **Deleting a staff account: built.** `capsnapctl remove-staff <login>` anonymises the account (login and display name
   replaced, password replaced by a hash of a random value, deactivated, sessions and location links deleted). The
   captures and consent records that point at the row stay, as the restaurant's records; they now show "Former staff".
   The ONB-1 consent record keeps the site URL and acceptance time. **Built**, `server/tests/remove_staff.rs`.
2. **Link to the policy in the app: built.** Settings has "Read the privacy policy" (enabled once signed in). It calls
   one Rust command, `open_privacy_policy`, which builds `<signed-in server>/privacy` (https, or http only to this
   phone or the emulator host) and opens it in the phone's browser through `tauri-plugin-opener`. The plugin is
   registered but **no opener permission is granted**: the capability file stays `core:default`, so the WebView cannot
   open any address itself. The Play store listing link still waits on P1 and the live domain. **Built**; the address
   check is unit-tested, opening the browser is untested until the next device check.
5. **Menu import was missing from the policy (found 2026-10-09, fixed in W5b).** v0.1.3 lets an admin send a website
   address; the server fetches it and stores a consent record (address, statement, staff member, time). The policy did
   not say so. It now has a section, and `server/tests/privacy.rs` checks it. **Owner decision (C1):** those consent
   records have no retention job, so they are kept while the account exists. Keep that, or add a period?
3. **The contact is not set.** Until P2 is answered, `/privacy` says so in plain words.
4. **Server-side retention does not cover backups.** The policy says backups keep up to 30 days (W4a). That is only true
   once the backup timer is on; if backups are never switched on, that line is harmless but inaccurate.

## Tenth Man

- The CI check compares **permissions, features and CSP origins**. A server change that starts collecting a new field
  (say, a guest email) passes it. That stays a review item for every PR that touches the data model.
- The policy and the retention code are separate; `privacy.rs` checks the key periods appear, but not that they equal the
  constants. A change to `retention.rs` needs the page changed too.
- I have not checked Play Console's current form wording; entering it is the owner's step and should be done against
  the live form.
