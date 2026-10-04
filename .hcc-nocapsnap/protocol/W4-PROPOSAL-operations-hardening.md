# W4 — Operations hardening (PROPOSAL)

Date: 2026-10-04 · Status: **W4a opened** (owner, 2026-10-04; built in this branch, see the table below). W4b waits for B1–B4 and for W3b (a live box to
back up).

Spec §7 Phase 4: *"Execute the 3-copy encrypted backup plan and perform a full restoration drill."* The runbook adds
two rows this gate must satisfy: *Disk nearly full* ("verify backup before manual cleanup") and *Backup failure*
("pause destructive retention or migration tasks … rerun encrypted snapshot and verify hash"). R1 (W3 proposal)
ties real guest data to this gate: **W3b runs on test data only until the restore drill passes.**

## Gate report

```
## Gate W4 — Operations hardening
Status: Specified (proposal only)
Evidence: none yet; exit evidence defined below
Changes: this file only
Tenth Man: backups of data the spec says to delete (30-day photos, 12-month feedback) can quietly outlive retention
Decision needed from owner: open W4? plus B1–B4 below
```

## What exists today

- **Built:** the server keeps everything under `/var/lib/capsnap` (SQLite in WAL mode, plus the photo files),
  separate from the releases in `/var/www/capsnap`. `capsnap-server retention` runs hourly (Spec §6).
- **Not built:** any backup. The deploy README says so: *"the only copies today are on the same disk."* The
  pre-deploy tar snapshots are not Spec Phase 4 backups.

## Proposed scope

| Area | What | Spec |
|---|---|---|
| Snapshot | `capsnap-backup`, run on the box by a systemd timer (default daily). One consistent snapshot: the database through SQLite's online backup (`VACUUM INTO` or the backup API, never a file copy of a live WAL database), plus the photo files, plus `capsnap.env`. Written to a staging directory outside the data directory | §7 Ph4 |
| Encryption | Encrypted **before** it leaves the box, with a key kept off the box. Proposed tool: `age` (single static binary, public-key, so the box holds only the public half and cannot decrypt its own backups) | §7 Ph4 |
| Three copies | **Copy 1:** the box's local disk (fast restore, same-disk risk). **Copy 2:** a second machine the owner controls (B1). **Copy 3:** offline or off-site media the owner holds (B1). No managed cloud, per §2 | §2, §7 |
| Integrity | A SHA-256 manifest per snapshot, verified after every copy and before every restore. A failed copy or hash raises *Backup failure*: the timer exits non-zero, a marker file blocks retention and migrations, and `capsnap-release` refuses to activate a release while it is set | §7 runbook |
| Retention of backups | Bounded to the data's own limits (see Tenth Man): proposed 30 days of dailies, so a photo deleted at day 30 is gone from every copy by day 60 at the latest | §6 |
| Disk guard | The server already must "stop accepting server media" when nearly full. W4 adds the check that a verified snapshot exists before any manual cleanup (`capsnapctl` prints it) | §7 runbook |
| Restore | `capsnap-restore <snapshot>`: verify hash, decrypt, restore into a **fresh** data directory, run the migrations check, start the service against it, run `smoke.sh`. Never restores over live data without an explicit flag | §7 Ph4 |
| Alerting | One owner-visible signal when the last good snapshot is older than 36 hours (B3) | §7 runbook |

## Exit evidence (the restore drill)

1. On the live box, with test data: create a restaurant, a location, staff, several captures with photos, one
   feedback answer and one used guest link.
2. Take a snapshot and push it to all three copies. Verify the hashes on each.
3. **Restore from copy 3 only** onto a clean Ubuntu 24.04 machine (a second VPS, or a CI-rehearsed container, the
   box itself is not enough). The owner holds the decryption key and does this step with the written runbook.
4. Verify on the restored system: login works, the menu and captures match, photos decode, the used guest link is
   still used, retention state is intact, `smoke.sh` passes.
5. Record the elapsed time (the recovery time) and the age of the data restored (the recovery point).
6. Break it on purpose: corrupt one copy, and remove the key from the box. The snapshot must be refused, *Backup
   failure* must fire, and the other copies must still restore.
7. CI: unit and integration tests for snapshot consistency under writes and for refusal on a bad hash; the
   existing systemd + nginx rehearsal extended to take and restore a snapshot.

On a pass, R1 is lifted: real guest feedback is allowed.

## Tenth Man

- **Backups defeat retention.** Spec §6 deletes photos after 30 days and feedback after 12 months. A year of
  backups would keep all of it. Hence the 30-day backup window above. The consequence: feedback older than 12
  months cannot be recovered from a backup, which is the intended behaviour, but the owner must accept it (B2).
- **A backup nobody has restored is not a backup.** The gate is the drill, not the timer. If step 3 cannot be done
  on a second machine, the gate does not close.
- **Key loss is data loss.** If the private key is lost, all three copies are useless. Where the key lives (B4) is
  the most important decision here. Keeping it on the box would make the encryption pointless.
- **Copy 2 and 3 need hardware or an account the owner doesn't have yet.** The spec forbids managed cloud
  storage. Anything else is the owner's equipment and the owner's time. Without it W4 can be built and tested but
  not closed.
- **Consistency.** A plain copy of a WAL-mode database taken while the server writes can be corrupt. The design
  uses SQLite's own backup, and a test must prove it under concurrent writes.
- **Photos can be large.** Daily full copies of 30 days of photos may be heavy on a shared VPS. Mitigation: photo
  files are immutable, so snapshots copy only new files and the manifest references the rest. This adds code to
  get right, so it is called out here rather than assumed.
- **Size.** If it grows, split it: W4a (snapshot, encryption, restore, tested locally and in CI) and W4b (the
  real copies and the owner's drill), as W3 was.

## Decisions needed from owner

- **W4 — Open it?** Recommended: yes, as W4a first (no hardware needed), W4b when B1 is answered.
- **B1 — Where do copies 2 and 3 live?** Candidates: a second VPS or home server (copy 2); an encrypted external
  disk or a second location (copy 3). All owner-controlled, none a managed cloud service.
- **B2 — Backup window.** Accept 30 days of backups (matches photo retention), knowing older feedback is not
  restorable? Or keep longer for feedback, which needs a spec amendment because §6 says 12 months then delete.
- **B3 — Alert channel.** How should the owner learn a backup failed (email from the box, or a status line the owner
  checks)? Email needs an outbound mail path the box does not have yet.
- **B4 — Key custody.** Who holds the private key, and where is the second copy of the key (a password manager
  entry, a printed copy in a safe)? It must never be on the box or in this repository.
- **Drill machine.** A second Ubuntu 24.04 machine for step 3 (a throwaway VPS for an hour is enough).

## Labels

| Item | Label |
|---|---|
| Snapshot, encryption, manifest, restore tooling | **Specified** |
| Three copies in place, on the owner's hardware | **Specified**: waits for B1 |
| Restore drill passed | **Specified**: the gate's exit, not started |
| Lifting R1 | **Specified**: follows a passed drill |
