# W5b — Google Play store listing (DRAFT for the owner)

Date: 2026-10-09 · Part of W5 ([proposal](W5-PROPOSAL-beta-play-readiness.md)); companion to the
[Data Safety draft](W5a-data-safety-draft.md). **The owner enters all of this in Play Console.** Limits quoted
here (30 / 80 / 4000 characters, image sizes) are from memory of Play's rules; check them against the live form.

Labels: **Built** = exists in the repo; **Specified** = draft text or a list, nothing entered anywhere.

## Gate report

```
## Gate W5b — Store listing draft
Status: Specified (text and checklists); Built (icon source, mockups)
Evidence: this file; the app's behaviour as of v0.1.3 is the source for every claim in the text
Changes: this file; privacy policy section on menu import (see W5a gap 5)
Tenth Man: a listing that promises more than the app does is the likeliest reason for a rejection or a bad first review
Decision needed from owner: C1 (consent record retention), P2 (the contact), P3 (public or private)
```

## Text

| Field | Draft | Length |
|---|---|---|
| App name | CapSnap: Dish Feedback | 22 / 30 |
| Short description | Photograph a dish, share a QR code, get private guest feedback. | 63 / 80 |
| Category | Business | |
| Tags | Business, Restaurants (pick from Play's list) | |
| Contact email | **P2**: the owner's choice; Play shows it publicly | |
| Website | https://nocapsnap.hammurabi.click | |
| Privacy policy | https://nocapsnap.hammurabi.click/privacy | live since 2026-10-06 |

### Full description

```
CapSnap is for restaurant staff. Photograph the dish you prepared, and the guest who ate it can tell the restaurant,
privately, what they thought of that exact plate.

How it works
• Choose the dish and take the photo.
• CapSnap shows a QR code. The guest scans it with their own phone. No app, no account, no name, no email.
• The guest gives a rating from 1 to 5 and, if they like, a comment about that dish.
• The answer goes only to the restaurant, for internal quality control. Nothing is posted publicly, and every guest
  sees the same neutral form whatever their rating.

Built for a busy kitchen
• Works without a connection. The photo is saved on the phone first and syncs when the signal returns.
• Photos are resized on the phone, and their location and camera data are removed before they leave it.
• Administrators and managers can draft a menu from the restaurant's own website, check it, and save it.
• Administrators and managers read what guests said in the Insights tab: an average, how many gave each rating, and
  every answer, newest first.
• A lost phone can be signed out remotely.

Private by design
• No ads, no analytics, no trackers, no third-party services in the app.
• The app asks for the camera only while you take a photo.
• Photos are kept 30 days on the restaurant's server, guest feedback 12 months. Read the full policy in the app's
  Settings.

CapSnap connects to a CapSnap server run for your restaurant. Staff accounts are created by the restaurant's
administrator; there is no public sign-up. Made by Hammurabi Coding Company LLC.
```

Checked against the build: every sentence maps to something **Built** (capture and offline outbox: W2; sync and QR:
W3a; guest page: W3a; metadata stripping: W2; menu from a website: M1; session revocation: W3a;
retention: `retention.rs`; no trackers: Data Safety draft). **Left out on purpose:**

- *Who may read feedback.* The Insights tab shows guest answers to administrators and managers only (decision F1).
  The description says so; do not imply every staff member sees ratings.
- *iOS, Windows and tablets.* The Windows installer exists but is not part of this listing.
- *"Secure", "compliant", "GDPR".* Nothing here has been independently reviewed (Data Safety draft: no review).

## Declarations to answer in Play Console

| Form | Suggested answer | Why |
|---|---|---|
| Target audience | 18 and over; not designed for children | Staff tool; no child appeal |
| Content rating (IARC) | Expect "Everyone". Answer the user-generated-content question carefully: guests' comments reach the restaurant only, never other users | Spec §4 |
| Ads | None | No ad SDK |
| App access | **Needed.** Play reviewers cannot sign in without a staff login. Give them a test restaurant and login on a test server, with test data only | Staff accounts are admin-made; sign-in is required to sync |
| Government / news / health / finance app | No to each | |
| Data safety | See the W5a draft | |
| Privacy policy URL | The link above | |

**App access is the one most likely to be missed.** Make a throwaway restaurant on the live server for the reviewers
(`capsnapctl create-org`, `create-location`, `create-staff`), put its login in the Play Console form, and delete it
after review. It must stay test data (R1).

## Assets

| Asset | Requirement (check live) | Status |
|---|---|---|
| App icon | 512 × 512 PNG | **Built**: `app/src-tauri/icons/icon.png` is 512 × 512; `app-icon.png` is the 1024 source. Check it on Play's light and dark backgrounds |
| Feature graphic | 1024 × 500 PNG or JPEG | **Specified**: not made |
| Phone screenshots | 2 to 8, real build, portrait | **Specified**: take them from the signed build on a phone or emulator. The mockups in `.hcc-nocapsnap/design/mockups/` are design art, not screenshots; do not upload them as such |
| Short video (optional) | A YouTube link | Skip for the first release |

Suggested screenshots, in order: home; choose the dish; review the plate; history showing the offline state; the QR
screen ("Ready to share"); Settings with the privacy link. Show only test data and no real names.

## Release mechanics

| Item | State |
|---|---|
| `versionCode` | **Built**: Tauri derives it from the app version (0.1.3 gives 1003, as 0.1.0 gave 1000). Play rejects a repeat, so every upload needs a higher app version in `tauri.conf.json`, `Cargo.toml` and `package.json`. This breaks at patch 1000, which is far off |
| Upload key and four secrets | **Owner**: until set, CI builds no `.aab` |
| Release notes | Per upload, one short paragraph. Draft for the first internal test: "First test build: sign in, photograph a dish, show the guest a QR code." |
| Track order | Internal testing, then closed testing; production only after the restore drill (R1) and P1 verification |

## Tenth Man

- **The listing can outrun the app.** The full description is only true while the Built list above is. Re-read it
  when a feature is removed or changed, the way the Data Safety check re-reads permissions.
- **Reviewers see a login wall.** Without working test credentials in App access, expect a rejection that has nothing
  to do with the code.
- **A public listing invites strangers to install an app that needs a server they do not have.** P3 (public or
  private distribution) is the real fix: private distribution to invited restaurants avoids a stream of one-star
  reviews from people who cannot sign in.
- **Limits drift.** Character and image limits are quoted from memory; Play changes them.
