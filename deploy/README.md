# CapSnap deploy kit (W3b)

Puts the CapSnap server on the ZAP VPS behind the nginx that already serves the box's other
sites. Built for the conventions in the owner's ZAP VPS docs; the reasons for each choice are
in `../.hcc-nocapsnap/protocol/W3-PROPOSAL-hosted-feedback-slice.md` ("D4 intake").

| File | Runs where | What it does |
|---|---|---|
| `preflight.sh` | the box, as root | Read-only checks: architecture, OS, systemd, nginx, ports, disk, DNS. Changes nothing |
| `install.sh` | the box, as root | First install, and safe re-runs. See its header for the options |
| `smoke.sh` | your own computer | Checks the live site from outside: TLS, redirect, headers, upload limit, exposure |
| `bin/capsnap-release` | the box (installed to `/usr/local/sbin`) | Switch releases, check health, switch back if a release fails |
| `bin/capsnapctl` | the box (installed to `/usr/local/sbin`) | Admin commands (restaurants, staff, menus) as the service user |
| `systemd/`, `nginx/`, `capsnap.env.in` | templates | Used by `install.sh` |

## Layout on the box

| Path | Holds | Notes |
|---|---|---|
| `/var/www/capsnap/releases/<UTC time>-<id>/capsnap-server` | Binaries, newest 5 kept | `current` and `previous` are symlinks |
| `/var/lib/capsnap/` | The SQLite database and photos | Owned by `capsnap`, mode 0700. **Never** inside `/var/www/capsnap` |
| `/etc/capsnap/capsnap.env` | Settings: guest-link base URL, bind address, data directory | `root:capsnap` 0640 |
| `/etc/systemd/system/capsnap.service` | The sandboxed service | Runs as `capsnap`, not root |
| `/etc/systemd/journald@capsnap.conf` | Log retention: 30 days (Spec §6) | Only CapSnap's logs; the rest of the box is unchanged |
| `/etc/nginx/sites-available/capsnap.conf` | The two `server` blocks for the guest domain | Only nginx faces the internet |

The server listens on `127.0.0.1:8090` only. Don't open that port in `ufw`.

## First install

1. **Preflight.** Copy this folder to the box and run it, then send the output back. It
   contains no passwords, keys or IP addresses.

   ```bash
   scp -r deploy root@<box>:/root/capsnap-deploy
   ssh root@<box> /root/capsnap-deploy/preflight.sh            # later: preflight.sh <domain>
   ```

2. **DNS (G1).** Point an A record for the guest domain at the box, then re-run
   `preflight.sh <domain>` until DNS shows OK.

3. **The binary.**
   1. Open the repository's **Actions** tab, then the latest green **CapSnap server** run.
   2. Download the `capsnap-server-x86_64-linux-musl` artifact.
   3. Check it with `sha256sum -c capsnap-server.sha256`.
   4. Copy `capsnap-server` into `/root/capsnap-deploy/` on the box.

4. **Install.** This gets a Let's Encrypt certificate through the existing nginx. If certbot has
   no account on the box yet, add `--email`.

   ```bash
   ssh root@<box> /root/capsnap-deploy/install.sh --domain <domain> --binary /root/capsnap-deploy/capsnap-server
   ```

5. **Smoke test** from your own computer (Linux, macOS, WSL or Git Bash): `./smoke.sh <domain>`.
   Every line should say PASS.

6. **Test data only (R1).** Create a restaurant and an admin to try the app with. Don't use a
   real restaurant until the W4 backup restore drill has passed.

   ```bash
   capsnapctl create-org <slug> "<Name>"
   capsnapctl create-location <slug> "<Location>" Europe/Berlin     # prints the location id
   read -rs PW && printf '%s\n' "$PW" | capsnapctl create-staff <slug> <login> "<Name>" admin <location-id>
   ```

   In the app, go to Settings and sign in, with `https://<domain>` as the server.

## Later releases (GitHub Actions)

The `deploy` job in `.github/workflows/capsnap-server.yml` runs only when you start it:
**Actions → CapSnap server → Run workflow** on `main`, with **deploy** ticked. It uploads
the new binary, runs `capsnap-release activate`, and runs `smoke.sh` against the domain. A
release that fails its health check is switched back automatically.

It needs these repository settings:

- **Secrets:**
  - `VPS_HOST`, `VPS_PORT`, `VPS_USER` and `VPS_DEPLOY_KEY`: the same ones your other deploys use.
  - `VPS_KNOWN_HOSTS`: the box's host key, so the workflow never trusts an impostor. On the box,
    run `ssh-keygen -lf /etc/ssh/ssh_host_ed25519_key.pub`. On your computer, run
    `ssh-keyscan -p <port> -t ed25519 <host>`. Check that the scanned key's fingerprint matches
    the box's, then store that line as the secret.
- **Variable:** `CAPSNAP_DOMAIN`, the guest domain.
- **Environment (optional):** `capsnap-production`, with you as a required reviewer, if you want
  to approve each deploy.

## Running it

| Task | Command (on the box, as root) |
|---|---|
| Health and releases | `capsnap-release status` |
| Undo the last release | `capsnap-release rollback` |
| Server logs (30 days) | `journalctl --namespace=capsnap` |
| Start and stop messages | `journalctl -u capsnap` |
| A lost phone | `capsnapctl revoke-sessions <login>` |
| Restart | `systemctl restart capsnap` |

## Rules that keep this safe

- **Database migrations must be additive** (new tables, new nullable columns). An older binary
  starts on a newer schema, which is what makes `capsnap-release rollback` safe. A migration
  that drops or renames something needs its own plan.
- **Never change `CAPSNAP_PUBLIC_BASE_URL` casually.** Every guest QR already shown points there.
  `install.sh` refuses to change it.
- **Backups:** the only copies today are on the same disk. Real backups are W4 (Spec Phase 4).
  Until then, test data only.
