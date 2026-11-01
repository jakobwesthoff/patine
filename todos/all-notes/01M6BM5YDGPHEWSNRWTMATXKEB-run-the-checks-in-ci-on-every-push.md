---
title: "Run the checks in CI on every push"
kind: chore
component: project
origin: implementation
---
# Run the checks in CI on every push

patine has only `.github/workflows/release.yml`, which builds the release
binaries when a version tag is pushed. Nothing runs the tests, clippy or
the docs build on a push to `main` or on a pull request, so a commit that
breaks on Linux or Windows, or under an older lockfile, shows up only in
the release build or downstream in asqr and ntropy. Found on 2026-11-01
while updating the dependencies for 2.2.1.

## Decision

The user decided on 2026-11-01 (AskUserQuestion in the lead session of
the todo run): do it now, in parallel with the ntropy docs work, then
release patine 2.2.3.

## Suggested fix

A `ci.yml` on push and pull request, running on Linux and macOS (and
Windows, which release.yml builds for):

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test`
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps`
- the MSRV from `Cargo.toml` (`rust-version`, 1.89): `cargo +1.89 test --locked`

Optionally a job that pins unicode-width to 0.2.2
(`cargo update -p unicode-width --precise 0.2.2`) and runs the width
tests. ADR 0019 promises the same width for the halfwidth sound marks
under 0.2.2 and 0.2.3, and today only the unit tests of
`uncounted_mark_columns` in `src/styled.rs` guard the 0.2.2 branch.
asqr's `.github/workflows/ci.yml` is a model for the layout.
