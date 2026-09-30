# ONB-1 — Onboarding auto-fill from public data

Status: **Aspirational**. Not scheduled in any gate. Recorded 2026-09-30 from an owner-forwarded analysis that
proposes Google Places API search, suggestion and pre-fill for new restaurants, plus a blank menu template.

## Where the analysis stands against the spec

| Spec | Conflict |
|---|---|
| §2 "self-hosted first mandate … minimizes recurring operational costs and third-party account risks"; "explicitly rejects … managed cloud services" | Google Places is a managed third-party service with its own billing account and API key |
| §6 Asset isolation: prevent "leakage to third-party providers" | Every onboarding search is sent to Google |
| §1 Strategic independence from Google Reviews and aggregators | Adds a Google dependency to the onboarding path |

Adopting Places would need an explicit **spec amendment** by the owner. It can't be done as an implementation
detail.

## Claims checked against Google's own pages (2026-09-30)

- **"$200/month free tier": outdated.** Google's billing FAQ: *"we are modifying the USD $200 monthly recurring
  credit by offering a free monthly usage threshold for each Core Services SKU"*, effective **1 March 2025**
  (Essentials 10,000, Pro 5,000, Enterprise 1,000 free events per SKU per month). The per-request prices in the
  analysis weren't verified; read them from Google's current SKU price list before any costing.
- **"Cache 30 days max": imprecise.** The Places API policies say place IDs are *"exempt from"* caching
  restrictions and may be stored indefinitely. All other content (names, addresses, phone numbers, websites,
  hours) *cannot be cached* beyond the Terms of Service exceptions. Storing Google-sourced phone or website values
  in `organizations` needs legal review, even when a user confirms them.
- **Attribution and terms.** Displaying Places content requires attribution, and apps must *"provide publicly
  accessible Terms of Use and a Privacy Policy that incorporate Google's Terms of Service and Privacy Policy"*.
  That would bind CapSnap's own terms to Google's.

## Options that fit the spec

1. **Manual onboarding (launch).** A short form: organization, location name, address, IANA timezone (prefilled
   from the device's `Intl` timezone, then editable), plus a blank menu template. The forwarded analysis
   recommends the same for the first five restaurants.
2. **Self-hosted lookup (later).** An OpenStreetMap geocoder (Photon or Nominatim) on the owner's VPS. The data is
   ODbL: attribution is required, and there are share-alike terms for derived databases. Disk and RAM needs depend
   on the region extract, so this waits on D4 (the verified VPS specs).
3. **Google Places.** Only after a spec amendment, a legal review of the terms, and a current price check.

The blank menu template (for example "Starter 1 … Dessert 1") needs no third party and can ship with option 1
in W3.

## Decision needed from owner

**O1:** Launch with manual onboarding (option 1) and keep auto-fill Aspirational? Or amend the spec to allow a
third-party lookup (option 3)?
