# To-Do

Updated 2026-10-04. Gate status lives in the table in [README.md](README.md); this file is what is open and who holds it.

## Yours (the owner)

**Deploy (W3b)** — [checklist](.hcc-nocapsnap/protocol/W3b-owner-checklist.md)
- [ ] Back up the IIS site(s) on the VPS, with their bindings
- [ ] DNS: drop the VPS records for `hcc.software` / `www` (unless that site moves), drop the Manus record for `nocapsnap.hammurabi.click`
- [ ] Reinstall Ubuntu 24.04 in the ZAP panel, outside the 7 October 05:00–11:00 window, with your public key
  - 2026-10-04: `5.249.163.79` answers SSH but refuses the key (`Permission denied (publickey)`); either add the key or `ssh-add` the passphrase
- [ ] Then "A freshly installed box", `preflight.sh`, `install.sh`, `smoke.sh`

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
- [ ] D9 step 5: back button and camera indicator (still "not reported")
- [ ] Run the Windows desktop installer for the first time

## Mine (Claude) — open

- [ ] In-app link to the privacy policy. Needs the Tauri opener plugin, so it changes the app's capabilities and wants its own review
- [ ] iOS: opens after W4; proposal waits on I1–I4 ([proposal](.hcc-nocapsnap/protocol/IOS-PROPOSAL-apple-app-store.md))
- [ ] Incremental photo copies in the backup (proposed in W4, not built)

## Done

- [x] Remote branches cleaned up; W3b checklist, W4 and W5 proposals merged (#31, #32, #34)
- [x] W4a: encrypted backup, verify, restore, failure pause (#33)
- [x] W5a: `/privacy`, Data Safety draft and CI guard (#35); `capsnapctl remove-staff` (#36)
