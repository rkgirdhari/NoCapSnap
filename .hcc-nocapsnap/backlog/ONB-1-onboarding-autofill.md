# ONB-1 — Onboarding auto-fill from public data

Status: **Specified**, approved for build (owner, 2026-09-30) as *consent-based import from the restaurant's own website*. The Google Places route stays Aspirational. Recorded 2026-09-30 from an owner-forwarded analysis that
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

## Owner decision (O1, 2026-09-30) and design

Owner: *"Build in the URL Search & Scrape for the Restaurant being requested or customer etc where they either
link it or give us permission to scrape."*

Design (a server-side Rust crate, host-tested now, wired to `POST /api/v1/onboarding/import` with the W3 server):

1. **Consent.** The requester gives the restaurant's website URL and confirms *"I own or manage this website and
   allow NO CAP SNAP to read its public pages for this setup."* The request records who consented, when, and to
   which URL.
2. **Fetch safely:**
   - Only `https`/`http` URLs on the given site's registrable domain.
   - No private, loopback or link-local IPs, checked after DNS resolution, so it can't be abused for SSRF.
   - At most 3 redirects, a response-size cap, a timeout, and a few pages at most (home page plus pages linked as
     "menu").
   - `robots.txt` is honoured; the user agent identifies itself.
   - A deny-list of aggregator and review sites (Google, Yelp, TripAdvisor and similar). Spec §1: no review data.
3. **Extract.**
   - First choice: schema.org `Restaurant` / `LocalBusiness` JSON-LD (name, address, telephone, url, cuisine,
     opening hours, `hasMenu` → `MenuSection` / `MenuItem`).
   - Fallbacks: OpenGraph, `<title>`/`<h1>`, `tel:` links, microdata.
   - No photos are imported (copyright). Raw HTML is not stored.
4. **Pre-fill only.** The result is a draft organization, location and menu for the requester to review and edit.
   Nothing is saved until they confirm.
5. **"Search".** Name-based web search would need a third-party search engine (Spec §2/§6), so it isn't built.
   Search means discovering the menu pages *within* the linked site.

