# W3b owner checklist: reinstall the VPS and deploy CapSnap

**Status 2026-10-06:** steps 1–5 done, except dropping the VPS records for `hcc.software` / `www`. CapSnap is live at
<https://nocapsnap.hammurabi.click>; what happened is in [W3b-deploy-kit.md](W3b-deploy-kit.md) ("Live deploy").

Steps 1–5 of the order in [W3b-deploy-kit.md](W3b-deploy-kit.md) ("Owner decision (2026-10-03)"). Commands are in
[`deploy/README.md`](../../deploy/README.md). Target: **Ubuntu 24.04 LTS**, domain `nocapsnap.hammurabi.click`.

## 1. Back up the IIS site(s) (before anything is wiped)

- [ ] In IIS Manager, list every site and note its **bindings** (the domains it serves).
- [ ] Copy each site's physical folder (and any app pool or config you rely on) to your own computer.
- [ ] Decide where the company site (`hcc.software`) lives afterwards. Recommended: the existing Manus copy.
- [ ] Open the backup on your computer and confirm the files are there.

## 2. DNS

Do the first item before the reinstall, the second before install.

- [ ] Remove the **VPS** A records for `hcc.software` and `www.hcc.software`, unless the company site moves to
      the new box.
- [ ] Remove the **Manus** A record for `nocapsnap.hammurabi.click`, leaving only the VPS address.
- [ ] Wait for DNS to settle; preflight (step 5) checks it.

## 3. SSH key on your PC

- [ ] In PowerShell: `ssh-keygen -t ed25519`, with a passphrase.
- [ ] Keep `%USERPROFILE%\.ssh\id_ed25519.pub` ready to paste into the ZAP panel at reinstall.

## 4. Reinstall in the ZAP panel

- [ ] Do it **outside** ZAP's maintenance window (7 October, 05:00–11:00).
- [ ] Choose **Ubuntu 24.04 LTS** (Debian 12 if not offered); add your public key if the panel asks.
- [ ] Confirm steps 1 and 2 are done: this wipes the disk.
- [ ] Log in: `ssh root@<box>`. If `ssh` warns the host key changed, run `ssh-keygen -R <box>` and reconnect.

## 5. Prepare the box, then first install

"A freshly installed box" in `deploy/README.md`, as root:

- [ ] `apt update && apt full-upgrade -y`, then `apt install -y nginx certbot ufw unattended-upgrades`, then
      `dpkg-reconfigure -plow unattended-upgrades` (answer Yes).
- [ ] Firewall: allow 22, 80 and 443, then `ufw enable`. Allow your SSH port instead if it isn't 22.
- [ ] Key-only logins: first confirm in a second window that the key works with no password, then write
      `/etc/ssh/sshd_config.d/10-key-only.conf` and restart ssh. Keep the first window open until a new key login works.

"First install":

- [ ] Copy `deploy/` to `/root/capsnap-deploy` on the box and run `preflight.sh nocapsnap.hammurabi.click`.
      Every line should be OK, with "DNS: … points at this box".
- [ ] Download the `capsnap-server-x86_64-linux-musl` artifact from the latest green **CapSnap server** run, check
      it with `sha256sum -c capsnap-server.sha256`, and copy `capsnap-server` into `/root/capsnap-deploy/`.
- [ ] `install.sh --domain nocapsnap.hammurabi.click --binary /root/capsnap-deploy/capsnap-server`
      (add `--email` if certbot has no account).
- [ ] From your own computer: `./smoke.sh nocapsnap.hammurabi.click`. Every line should say PASS.
- [ ] Test data only (R1): `capsnapctl create-org`, `create-location`, `create-staff`. Sign in from the app's
      Settings with `https://nocapsnap.hammurabi.click` as the server.
- [ ] Send the preflight and smoke output back so the W3b record can be closed.
