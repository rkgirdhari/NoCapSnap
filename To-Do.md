# To-Do

Updated 2026-10-06. Gate status lives in the table in [README.md](README.md); this file is what is open and who holds it.

## Yours (the owner)

**Deploy (W3b)** — [checklist](.hcc-nocapsnap/protocol/W3b-owner-checklist.md). Live at <https://nocapsnap.hammurabi.click> since 2026-10-06 ([record](.hcc-nocapsnap/protocol/W3b-deploy-kit.md), "Live deploy")
- [x] Back up the IIS site(s); reinstall (Ubuntu 24.04.5, since upgraded to 26.04.1); "A freshly installed box"
- [x] DNS: `nocapsnap.hammurabi.click` points at the VPS only
- [x] `preflight.sh`, `install.sh`, `smoke.sh` (11/11); test restaurant and admin; phone sign-in, sync and QR
- [ ] DNS: drop the VPS records for `hcc.software` / `www` (both still answer with Manus *and* the VPS)
- [ ] Scan a guest QR with a second phone: guest page, rating, and the second scan refused (W3a steps 3–5)
- [ ] Check the server's SSH host-key fingerprint once from the ZAP console, then add `VPS_KNOWN_HOSTS` for the deploy job

**Backups (W4b)** — [proposal](.hcc-nocapsnap/protocol/W4-PROPOSAL-operations-hardening.md)
- [ ] B1 where copies 2 and 3 live · B2 backup window (30 days) · B3 how you learn a backup failed · B4 who holds the private key
- [ ] Make the `age` key pair on your own PC; only the public half goes on the box
- [ ] A second Ubuntu 24.04 machine for the restore drill
- [ ] Real guest data stays off until the drill passes (R1)

**Play (W5)** — [proposal](.hcc-nocapsnap/protocol/W5-PROPOSAL-beta-play-readiness.md), [draft](.hcc-nocapsnap/protocol/W5a-data-safety-draft.md)
- [ ] P1 register the Play developer account as the LLC; start D-U-N-S verification (the long pole)
- [ ] P2 the privacy-policy contact (set `CAPSNAP_PRIVACY_CONTACT`) · P3 public or private distribution · P4 beta group and phones
- [ ] Enter the Data Safety answers in Play Console, against the live form
- [ ] Make the Play upload key and add the four repository secrets (see the Android CI record)

**Devices**
- [ ] D9 step 5: back button and camera indicator (still "not reported"); also tap Settings → "Read the privacy policy" once signed in
- [ ] Run the Windows desktop installer for the first time

## Mine (Claude) — open

- [ ] iOS: opens after W4; proposal waits on I1–I4 ([proposal](.hcc-nocapsnap/protocol/IOS-PROPOSAL-apple-app-store.md))

## Done

- [x] Remote branches cleaned up; W3b checklist, W4 and W5 proposals merged (#31, #32, #34)
- [x] W4a: encrypted backup, verify, restore, failure pause (#33)
- [x] W4a: incremental photo copies: each photo encrypted once into a pool, daily archives refer to it by hash
- [x] W5a: in-app "Read the privacy policy" link (Rust-side opener, no new WebView permission); needs a phone check
- [x] W5a: `/privacy`, Data Safety draft and CI guard (#35); `capsnapctl remove-staff` (#36)
