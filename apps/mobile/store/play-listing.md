# Google Play listing — CapSnap

App name: CapSnap
Package: com.hammurabicoding.nocapsnap
Developer: Hammurabi Coding Company LLC
Contact: R. K. Girdhari

## Short description (80 chars max)

Snap the real dish before it hits the table. Reviews tied to that plate.

## Full description

CapSnap lets restaurant staff photograph the actual dish as it leaves the pass,
stamp it with restaurant, dish, table, and time, then send that image to the guest
as the review. No menu fantasy. No random phone pics. Proof of the plate.

## Classification

Category: Business
Content rating: Everyone (complete the IARC questionnaire in Play Console)
Privacy: staff accounts only; guest review links are tokenized.

## Play Console items this app will need

- Privacy policy URL — required because the app uses the camera.
- Data safety form — declare: photos (collected, shared with the guest via
  review link), email address (staff login), and that data is encrypted in transit.
- App access — provide a demo staff login for Google's reviewers.
- Target API level — the app targets Android 16 (API 36) via Expo SDK 57, which
  meets the requirement in force since Aug 31, 2026.

## Android permissions requested

- CAMERA — capture the dish photo.
- INTERNET — upload the photo to the CapSnap API.

RECORD_AUDIO, storage and overlay permissions are explicitly blocked in
`app.json` so they don't show up in the Play listing or Data safety review.
