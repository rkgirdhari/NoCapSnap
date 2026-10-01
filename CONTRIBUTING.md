# Contributing

CapSnap is proprietary software by Hammurabi Coding Company LLC (see [LICENSE](LICENSE)). The source is public to
read. **We don't accept pull requests from outside the company.**

- **Bugs:** open an issue using the bug report form.
- **Security problems:** report them privately, as described in [SECURITY.md](SECURITY.md).

## How the team works

Work runs in **gates** (W0, W1, …), as set out in the specification (`.hcc-nocapsnap/spec/`).

- Each gate ends with a report in `.hcc-nocapsnap/protocol/` and waits for the owner's approval.
- Every claim is labelled **Built** (compiles and tested), **Specified** (designed, not built) or **Aspirational**,
  and every report includes a **Tenth Man** section arguing against itself.

Before pushing, run the checks for what you changed:

| Area | Checks |
|---|---|
| A Rust crate (`server/`, `crates/*`, `app/src-tauri/`) | `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `scripts/test-host.sh` |
| The app UI (`app/`) | `pnpm check`, `pnpm build` |
| The deploy kit (`deploy/`) | `shellcheck deploy/*.sh` and `shellcheck -s sh deploy/bin/*` |

Rules that keep releases safe:

- **Never commit secrets:** no `.env` files, keystores, signing keys or service-account files. `.gitignore` blocks
  the usual names, but check anyway.
- **Database migrations are additive only**, so the previous server release can still start (`deploy/README.md`).
- **Tests are never skipped or disabled to get CI green.**
