# capsnap-server

The CapSnap server (Spec §2, §5): the staff API, media validation and the guest feedback
portal. It runs as one Rust binary on Axum with SQLite, is self-hosted, and uses no cloud
services.

Status: **W3a, built and host-tested only.** Deployment (W3b) waits on the hosting check
(D4), the guest-link domain (G1) and HTTPS (G2).

## Run locally

```bash
export CAPSNAP_PUBLIC_BASE_URL=http://127.0.0.1:8080   # where guest links point (G1 in production)
export CAPSNAP_DATA_DIR=./data CAPSNAP_INSECURE_COOKIES=1  # plain http on this machine only
cargo run -- create-org atelier "Atelier No. 8"
cargo run -- create-location atelier "Atelier No. 8" America/Chicago     # prints the location id
echo 'a long password here' | cargo run -- create-staff atelier ada@atelier Ada admin
cargo run -- import-menu <location-id> menu.json                         # [{"name":…,"category":…}]
cargo run -- serve                                                       # 127.0.0.1:8080
```

## API (`/api/v1`)

| Route | Who | What |
|---|---|---|
| `GET /health` | anyone | liveness |
| `POST /auth/sessions` | staff | Sign in; returns a device session token. Rate-limited per sign-in name |
| `DELETE /auth/sessions/current`, `GET /me` | staff | Sign out; who am I |
| `GET /locations/{id}/menu-items` | staff | The menu of a location this person may use |
| `POST /media` | staff | Streamed JPEG upload with `X-Content-SHA256` (see below) |
| `POST /captures` | staff | Idempotent on `clientId`; returns the acknowledgement and the guest link |
| `POST /guest/session`, `GET /guest/photo`, `POST /guest/feedback` | guest | Token exchange, the plate photo, one rating (1–5) plus an optional comment |
| `POST /onboarding/import` | admin, manager | ONB-1 website import, with consent recorded |

The guest page is served at `/g/`.

### Rules the tests check

- **Tenancy.** The organization, location access and role come only from the session. Other
  tenants' ids get "not found".
- **Media.** An upload is refused unless:
  - the SHA-256 matches the declared digest;
  - the file is a JPEG;
  - it has no metadata segments;
  - it decodes fully;
  - its long edge is at most 2048 px.
- **Guest links.** Each link carries a 256-bit token and only its hash is stored. A link is
  single-use and lasts 30 days. The token lives in the URL fragment and is removed with
  `history.replaceState`.
- **Feedback is neutral.** Every guest gets the same form and the same thank-you. Every failure
  gets the same answer.
- **Logs.** They record the method, route template, status and time. They never record tokens,
  passwords, comments or image bytes.
- **Retention (hourly).**
  - Photos are deleted 30 days after sync. Photos that never got a capture are deleted after 1 day.
  - Feedback is deleted after 12 months.
  - Expired guest sessions and old device sessions are deleted.
  - Operational logs are kept for 30 days, via the journald `MaxRetentionSec=30day` setting (W3b).

Tests: `scripts/test-host.sh`.
