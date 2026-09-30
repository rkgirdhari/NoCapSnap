# W3 — Hosted feedback slice (PROPOSAL)

Date: 2026-09-30 · Status: **Proposed. Not started; nothing is built until the owner approves.**

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
