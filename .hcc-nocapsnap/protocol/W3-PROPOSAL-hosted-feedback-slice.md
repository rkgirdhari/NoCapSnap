# W3 — Hosted feedback slice (PROPOSAL)

Date: 2026-09-30 · Status: **W3a opened** (*"Approved, open W3a"*) **and approved as built** (*"approved"*), both
2026-09-30; see [`W3a-hosted-feedback-slice-build.md`](W3a-hosted-feedback-slice-build.md).
**W3b opened** by the owner (*"Approved"*, 2026-09-30) after the D4 intake below. G1 (the guest domain) is still
open, so the W3b kit takes the domain as a parameter.
W3b is still waiting on D4, G1 and G2. D9 and R1 are unanswered; R1's recommendation (test data only) applies.

Spec §7 Phase 3: *"Deploy the Axum/SQLite backend to a verified vServer and implement the fragment-based guest
portal."* Spec §7's runbook makes the hosting check a **hard gate**: *"Axum requires a long-running Linux
process."* That is decision D4, still open. So W3 is proposed in two halves:

- **W3a — build and host-test.** Server, guest portal and app sync, run against a server on this machine.
  Needs no hosting.
- **W3b — deploy.** The same build on the verified vServer, with TLS and the real domain. Needs D4, G1 and G2.

## Gate report

```
## Gate W3 — Hosted feedback slice
Status: Specified (proposal only)
Evidence: none yet; exit evidence defined below
Changes: this file only
Tenth Man: W3 is the first code that moves photos off the phone, and the phone path is still unproven (D9)
Decision needed from owner: open W3a? plus D4, G1, G2, D9, R1 below
```

## Proposed scope

### Server (`nocapsnap/server/`, Rust + Axum + SQLite via sqlx, one binary, systemd)

| Area | What | Spec |
|---|---|---|
| Schema | See the entity list below this table | §5 |
| Routes | `GET /health`; `POST /auth/sessions` (rate-limited); `POST /media` (streamed); `POST /captures` (idempotent on `client_id`); `GET /locations/{id}/menu-items`; `POST /guest/session`; `POST /guest/feedback`; `POST /onboarding/import` (ONB-1 crate) | §5 route map |
| Tenancy | The organization, location and role come **only** from the session. IDs in request bodies are ignored or refused | §5 |
| Media | The server checks the file signature, MIME type and SHA-256, and that the image **decodes**, before it accepts anything. It also re-strips metadata | §5, §3 |
| Acknowledgement | A capture is "synced" only after its media has been validated. Only then is a guest link issued | §3 sync protocol |
| Guest tokens | 256-bit random from the OS generator. **Only a hash is stored.** One use; 30-day expiry; never logged | §4, §6 |
| Guest portal | A static page at `/g/` served by Axum. It reads the `#token`, calls `POST /guest/session`, then `history.replaceState`. The same neutral 1–5 rating and optional comment for everyone (≤ 2k characters). No review gating, no third-party assets, strict CSP | §4, §6 |
| Retention jobs | Originals 30 days after sync. Links 30 days or first submission. Logs 30 days, redacted. Feedback 12 months, then deleted | §6 |
| Bootstrap | The first organization and admin are created with a server command-line tool. There is no public sign-up | §5 |

Schema entities:

- organizations
- locations, with an IANA timezone
- staff, with an argon2id password hash and a role
- revocable device sessions, stored hashed
- media assets
- captures
- menu items
- guest links, stored hashed
- guest feedback

### App

- **Sign in.** Staff credentials are exchanged for a revocable device session (Spec runbook: "Device lost").
- **Sync.** A sync worker in Rust, not in the WebView, runs outbox → `POST /media` (the processed file) →
  `POST /captures` → mark synced and store the guest link.
  - It retries with backoff.
  - "Last synced" becomes real, and **Retry becomes live**.
- **Real guest QR** for synced captures: `https://<G1 domain>/g/#<token>`, error correction H, in the mockup
  styling. It replaces the concept QR.
- **Menu.** The server's menu replaces the demo seed (`source = server`), and the "Demo" tags disappear with it.

### Exit evidence

- **Server integration tests:**
  - Cross-tenant reads and writes are refused.
  - Capture retries are idempotent.
  - Token hashes are the only token values stored; tokens are one-time and expire.
  - Logs contain no tokens, comments or image bytes.
  - Media that doesn't decode is refused.
  - Login is rate-limited.
- **Guest portal in a browser:**
  - The token is gone from the address bar and history after the exchange.
  - The form is identical whatever the rating.
  - No third-party requests are made.
- **End to end:** the browser preview app runs against a local server, from capture through "Synced · QR ready"
  to a scan of the QR to feedback submitted. The QR is decoded by two decoders (jsQR and OpenCV, as in W2).
- **Checks:** clippy, svelte-check and APK Play checks, as in W2.
- **W3b only:** a smoke test on the vServer (`/health`, TLS certificate, one real capture from a phone).

## Tenth Man

- **The phone path is unproven.** W3 is the first code that sends photos off the device. W2 already found one
  Android IPC bug by reading source; D9 (the device check) is still open. Syncing on top of an unverified
  capture path compounds risk. Recommendation: run D9 before the sync client is written.
- **Hosting is unknown.** D4 has been open since W0. If the ZAP product turns out to be web space, not a vServer,
  W3b is blocked, and the spec calls that a hard gate. W3a is safe to build either way.
- **Real guest data needs backups.** Spec Phase 4 (3-copy encrypted backups plus a restore drill) comes after
  W3. Recommendation (R1): W3b runs with **test data only** until W4's restore drill passes.
- **Device session storage.** App-private storage is readable on rooted phones. Android Keystore needs extra
  native code. Proposal: app-private storage plus short sessions with server-side revocation now; Keystore
  later if the owner wants it.
- **Size.** W3 is larger than W2. Hence the split into W3a and W3b, each with its own gate report and approval.

## Decisions needed from owner

- **W3a — Open it?** Build and host-test the server, guest portal and app sync, with no deployment.
  Recommended: yes.
- **D4 — Hosting (hard gate for W3b).** Send the ZAP product name and the output of
  `uname -a; systemctl --version | head -1; nproc; free -h; df -h /`. Please don't send passwords or keys.
- **G1 — Guest link domain.** Which domain will the QR codes open (for example `nocapsnap.com`)? You need
  control of its DNS.
- **G2 — HTTPS.** Caddy in front of the server (automatic certificates, a mature separate process; recommended),
  or certificates inside the Axum binary (one process, more of our own code)?
- **D9 — Device check.** Run the W2 checklist (about 10 minutes) before the sync client is built (recommended),
  or waive it again?
- **R1 — Real guest data.** Allow real guest feedback only after the W4 backup restore drill passes
  (recommended)?

## D4 intake (2026-09-30)

The owner sent the VPS deployment and infrastructure documents. *[Details redacted for publication.]* Addresses and credentials
from them were never copied here.

**What the docs establish** (**Specified**: owner documentation, not yet observed on the box):

- **The product is a ZAP Hosting VPS with root access, not web space.** Spec §7's hard gate ("Axum requires a
  long-running Linux process") is met on paper: other apps already run there as systemd services.
- **nginx owns ports 80 and 443**, with Let's Encrypt certificates and one server block per site.
- **The box is shared with other services.** Their deploy conventions, ports and software were taken into
  account. *[Details redacted for publication.]*

**How W3b changes as a result:**

| # | Change | Why |
|---|---|---|
| 1 | **G2 becomes nginx**, not Caddy, using the existing certificate flow | Caddy would need ports 80 and 443, which nginx already holds |
| 2 | Set nginx `client_max_body_size 10m` on the CapSnap server block | nginx's 1 MB default would refuse most photos with a 413 (the server allows 10 MiB) |
| 3 | Data lives in `/var/lib/capsnap`, **outside** `/var/www/capsnap` | `rsync --delete` would erase the database and photos. A tar rollback would also restore old data, losing feedback and reviving used guest links |
| 4 | Bind to `127.0.0.1:8090` (`CAPSNAP_BIND`); never open it in `ufw` | 8090 is free on the box. Only nginx should face the internet |
| 5 | Keep logs 30 days with a journald namespace (`LogNamespace=capsnap`, `journald@capsnap.conf`), not the global setting | A global `MaxRetentionSec` would also cut every other service's logs. Needs systemd ≥ 245 |
| 6 | Run as a dedicated `capsnap` system user with systemd hardening | The service doesn't need root |
| 7 | Ship a static musl binary (`x86_64-unknown-linux-musl`) | A glibc binary built on `ubuntu-latest` can fail on an older VPS with `GLIBC_2.xx not found` |
| 8 | A new Rust workflow at the repo root, not the Node template:<br>• tests block the deploy;<br>• `actions/*-artifact@v4`;<br>• a pinned host key | The existing template targets Node projects. `ssh-keyscan` at deploy time trusts whichever host answers |
| 9 | nginx adds HSTS for the guest domain | The server sets CSP and the other headers, but not HSTS |
| 10 | No analytics on the guest page | Spec §6. The portal's CSP would block it anyway |

**Unchanged:**

- **Backups.** The pre-deploy tar snapshots sit on the same disk and are not Spec Phase 4 backups. R1 stands:
  test data only until W4.
- **Merging PR #1 deploys nothing.** This repository has no root `.github/workflows/`. The only workflow file,
  `hms-stele/.github/workflows/ci.yml`, is nested, so GitHub never runs it.

**Still open:**

- **G1: the guest domain.** Not in the docs.
- **The box's real numbers.** Architecture, OS version, systemd version, memory and disk are unknown. These become
  W3b step 1: read-only checks on the box before anything is installed. Memory matters because the box is shared
  with other services.
- **Owner approval to open W3b.**
