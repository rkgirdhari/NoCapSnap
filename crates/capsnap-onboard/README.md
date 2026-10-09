# capsnap-onboard (ONB-1)

Consent-based import from a restaurant's **own** website into a draft
organization, location and menu (owner decision O1). This is a server-side crate.
The W3 server exposes it as `POST /api/v1/onboarding/import`. Until then it is
tested on the host only.

- **Consent first.** The requester must accept `CONSENT_STATEMENT` for exactly
  the URL being read. Otherwise nothing is fetched. The consent comes back
  with the draft so the server can record it.
- **Own site only.** Review, delivery, social and search sites are refused
  (Spec §1). The importer stays on the given site (its host without `www.`,
  plus subdomains), and every redirect hop is re-checked. It follows at most
  3 redirects and reads at most 4 pages.
- **No SSRF.** The crate resolves DNS itself and hands the HTTP client only
  globally routable addresses. Loopback, private ranges, link-local and cloud
  metadata, CGNAT, ULA, NAT64 and 6to4 are all refused. No proxy is used, and
  IP-literal URLs are refused.
- **Polite.** robots.txt is honoured per RFC 9309: our group, else `*`;
  longest match wins; if robots.txt is unreachable, nothing is read. The user
  agent is `NoCapSnapBot`. Each request has a 10 s timeout and a 2 MiB cap.
- **Extract.** schema.org JSON-LD `Restaurant`/`Menu` first, then microdata,
  then OpenGraph, `<title>`/`<h1>` and `tel:` links. Photos are never
  fetched. Raw HTML is dropped after parsing.
- **Plain-text menus.** With no structured menu, the page text is read
  (`plain.rs`): course headings, then one line per dish ending in a price
  (`$32`, `32.00`, `€14,50`, `Short rib .... 32`, or the price on its own
  line), a `<br>` ends a line. A page is believed only with 3 dishes if it
  says "menu" in its address, title or main heading, otherwise 6. A menu page
  that prints no prices is read by its headings, and only with at least 6
  dishes under 2 headings. Charges, notices, party-room and contact sections
  and leaked markup are skipped. The draft says these dishes came from text.
- **Sorted.** The draft lists courses in the order a diner reads them
  (breakfast, starters, soups and salads, mains, sides, desserts, drinks); the
  site's own order is kept within a course and among courses we don't
  recognise (`courses.rs`, English keywords).
- **Not read.** Menus drawn by JavaScript (for example Dave & Buster's) or
  held in images or PDFs are not in the HTML we fetch; the draft says no menu
  was found and the owner adds dishes by hand.
- **Draft only.** Nothing is saved until the requester reviews and confirms.
  There is no third-party name search (Spec §2, §6).

Host tests: `scripts/test-host.sh`. They use an in-process HTTP server on
127.0.0.1, with test host names pinned to it.
