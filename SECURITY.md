# Security policy

CapSnap handles restaurant staff accounts, dish photos and private guest feedback. Reports that help keep
that safe are welcome.

## Reporting a vulnerability

**Please do not open a public issue.** Report privately through GitHub: **Security → Report a vulnerability**
on this repository. Include what you found, how to reproduce it, and what an attacker could gain.

- We aim to acknowledge reports within **5 business days**, and to keep you updated until there is a fix.
- With your agreement, we credit you when the fix is published.

## Scope

In scope:

- the server in `server/`: staff API, media validation, guest portal at `/g/`;
- the staff app in `app/` and its Rust crates in `crates/`;
- the deploy kit in `deploy/` and the CI workflows.

Out of scope:

- third-party services, and marketing pages that are not served by the CapSnap server;
- volumetric denial of service, social engineering, and physical attacks.

## Testing

- **Test against your own local build** (see the README and `server/README.md`), not the live service.
- Never access, change or delete other people's data.
- Stop and report as soon as you see data that isn't yours.

We won't pursue good-faith research that follows these rules.

## Design commitments

These are tested in `server/tests/` and described in the specification
(`.hcc-nocapsnap/spec/`, §4 to §6):

- **Guest links:** 256-bit random and one use. Only their hashes are stored, and they never appear in logs. The
  token travels in the URL fragment, which browsers do not send to servers.
- **Guest privacy:** no names, emails or phone numbers. No third-party scripts, fonts or analytics on guest pages,
  enforced by a strict Content-Security-Policy.
- **Staff passwords and sessions:** Argon2id passwords; sessions are revocable and stored hashed.
- **Uploads:** photos are refused unless they decode cleanly and carry no metadata.
- **Retention:** originals 30 days after sync, guest links 30 days or first use, feedback 12 months, logs 30 days.
