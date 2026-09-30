# W2 — Staff UI from the owner's mockups (local vertical slice)

Date: 2026-09-30 · Status: **Approved by the owner, in progress** (see "Owner decisions").

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
Status: In progress — approved 2026-09-30 with the D7 device check waived by the owner
Evidence: none yet; W2 exit evidence is defined below
Changes: this file only
Tenth Man: building this UI before the device camera check (D7) contradicts Spec §7's hard gate
Decision needed from owner: M4–M6 still open (defaults applied, see below)
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

## Tenth Man

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

## Decisions needed from owner

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

