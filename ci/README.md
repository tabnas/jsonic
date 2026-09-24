# ci/

Staging area for GitHub Actions workflow changes.

This directory exists because session credentials cannot write
`.github/workflows/*` — see admin `DECISIONS.md` ADR-8. To change CI:

1. Put the intended workflow file in `workflows/`.
2. A maintainer promotes it with the admin `rollout/apply-ci-folders.sh`
   script.

## Promoted

Nothing is pending. The Rust gate staged here has been promoted and now
lives in `.github/workflows/rust.yml`:

- **`rust.yml`**, the Rust gate: `ci/rust/run.sh` (formatting, build, the
  shared fixtures, doctests, clippy, and a lockfile check that exempts
  only the sibling crates' versions) on the MSRV pinned in
  `rs/Cargo.toml`. It clones `tabnas/parser`, `tabnas/json` and
  `tabnas/support` beside the checkout, because the crate takes all three
  as path dependencies and none is published. `make test-rs` is the fast
  local loop; the script is what CI runs.
