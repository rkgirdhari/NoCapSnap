# W3b — Deploy kit for the ZAP VPS, built and rehearsed

Date: 2026-09-30 · Status: **Kit built and rehearsed (locally and in CI, all green); not yet on the VPS.** The owner opened W3b with *"Approved"*
after the D4 intake ([`W3-PROPOSAL-hosted-feedback-slice.md`](W3-PROPOSAL-hosted-feedback-slice.md)).

Nobody from this project has access to the VPS, and nobody should: no keys or passwords are handled here. So W3b is
split:

- **Done here:** everything needed to put the server on the box, and a rehearsal of it on real systemd and nginx.
- **Done by the owner on the box:** preflight, DNS, install and the smoke test. The runbook is
  [`nocapsnap/deploy/README.md`](../../deploy/README.md).

## Gate report

```
## Gate W3b — Deploy kit (built + rehearsed; live deploy pending)
Status: Built — kit and CI; the VPS itself is Specified (not touched)
Evidence: local run of the real install.sh with nginx and TLS: smoke 11/11; 2 mutations caught;
          rollback drill; refusals checked. CI (run 36789703170, 93a9cb3): test, build and kit
          rehearsal green on real systemd 255 + nginx; smoke 11/11; systemd-analyze security 1.4 OK
          server 10 tests (1 new), sync e2e 6, onboard 17; shellcheck, actionlint clean
Changes: nocapsnap/deploy/ (new), .github/workflows/capsnap-server.yml (new),
         server: `version` command, rollback-tolerant migrations (+ test)
Tenth Man: the rehearsal box is a fresh Ubuntu runner; the VPS has other live sites, an nginx and
           a certbot state nobody here has seen
Decision needed from owner: G1 (domain); run preflight.sh on the box and send the output
```

## What was built

**The deploy kit** (`nocapsnap/deploy/`):

| Piece | What it does |
|---|---|
| `preflight.sh` | Read-only checks on the box: architecture, OS, systemd ≥ 245, memory, disk, nginx (`nginx -t`, layout), certbot, ports 8090, 80 and 443, IPv6, tools, and DNS for the domain. The output has no secrets or IP addresses |
| `install.sh` | Creates the `capsnap` user and installs the release, settings, unit, journald namespace, nginx blocks and tools. It gets a certificate with certbot (webroot) or uses a given one. Re-runs are safe |
| `systemd/capsnap.service` | Runs as `capsnap`, sandboxed: read-only system, no capabilities, a syscall filter, private /tmp, state only in `/var/lib/capsnap` |
| `systemd/journald@capsnap.conf` | CapSnap's logs only, kept 30 days (Spec §6); the rest of the box is unchanged |
| `nginx/capsnap.conf.in` | Two `server` blocks, described below this table |
| `bin/capsnap-release` | Switches the `current` symlink, restarts and waits 30 s for health. On failure it switches back automatically. `rollback` goes to the previous release; old releases are pruned, keeping the newest 5 |
| `bin/capsnapctl` | Admin commands as the service user, with the service's settings |
| `smoke.sh` | Outside-in checks, described below this table |

The two nginx `server` blocks:

- **Port 80:** the ACME challenge, then a 301 redirect to https.
- **Port 443:**
  - TLS 1.2 and 1.3;
  - HSTS for this host only;
  - `client_max_body_size 10m`;
  - `server_tokens off`;
  - an access log with no client addresses and no query strings;
  - a proxy to `127.0.0.1:8090`.

The outside-in checks in `smoke.sh`:

- health over verified TLS;
- the certificate name and at least 14 days left;
- the http → https redirect;
- the portal, with CSP and HSTS;
- no server version in the headers;
- a 2 MiB upload reaches the server, and 11 MiB is refused;
- an unknown guest token gets a 4xx;
- port 8090 is not reachable from outside.

**CI** (`.github/workflows/capsnap-server.yml`, at the repo root, so GitHub runs it):

1. **test:** fmt, clippy `-D warnings` and tests for the server and ONB-1, plus shellcheck. Failures block the rest.
2. **build:** a static `x86_64-unknown-linux-musl` binary, which runs on any x86_64 Linux whatever its glibc.
   Uploaded with a sha256, using `upload-artifact@v4`.
3. **kit:** the whole runbook on the runner itself, a real systemd and a real nginx:
   - preflight, install and smoke;
   - `systemd-analyze verify` and `security`;
   - the journald namespace;
   - admin commands;
   - a broken release rolled back;
   - an idempotent re-run.
4. **deploy:** runs **only by hand, on main, with "deploy" ticked**.
   - It uses the ZAP docs' secret names plus `VPS_KNOWN_HOSTS`: a pinned host key, not `ssh-keyscan` at deploy time.
   - It uploads the binary as a new release and runs `capsnap-release activate`, then `smoke.sh` against
     `CAPSNAP_DOMAIN`.
   - The first install is `install.sh` by hand.

**Server changes** (small, for the kit):

- `capsnap-server version`, which `install.sh` uses to check that the binary runs on the box.
- **Migrations now tolerate a newer schema** (`set_ignore_missing(true)`). Without this, rolling back to the
  previous binary after a release had added a migration would fail at startup with `VersionMissing`.
  - The test `tests/rollback.rs` covers it.
  - A mutation check (tolerance switched off) makes the test fail with exactly that error.

## Found and fixed during the gate

1. **The first install on the box would have failed.** `capsnap-release` resolved a missing `current` link with
   `readlink -f`. GNU returns a path even for a missing link, so the script treated `current` as the previous
   release. When the health check was slow, it "switched back" to `current -> current`, a self-loop.
   - It is fixed with a `target_of` helper that resolves only links that exist.
   - It was found by running the real `install.sh`; the broken state was then used to test recovery.
2. **Local health checks could go through a proxy.** Wherever `https_proxy` is set, `curl` sent the check for
   `https://<domain>` through the proxy (a 502 here). `install.sh` and `capsnap-release` now use `--noproxy '*'`.
3. **Rolling back past a migration would fail**, as described above.
4. **A refused upload could reach the client as 502 instead of 401.** The server answered "no session" before
   reading the photo and closed the connection; nginx, still sending the buffered body, logged `sendfile() failed
   (32: Broken pipe) while sending request to upstream` and answered 502. The CI rehearsal hit it once, on a
   docs-only commit, after three green runs. It was reproduced locally (10 of 30 parallel 9 MiB uploads came back
   502).
   - Impact: a phone whose session had expired would have kept retrying a "server error" instead of signing out.
   - Fix: `POST /media` now reads the rest of the body (at most 10 MiB, 30 s) before any early refusal: no
     session, wrong type, or missing checksum.
   - Test: `tests/upload_refusal.rs` failed 3 of 3 before the fix and passes after.
   - Local stress after the fix: 90 of 90 return 401, with no new nginx upstream errors.
5. **Hazard seen, not a bug:** Ubuntu's stock nginx site listens on `[::]:80` and fails `nginx -t` on a machine
   without IPv6. `install.sh` writes `[::]` listeners only when the box has IPv6, and `preflight.sh` fails if
   `nginx -t` already fails before CapSnap is added.

## Evidence

**Local rehearsal** (this container):

- **Setup:**
  - the real `install.sh`;
  - nginx 1.24;
  - a certificate from a test CA;
  - the static binary;
  - a stand-in `systemctl` that creates the state directory and starts the process, because the container has no
    systemd. The unit's sandbox is therefore tested in CI, not here.
- **Results:**

| Check | Result |
|---|---|
| `install.sh` first run (over the broken state from finding 1) | release staged, activated, healthy; https health through nginx OK |
| `install.sh` re-run, same binary | no new release; restart; health OK |
| `smoke.sh capsnap.test` (resolving to the container's non-loopback address) | **11/11 PASS** |
| Mutation: remove `client_max_body_size` | caught: "a 2 MiB body got '413'" |
| Mutation: bind the server to `0.0.0.0` | caught: "port 8090 answers from outside" |
| `capsnap-release activate` with a release that dies on start | "failed its health check … switched back"; health OK after 30 s |
| `capsnap-release activate` good release, then `rollback` | current and previous swap as expected |
| Prune with 10 releases | newest 5 plus current and previous kept |
| `capsnapctl create-org / create-location / create-staff` | ran as `capsnap`; data dir `capsnap` 0700, settings `root:capsnap` 0640 |
| `install.sh --domain other.test` over an existing install | refused: changing the domain breaks printed QRs |
| `install.sh` with a certificate nginx rejects | refused; the site file is unchanged, `nginx -t` passes, and https still serves |
| nginx access log | `2026-…T22:49:03+00:00 "POST /api/v1/media" 401 75 0.004`: no client address, no query |

**Build and static checks:**

- **Static binary:** `static-pie linked`, 10.9 MB, `capsnap-server 0.0.0`.
- **Host tests:** server 10 (8 API, 1 log, 1 rollback), sync end-to-end 6, onboard 17. All pass with `--locked`.
- **Clippy:** `-D warnings` is clean.
- **Scripts:** shellcheck is clean on every script, and actionlint 1.7.7 (with shellcheck) is clean on the workflow.

**CI** (GitHub Actions, `ubuntu-24.04` runners). Run 36789703170, on commit `93a9cb3`:

| Job | Result |
|---|---|
| Test: fmt, clippy `-D warnings`, server and ONB-1 tests, shellcheck | success (1 min 40 s) |
| Build: static musl binary, sha256, artifact | success |
| Kit rehearsal on real systemd 255, nginx 1.24 and Ubuntu 24.04.5 (IPv4 only) | success |
| Deploy | skipped: runs only by hand from `main` |

Rehearsal highlights:

- **Preflight passed:** "systemd 255", "DNS: capsnap.test points at this box".
- **Install:** release activated; https health through nginx OK.
- **`smoke.sh`:** **11/11 PASS**, including "port 8090 does not answer from outside".
- **The unit:**
  - `systemctl show`: `User=capsnap`, `LogNamespace=capsnap`, `ActiveState=active`;
  - `systemd-analyze security`: **"Overall exposure level for capsnap.service: 1.4 OK"**;
  - the server's logs went to the `capsnap` journal namespace.
- **Permissions:** `/var/lib/capsnap` is `capsnap:capsnap 700`; `capsnap.env` is `root:capsnap 640`.
- **`capsnapctl`:** create-org and create-location worked under the sandbox.
- **Broken release:** "failed its health check … switched back"; `health: ok`; smoke 11/11 again.
- **Re-running install:** no new release.

**The first CI run failed on one check.** CI's own test certificate was issued for 2 days, and `smoke.sh` correctly
refuses a certificate with under 14 days left. The fixture was fixed; the check itself was not changed.

## Labels

| Item | Label | Why |
|---|---|---|
| Deploy kit: install, release switching, rollback, admin wrapper, smoke test, nginx blocks | **Built** | Local rehearsal above, and the CI `kit` job |
| systemd sandbox (exposure 1.4) and 30-day journald namespace | **Built** on systemd 255 (CI runner) | On the VPS: **Specified** until preflight shows its systemd version |
| Static musl binary | **Built** | CI `build` job |
| Let's Encrypt via certbot webroot | **Specified** | Needs a public domain; no test can reach Let's Encrypt |
| The deploy job (GitHub Actions to the VPS) | **Specified** | Needs the secrets, `VPS_KNOWN_HOSTS` and `CAPSNAP_DOMAIN`; never run |
| CapSnap live on the VPS; smoke test against the real domain; one real capture from a phone | **Specified** | Waiting on G1, preflight and install by the owner |
| Backups of `/var/lib/capsnap` | **Specified** | W4 (Spec Phase 4). R1: test data only |

## Tenth Man

- **The rehearsal box is not the VPS.**
  - The runner is a fresh Ubuntu 24.04 with nothing else on it. The VPS serves other sites, and its `nginx.conf`,
    certbot state and firewall are unknown here.
  - `nginx -t` tests the **whole** configuration, and a reload applies **all** of it. If someone left an unrelated
    site half-edited on disk, CapSnap's install would reload that too. Preflight fails if `nginx -t` already fails,
    but it can't detect a half-edit that happens to be valid.
- **Older systemd.** The unit needs systemd 245 or later (`LogNamespace`). On 245 and 246, two hardening lines
  (`ProtectProc`, `ProcSubset`) are ignored with a warning, so the sandbox is a little weaker. Preflight prints the
  version.
- **Least privilege for deploys.** The CapSnap service runs as `capsnap`, but the deploy job's SSH account has more
  rights than shipping a release needs. A deploy user allowed to run only `capsnap-release` would limit what a leaked
  `VPS_DEPLOY_KEY` could do; it isn't built.
- **Client IP addresses in nginx's error log.** The access log leaves them out, but the shared error log includes
  the client address on errors. Ubuntu rotates it after 14 days, within Spec §6's 30. It is not configurable per
  site without changing the box's global log settings.
- **Rollback safety depends on a rule, not a check.** Migrations must stay additive. Nothing enforces that yet; a
  destructive migration would make `rollback` unsafe.
- **No backups.** A disk failure loses everything. Until W4, R1 applies: test data only.

## Decisions needed from owner

- ~~**G1: the guest domain.**~~ Answered: `nocapsnap.hammurabi.click` (see "Owner decisions" below).
- **Run `preflight.sh` on the box and send the output.** Step 1 of the runbook. Once G1 is chosen, run it with the
  domain.
- **For the deploy job (optional now):**
  - the `VPS_KNOWN_HOSTS` secret, with the fingerprint checked on the box;
  - the `CAPSNAP_DOMAIN` variable;
  - optionally, a `capsnap-production` environment with you as required reviewer.
- **D9: still open.** The first real capture from a phone becomes part of W3b's live check.
- **Optional:** a restricted deploy user (Tenth Man, third point).

## Owner decisions (2026-10-01)

- **G1: `nocapsnap.hammurabi.click`.** It moves to the VPS. The owner chose this over a new subdomain, knowing that
  the Manus page there goes offline.
- **The Manus "Guest ordering" page is marketing or demo only.** It is not CapSnap's guest site, so the spec doesn't
  apply to it, and its ordering flow and `manus-analytics.com` script stay out of CapSnap.
- **Repository:** CapSnap moves to `rkgirdhari/nocapsnap` with its history, **after** that repo is made private. As of
  this entry the repo is still public, and nothing has been pushed to it.

**What G1 means for going live** (observed 2026-10-01, read-only from outside):

- The hostname currently resolves to Cloudflare, which serves the Manus site. Every path, including
  `/api/v1/health` and `/g/`, returns the Manus page.
- The DNS record must become an **A record to the VPS with Cloudflare's proxy off** (DNS only). Otherwise:
  - Let's Encrypt's http check can't reach nginx on the box;
  - preflight's DNS check can't match the box;
  - `smoke.sh` would be testing Cloudflare, not the server.
- The current site sends `Strict-Transport-Security: max-age=31536000; includeSubDomains; preload`. Browsers that
  visited it will insist on https for this name, which CapSnap already serves. Nothing to do.
- **Order:** change DNS → `preflight.sh nocapsnap.hammurabi.click` shows "DNS: … points at this box" →
  `install.sh --domain nocapsnap.hammurabi.click` → `smoke.sh` from outside.

## Owner decision (2026-10-03): reinstall the VPS with Linux

> *"for server hosting we will need to setup the VPS I will need help switching the 2016 server out for the Linux as
> discussed since windows 2016 ends in Jan. 2027"*

- **Why:**
  - The ZAP VPS turned out to run **Windows Server 2016**, which ends extended support on 12 January 2027 (Desktop
    record).
  - The owner chose to reinstall it with Linux rather than build a Windows deploy kit.
- **Target:** **Ubuntu 24.04 LTS**, the system the kit was rehearsed on in CI (systemd 255, nginx 1.24). Debian 12 if
  ZAP doesn't offer it.
- **What changes for W3b:**
  - The kit is used unchanged.
  - The box starts empty: no nginx, no certbot, no other sites. The runbook gains a step for that ("A freshly
    installed box" in `deploy/README.md`).
  - The first Tenth Man point (an nginx, certbot state and firewall nobody here has seen) mostly falls away, because
    the box will match the rehearsal.
- **What the reinstall costs:**
  - It wipes the disk, including the company site that IIS serves there now. The owner backs it up first.
  - Where the company site lives afterwards is the owner's call; the recommendation is the existing Manus copy (see
    DNS below).

**DNS, observed read-only from outside on 2026-10-03:**

| Name | Answers |
|---|---|
| `nocapsnap.hammurabi.click` | Two A records: Manus's address, and the VPS |
| `hcc.software`, `www.hcc.software` | Two A records each: Manus's address, and the VPS |
| `hammurabi.click`, `www.hammurabi.click` | Manus only |

With two A records, each visitor lands on either one, so:
- **Before the reinstall:** the VPS records for `hcc.software` and `www` should go, unless the company site moves
  to the new box.
- **Before install:** the Manus record for `nocapsnap` should go.

**Order, done by the owner:**
1. Back up the IIS site(s): their folders, plus their bindings, which show the domains they serve.
2. DNS changes as above.
3. Make an SSH key on the owner's PC.
4. In the ZAP panel, reinstall with Ubuntu 24.04 LTS, outside ZAP's maintenance window (7 October, 05:00–11:00).
5. Run "A freshly installed box", then the first-install runbook: preflight, install, smoke, test data.

| Item | Label |
|---|---|
| The VPS on Ubuntu 24.04, prepared as in "A freshly installed box" | **Specified**: the owner's steps, not started |
| CapSnap live on the VPS | **Specified**, as above |
