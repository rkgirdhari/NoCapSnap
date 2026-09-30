# W2 — PROPOSAL: staff UI from the owner's mockups (not approved)

Date: 2026-09-30 · Status: **Proposed**. Nothing here is built. W2 needs explicit owner approval (protocol rule 1).

**Input.** Five high-fidelity mockups from the owner, titled "NO CAP SNAP":
1. Home
2. Capture 01/03 "Which dish?"
3. Capture 02/03 "The plate, as served."
4. History while offline
5. Guest invitation / QR

They were shown inline in chat and are **not saved in the repo or the sandbox**. The owner should commit them
(for example `nocapsnap/.hcc-nocapsnap/design/mockups/`) so later gates can cite them. Colours below are visual
estimates; they could not be sampled.

## Gate report

```
## Gate W2 — Local vertical slice (proposed)
Status: Specified (this document) — nothing built
Evidence: none yet; W2 exit evidence is defined below
Changes: this file only
Tenth Man: building this UI before the device camera check (D7) contradicts Spec §7's hard gate
Decision needed from owner: approve W2 with these mockups as the target, and answer M1–M6
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

## Visual deltas from the current W1 build (estimated from the images)

- **Wordmark:** "NO CAP SNAP" in widely tracked capitals, replacing the "CS" seal plus "CapSnap".
- **Headlines:** a high-contrast display serif in a cream-to-gold tone, larger and tighter than the current system
  serif.
- **Base colour:** a deeper aubergine than `#2D1B4E`.
- **Cards:** a desaturated plum-brown rather than `#744210`.
- **CTA:** a more saturated yellow than `#F6E05E`.
- **Chips:** carry icons (hourglass, QR) and warm amber for "waiting".
- **Photography:** real dish photos everywhere (hero, list thumbnails, dish cards).
- **Tabs:** Home, History, Insights, Settings. Capture moves to the primary CTA; Manager is folded into Settings.

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
- **M3 — Canonical colours.** Keep the brand-sheet hexes (contrast-verified in W1), or send the mockup source file
  or exact hexes so they can be measured?
- **M4 — Display serif.** Bundle an OFL display serif to match the mockups (licence and size checked first), or
  stay with the phone's system serif?
- **M5 — Status wording.** The mockup chips say "Waiting to sync". Spec §3 requires "saved offline / QR not ready"
  to be explicit. Proposal: keep the mockup chip and banner, but the chip reads "Saved offline · QR not ready" to
  satisfy the spec.
- **M6 — Table label.** Device-only (staff reference, never synced) or synced to the server as staff metadata? It
  is never shown to guests either way.
