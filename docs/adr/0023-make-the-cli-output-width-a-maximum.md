# 23. Make the CLI output width a maximum

Date: 2026-11-24

## Status

Accepted

Amends [4. Wrap text at terminal width, never wrap code blocks](0004-terminal-width-wrapping.md)

## Context

`patine --width <n>` laid the output out for exactly `n` columns. On a
wide terminal, running text wrapped at the full terminal width is hard
to read, and an exact `--width` can make the output wider than the
terminal.

ADR 4 says that output piped to a file falls back to 80 columns and that
a `--width` flag would address this. The command asks crossterm for the
size, and crossterm asks the controlling terminal (`/dev/tty` on Unix,
`CONOUT$` on Windows) before it looks at stdout. A run started from a
terminal therefore finds that terminal's width even when its output goes
to a file or a pipe.

## Decision

`--width` is a maximum.

- When stdout is a terminal, the output is laid out for the smaller of
  the terminal width and `--width`.
- When stdout is not a terminal, the output is laid out for `--width`
  as given.
- Without `--width` the output is laid out for the terminal width, as
  before. No maximum applies by default.
- When no terminal size can be read, or a size of 0 columns is
  reported, `--width` is used as given, and without `--width` the width
  is 80 columns.

The decision looks at whether stdout is a terminal and not at whether a
size can be read, because a size can be read in a run whose output goes
to a file.

The command computes the width in a function of the requested width,
whether stdout is a terminal and the terminal width. The function reads
the terminal width only when its result depends on it.

## Consequences

- With stdout a terminal, `--width` no longer makes the output wider
  than the terminal.
- `tests/cli.rs` runs the binary with stdout piped. The tests whose
  result depends on a width pass `--width`, so the terminal `cargo test`
  runs in does not change them.
